use crate::data::tree::{Ascendancy, Class, ClusterOrbitData, Node, NodeType, TreeData};
use crate::data::{TATTOOS, TREE};
use crate::item::ClusterData;
use crate::modifier::{parse_mod, Mod, Source};
use arc_swap::ArcSwap;
use lazy_static::lazy_static;
use pathfinding::directed::strongly_connected_components;
use pathfinding::prelude::bfs;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use derivative::Derivative;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;
use std::convert::AsRef;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Player tree used in Build
#[derive(Derivative, Debug, Serialize, Deserialize)]
#[derivative(Clone)]
pub struct PassiveTree {
    pub class: Class,
    pub ascendancy: Option<Ascendancy>,
    pub bloodline: Option<Ascendancy>,
    pub nodes: Vec<u32>,
    // Additional nodes come mostly from "Allocates <xxx>" mods
    #[serde(default)]
    pub nodes_additional: Vec<u32>,
    #[serde(skip, default = "init_data")]
    pub nodes_data: imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>,
    #[serde(default)]
    pub nodes_cluster: Vec<(u32, Node)>,
    // <node_id, effect_id>
    pub masteries: FxHashMap<u32, u32>,
    #[serde(default)]
    pub tattoos: FxHashMap<u32, String>,
    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_arc_swap"))]
    mod_cache: ArcSwap<Vec<Mod>>,
    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_atomic_bool"))]
    is_modcache_fresh: AtomicBool,
}

fn clone_arc_swap<T>(cache: &ArcSwap<T>) -> ArcSwap<T> {
    ArcSwap::new(cache.load_full())
}

fn clone_atomic_bool(bool_ref: &AtomicBool) -> AtomicBool {
    AtomicBool::new(bool_ref.load(Ordering::Relaxed))
}

fn init_data() -> imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK> {
    let mut data = TREE.nodes.clone();

    // Add proxy tag to jewel socket nodes so that they're hidden by default
    let node_ids: Vec<u32> = data.values().filter(|n| n.name == "Small Jewel Socket" || n.name == "Medium Jewel Socket").map(|n| n.skill).collect();
    for id in node_ids {
        let mut jewel_node = data.get(&id).unwrap().clone();
        jewel_node.is_proxy = true;
        data.insert(id, jewel_node);
    }

    data
}

fn default_cell_true() -> Cell<bool> {
    Cell::new(true)
}

impl Default for PassiveTree {
    fn default() -> Self {
        let mut pt = Self {
            class: Default::default(),
            ascendancy: None,
            bloodline: None,
            nodes: Default::default(),
            nodes_additional: Default::default(),
            masteries: Default::default(),
            nodes_data: init_data(),
            nodes_cluster: Default::default(),
            mod_cache: Default::default(),
            is_modcache_fresh: Default::default(),
            tattoos: Default::default(),
        };
        pt.nodes.push(get_class_node(pt.class));
        pt
    }
}

pub const NOTHINGNESS_NODE_ID: u32 = u32::MAX - 1;

fn get_class_node(class: Class) -> u32 {
    TREE.nodes
        .values()
        .find(|n| n.class_start_index == Some(class as i32))
        .unwrap()
        .skill
}

fn get_ascendancy_node(ascendancy: Ascendancy) -> u32 {
    TREE.nodes
        .values()
        .find(|n| n.is_ascendancy_start && n.ascendancy == Some(ascendancy))
        .unwrap()
        .skill
}

fn get_bloodline_node(bloodline: Ascendancy) -> u32 {
    TREE.nodes
        .values()
        .find(|n| n.is_ascendancy_start && n.is_bloodline && n.ascendancy == Some(bloodline))
        .unwrap()
        .skill
}

// TODO: should be extracted from game data in .json
// For now this is just copied from PoB
fn get_orbit_offsets(proxy_node_id: u32) -> [usize; 3] {
    match proxy_node_id {
        43989 => [3, 5, 5],
        25134 => [0, 11, 11],
        30275 => [2, 3, 3],
        28650 => [1, 1, 1],
        48132 => [5, 9, 9],
        18756 => [4, 7, 7],
        55706 => [2, 3, 0],
        26661 => [3, 5, 0],
        13201 => [3, 7, 0],
        40114 => [1, 0, 0],
        18361 => [2, 0, 0],
        7956  => [3, 0, 0],
        51233 => [5, 9, 0],
        57194 => [5, 11, 0],
        35853 => [0, 1, 0],
        35313 => [4, 0, 0],
        44470 => [5, 0, 0],
        37147 => [0, 0, 0],
        25441 => [1, 1, 0],
        28018 => [2, 3, 0],
        53203 => [3, 5, 0],
        3854  => [0, 0, 0],
        49951 => [1, 0, 0],
        22046 => [2, 0, 0],
        37898 => [5, 11, 0],
        64166 => [1, 1, 0],
        58355 => [2, 3, 0],
        48128 => [5, 0, 0],
        27475 => [0, 0, 0],
        35070 => [1, 0, 0],
        35926 => [4, 7, 0],
        33833 => [5, 9, 0],
        50179 => [5, 11, 0],
        36414 => [3, 0, 0],
        10643 => [4, 0, 0],
        56439 => [5, 0, 0],
        58194 => [3, 5, 0],
        34013 => [4, 7, 0],
        24452 => [5, 9, 0],
        63754 => [2, 0, 0],
        54600 => [3, 0, 0],
        27819 => [4, 0, 0],
        _ => [0, 0, 0],
    }
}

fn translate_cluster_orbit_index(src_oidx: usize, src_nodes_per_orbit: usize, dest_nodes_per_orbit: usize) -> usize {
    if src_nodes_per_orbit == dest_nodes_per_orbit {
        return src_oidx;
    }
    match (src_nodes_per_orbit, dest_nodes_per_orbit) {
        (12, 16) => [0, 1, 3, 4, 5, 7, 8, 9, 11, 12, 13, 15][src_oidx],
        (16, 12) => [0, 1, 1, 2, 3, 4, 4, 5, 6, 7, 7, 8, 9, 10, 10, 11][src_oidx],
        (6, 16)  => [0, 3, 5, 8, 11, 13][src_oidx],
        (16, 6)  => [0, 0, 0, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 5, 5, 5][src_oidx],
        _ => (src_oidx * dest_nodes_per_orbit) / src_nodes_per_orbit
    }
}

fn apply_cluster_orbit_index_adjustment(
    base_idx: usize,
    proxy_node_id: u32,
    size_index: usize,
    cluster_total_indices: usize,
    tree_skills_per_orbit: usize
) -> usize {
    let start_oidx = get_orbit_offsets(proxy_node_id)[size_index];
    let corrected_idx = (base_idx + start_oidx) % cluster_total_indices;

    translate_cluster_orbit_index(corrected_idx, cluster_total_indices, tree_skills_per_orbit)
}

lazy_static! {
    static ref PATH_OF_THE: Vec<u32> = TREE
        .nodes
        .values()
        .filter(|n| n.name.starts_with("Path of the") && n.ascendancy.is_some())
        .map(|n| n.skill)
        .collect();
}

struct FindDisconnectedNodes<'a> {
    pub nodes_search_remove: Vec<u32>,
    class: Class,
    bloodline: Option<Ascendancy>,
    nodes_data: &'a imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>,
}

impl<'a> FindDisconnectedNodes<'a> {
    fn new(
        nodes_search_remove: Vec<u32>,
        class: Class,
        bloodline: Option<Ascendancy>,
        nodes_data: &'a imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>,
    ) -> Self {
        Self {
            nodes_search_remove,
            class,
            bloodline,
            nodes_data,
        }
    }

    fn successors_allocated(&self, node: u32) -> Vec<u32> {
        let mut v: Vec<u32> = self.nodes_data[&node]
            .out
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| self.nodes_search_remove.contains(id))
            .copied()
            .collect();
        if !self.nodes_data[&node].is_mastery {
            let nodes_in: Vec<u32> = self.nodes_data[&node]
                .r#in
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| self.nodes_search_remove.contains(id))
                .copied()
                .collect();
            v.extend(nodes_in);
        }
        v
    }

    /// Find a group of nodes to remove when a single node gets deallocated
    pub fn find_nodes_remove(&self) -> Vec<u32> {
        let mut col = vec![];

        let mut start_nodes = vec![get_class_node(self.class)];
        if let Some(bloodline) = self.bloodline {
            start_nodes.push(get_bloodline_node(bloodline));
        }

        for start_node in start_nodes {
            let groups = strongly_connected_components::strongly_connected_components_from(&start_node, |p| self.successors_allocated(*p));
            for group in groups {
                col.extend(group);
            }
        }

        let ret: Vec<u32> = self.nodes_search_remove.iter().filter(|id| !col.contains(id)).copied().collect();
        ret
    }
}

impl PassiveTree {
    fn successors(&self, node_id: u32) -> Vec<u32> {
        let node = self.nodes_data.get(&node_id).unwrap();
        if node.class_start_index.is_some() {
            return vec![node_id];
        }
        let mut v: Vec<u32> = node.r#in
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| (!PATH_OF_THE.contains(*id) && (!self.nodes_data[id].is_ascendancy_start || self.nodes_data[id].ascendancy == self.ascendancy)) || self.nodes.contains(id)).copied()
            .collect();

        if PATH_OF_THE.contains(&node_id) {
            let nodes_out: Vec<u32> = node
                .out
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| self.nodes_data[id].ascendancy.is_some() && !self.nodes.contains(id)).copied()
                .collect();
            v.extend(nodes_out);
        } else {
            let nodes_out: Vec<u32> = node
                .out
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| self.nodes_data.contains_key(id) && !self.nodes_data[id].is_mastery).copied()
                .collect();
            v.extend(nodes_out);
        }
        v
    }

    /// To be called after deserializing
    pub fn init(&mut self) {
        for (_, node) in &self.nodes_cluster {
            self.nodes_data.insert(node.skill, node.clone());
        }
        let tattoos = self.tattoos.clone();
        for (node_id, tattoo_str) in tattoos {
            self.set_tattoo(node_id, Some(&tattoo_str));
        }
    }

    pub fn passives_count(&self) -> usize {
        self.nodes.iter().filter(|n| self.nodes_data[n].ascendancy.is_none() && self.nodes_data[n].class_start_index.is_none()).count()
    }

    /// Find the shortest path to link a node to
    /// the rest of the tree. Using Breadth-First-Search.
    pub fn find_path(&self, node: u32) -> Option<Vec<u32>> {
        bfs(&node, |p| self.successors(*p), |p| self.nodes.contains(p))
    }

    /// Find a group of nodes to remove when a single node gets deallocated
    pub fn find_path_remove(&self, node: u32) -> Vec<u32> {
        let mut nodes = self.nodes.clone();
        nodes.retain(|&x| x != node);
        let fdn = FindDisconnectedNodes::new(nodes, self.class, self.bloodline, &self.nodes_data);
        let mut to_remove = fdn.find_nodes_remove();
        to_remove.push(node);
        to_remove
    }

    /// Flip a node status (allocated <-> non-allocated)
    pub fn flip_node(&mut self, node: u32) {
        if self.nodes.contains(&node) {
            let to_remove = self.find_path_remove(node);
            for node_remove in &to_remove {
                if self.nodes_data[node_remove].is_mastery {
                    self.masteries.remove(node_remove);
                }
            }
            self.nodes = self
                .nodes
                .iter().copied()
                .filter(|id| !to_remove.contains(id))
                .collect();
        } else if let Some(path) = self.find_path(node) {
            self.nodes.extend_from_slice(&path[0..path.len() - 1]);
        }

        self.is_modcache_fresh.store(false, Ordering::Relaxed);
    }

    pub fn invalidate_modcache(&self) {
        self.is_modcache_fresh.store(false, Ordering::Relaxed);
    }

    pub fn jewel_slots(&self) -> Vec<u32> {
        self.nodes.iter().filter(|n| self.nodes_data[n].node_type() == NodeType::JewelSocket).copied().collect()
    }

    pub fn set_class(&mut self, class: Class) {
        if class == self.class {
            return;
        }

        let old_class = self.class;
        self.class = class;
        self.nodes.push(get_class_node(class));
        self.flip_node(get_class_node(old_class));
        self.set_ascendancy(None);
    }

    pub fn set_ascendancy(&mut self, ascendancy: Option<Ascendancy>) {
        if ascendancy == self.ascendancy {
            return;
        }

        if let Some(old_ascendancy) = self.ascendancy {
            self.flip_node(get_ascendancy_node(old_ascendancy));
        }
        if let Some(ascendancy) = ascendancy {
            if let Some(class) = ascendancy.class() {
                self.set_class(class);
            }
            self.nodes.push(get_ascendancy_node(ascendancy));
        }

        self.ascendancy = ascendancy;
    }

    pub fn set_bloodline(&mut self, bloodline: Option<Ascendancy>) {
        if bloodline == self.bloodline {
            return;
        }

        let old_bloodline = self.bloodline;
        self.bloodline = bloodline;

        if let Some(bloodline) = bloodline {
            self.flip_node(get_bloodline_node(bloodline));
        }
        if let Some(old_bloodline) = old_bloodline {
            self.flip_node(get_bloodline_node(old_bloodline));
        }
    }

    pub fn regen_modcache(&self) {
        let mut mods = Vec::with_capacity(300);

        let extra_nodes = self.nodes_additional.iter().filter(|n| !self.nodes.contains(n));
        for node_id in self.nodes.iter().chain(extra_nodes) {
            for mod_lines in &self.nodes_data[node_id].stats {
                for mod_str in mod_lines.split('\n') {
                    if let Some(modifiers) = parse_mod(mod_str, Source::Node(*node_id)) {
                        mods.extend(modifiers);
                    }
                }
            }
        }

        for (node_id, effect_id) in &self.masteries {
            if let Some(effect) = self.nodes_data[node_id]
                .mastery_effects
                .iter()
                .find(|m| m.effect == *effect_id)
            {
                for mod_str in &effect.stats {
                    if let Some(modifiers) = parse_mod(mod_str, Source::Mastery((*node_id, *effect_id))) {
                        mods.extend(modifiers);
                    }
                }
            }
        }

        self.mod_cache.store(Arc::new(mods));
        self.is_modcache_fresh.store(true, Ordering::Relaxed);
    }

    pub fn calc_mods(&self) -> Arc<Vec<Mod>> {
        if !self.is_modcache_fresh.load(Ordering::Relaxed) {
            self.regen_modcache();
        }

        arc_swap::Guard::into_inner(self.mod_cache.load())
    }

    fn _remove_jewel(&mut self, node_id: u32, removed_sockets: &mut Vec<u32>) {
        let node_ids: Vec<u32> = self.nodes_cluster.iter().filter_map(|(jewel_node_id, node)| {
            if *jewel_node_id == node_id && node.skill != node_id {
                Some(node.skill)
            } else {
                None
            }
        }).collect();

        if node_ids.is_empty() {
            return;
        }

        for id in node_ids.iter().copied() {
            if self.nodes_data[&id].is_jewel_socket {
                self._remove_jewel(id, removed_sockets);
                let mut new_node = TREE.nodes.get(&id).cloned().unwrap();
                new_node.is_proxy = true;
                removed_sockets.push(id);
                self.nodes_data.insert(id, new_node);
            }
            if id > u16::MAX as u32 {
                self.nodes_data.remove(&id);
            }
        }

        let mut new_root_node = self.nodes_data[&node_id].clone();
        new_root_node.out = Some(new_root_node.out.unwrap().iter().copied().filter(|id| self.nodes_data.contains_key(id)).collect());
        self.nodes_data.insert(new_root_node.skill, new_root_node);
        self.nodes_cluster.retain(|(jewel_id, _)| *jewel_id != node_id);
        self.nodes.retain(|id| !node_ids.contains(id));
    }

    pub fn remove_jewel(&mut self, node_id: u32) -> Vec<u32> {
        let mut removed_sockets = vec![];
        self._remove_jewel(node_id, &mut removed_sockets);
        removed_sockets
    }

    pub fn add_cluster(&mut self, mut cluster_data: ClusterData, jewel_node_id: u32, base_item: &str) {
        let proxy_node_id = match TREE.nodes.get(&jewel_node_id).and_then(|n| n.expansion_jewel.as_ref()) {
            Some(ej) => ej.proxy,
            None => return,
        };

        let group_id = match TREE.nodes.get(&proxy_node_id).and_then(|n| n.group) {
            Some(id) => id,
            None => return,
        };

        if let Some(parent_node) = self.nodes_data.get_mut(&jewel_node_id) {
            if let Some(out_links) = parent_node.out.as_mut() {
                out_links.retain(|&id| id != proxy_node_id);
            }
        }

        let (size_index, total_indices, orbit, small_idx_order, notable_idx_order, socket_idx_order) = match base_item {
            "Small Cluster Jewel" => (0, 6, 1, vec![0, 4, 2], vec![4], vec![4]),
            "Medium Cluster Jewel" => (1, 12, 2, vec![0, 6, 8, 4, 10, 2], vec![6, 10, 2, 0], vec![6]),
            "Large Cluster Jewel" => (2, 12, 3, vec![0, 4, 6, 8, 10, 2, 7, 5, 9, 3, 11, 1], vec![6, 4, 8, 10, 2], vec![4, 8, 6]),
            _ => return,
        };

        let skills_per_orbit = TREE.constants.skills_per_orbit.get(orbit as usize).copied().unwrap_or(16) as usize;

        if cluster_data.small_passives_node_id == NOTHINGNESS_NODE_ID {
            cluster_data.small_passives_amount += 3;
        }

        let mut id_counter = self.nodes_cluster.iter().map(|nc| nc.1.skill).max().unwrap_or(u16::MAX as u32) + 1;
        let mut template_nodes: FxHashMap<usize, Node> = FxHashMap::default();

        // Place Sockets
        let socket_count = cluster_data.added_sockets_amount as usize;
        let socket_expansion_indices = if base_item == "Large Cluster Jewel" { &[0, 2, 1] } else { &[0, 1, 2] };

        let mut socket_assignments = vec![];
        if base_item == "Large Cluster Jewel" && socket_count == 1 {
            socket_assignments.push((6, 1));
        } else {
            for i in 0..socket_count {
                socket_assignments.push((socket_idx_order[i], socket_expansion_indices[i]));
            }
        }

        for (node_idx, expansion_idx) in socket_assignments {
            if let Some(mut node) = TREE.nodes.values().find(|n| n.group == Some(group_id) && n.is_jewel_socket && n.expansion_jewel.as_ref().map_or(false, |ej| ej.index == expansion_idx as u32)).cloned() {
                node.group = Some(group_id);
                node.r#in = Some(vec![]);
                node.out = Some(vec![]);
                node.is_proxy = false;
                node.name = match base_item {
                    "Large Cluster Jewel" => "Medium Jewel Socket".to_string(),
                    "Medium Cluster Jewel" => "Small Jewel Socket".to_string(),
                    _ => "Jewel Socket".to_string(),
                };
                template_nodes.insert(node_idx, node);
            }
        }

        // Place Notables
        let mut active_notable_indices = vec![];
        let total_nodes = cluster_data.small_passives_amount;

        for &idx in &notable_idx_order {
            if active_notable_indices.len() == cluster_data.notables.len() { break; }
            let mut adj_idx = idx;

            if base_item == "Medium Cluster Jewel" {
                if socket_count == 0 && cluster_data.notables.len() == 2 {
                    if adj_idx == 6 { adj_idx = 4; } else if adj_idx == 10 { adj_idx = 8; }
                } else if total_nodes == 4 {
                    if adj_idx == 10 { adj_idx = 9; } else if adj_idx == 2 { adj_idx = 3; }
                }
            }

            if !template_nodes.contains_key(&adj_idx) {
                active_notable_indices.push(adj_idx);
            }
        }
        active_notable_indices.sort_unstable();

        for (i, notable) in cluster_data.notables.into_iter().enumerate() {
            if i >= active_notable_indices.len() { break; }
            let node_idx = active_notable_indices[i];

            let mut node = notable.clone();
            node.skill = id_counter;
            node.group = Some(group_id);
            node.r#in = Some(vec![]);
            node.out = Some(vec![]);
            template_nodes.insert(node_idx, node);
            id_counter += 1;
        }

        // Place small passives
        let small_count = (total_nodes as usize).saturating_sub(socket_count + active_notable_indices.len());
        let mut active_small_indices = vec![];

        for &idx in &small_idx_order {
            if active_small_indices.len() == small_count { break; }
            let mut adj_idx = idx;

            if base_item == "Medium Cluster Jewel" {
                if total_nodes == 5 && adj_idx == 4 { adj_idx = 3; }
                else if total_nodes == 4 {
                    if adj_idx == 8 { adj_idx = 9; } else if adj_idx == 4 { adj_idx = 3; }
                }
            }
            if !template_nodes.contains_key(&adj_idx) {
                active_small_indices.push(adj_idx);
            }
        }

        let small_node_base = if cluster_data.small_passives_node_id == NOTHINGNESS_NODE_ID {
            Some(Node {
                name: "Nothingness".to_string(),
                icon: "Art/2DArt/SkillIcons/passives/flaskdex.png".to_string(),
                ..Default::default()
            })
        } else {
            TREE.nodes.get(&cluster_data.small_passives_node_id).cloned()
        };

        if let Some(small_node) = small_node_base {
            for &node_idx in &active_small_indices {
                let mut node = small_node.clone();
                node.skill = id_counter;
                node.group = Some(group_id);
                node.r#in = Some(vec![]);
                node.out = Some(vec![]);
                node.stats.extend_from_slice(&cluster_data.added_stats);
                template_nodes.insert(node_idx, node);
                id_counter += 1;
            }
        }

        // Connect nodes in order of their orbit_idx
        let mut active_indices: Vec<usize> = template_nodes.keys().copied().collect();
        active_indices.sort_unstable();

        for i in 0..active_indices.len().saturating_sub(1) {
            let curr_idx = active_indices[i];
            let next_idx = active_indices[i + 1];

            let curr_skill = template_nodes[&curr_idx].skill;
            let next_skill = template_nodes[&next_idx].skill;

            template_nodes.get_mut(&curr_idx).unwrap().out.get_or_insert_with(Vec::new).push(next_skill);
            template_nodes.get_mut(&next_idx).unwrap().r#in.get_or_insert_with(Vec::new).push(curr_skill);
        }

        if active_indices.len() >= 2 && base_item != "Small Cluster Jewel" {
            let first_idx = active_indices[0];
            let last_idx = *active_indices.last().unwrap();

            let first_skill = template_nodes[&first_idx].skill;
            let last_skill = template_nodes[&last_idx].skill;

            template_nodes.get_mut(&last_idx).unwrap().out.get_or_insert_with(Vec::new).push(first_skill);
            template_nodes.get_mut(&first_idx).unwrap().r#in.get_or_insert_with(Vec::new).push(last_skill);
        }

        // Rotate nodes depending on which socket they're in
        if let Some(entrance_node) = template_nodes.get_mut(&0) {
            entrance_node.r#in.get_or_insert_with(Vec::new).push(jewel_node_id);

            let mut jewel_node = self.nodes_data[&jewel_node_id].clone();
            jewel_node.out.get_or_insert_with(Vec::new).push(entrance_node.skill);

            self.nodes_data.insert(jewel_node_id, jewel_node.clone());
            self.nodes_cluster.push((jewel_node_id, jewel_node));
        }

        for (template_idx, mut node) in template_nodes {
            node.orbit = Some(orbit);
            node.orbit_index = Some(apply_cluster_orbit_index_adjustment(
                template_idx,
                proxy_node_id,
                size_index,
                total_indices,
                skills_per_orbit,
            ) as u16);

            self.nodes_data.insert(node.skill, node.clone());
            self.nodes_cluster.push((jewel_node_id, node));
        }
    }

    pub fn set_tattoo(&mut self, node_id: u32, tattoo_str: Option<&str>) {
        if let Some(tattoo_str) = tattoo_str {
           if let Some(tattoo_data) = TATTOOS.get(tattoo_str) &&
              let Some(mut original_node) = self.nodes_data.get(&node_id).cloned()
            {
                original_node.stats = tattoo_data.stats.clone();
                original_node.name = tattoo_str.to_owned();
                original_node.icon = tattoo_data.icon.to_owned();
                original_node.active_effect_image = Some(tattoo_data.active_effect_image.to_owned());
                original_node.is_tattoo = true;
                self.nodes_data.insert(node_id, original_node);
                self.tattoos.insert(node_id, tattoo_str.to_owned());
            } else {
                eprintln!("Failed to add tattoo {node_id} / {tattoo_str}");
            }
        } else {
            if let Some(original_node) = TREE.nodes.get(&node_id) {
                self.nodes_data.insert(node_id, original_node.to_owned());
                self.tattoos.remove(&node_id);
            }
        }
        self.masteries.remove(&node_id);
        self.invalidate_modcache();
    }

    pub fn get_proxy_group(&self, cluster_jewel_node_id: u32) -> Option<u16> {
        let proxy_node = TREE.nodes.get(&cluster_jewel_node_id)?.expansion_jewel.as_ref()?.proxy;
        TREE.nodes.get(&proxy_node)?.group
    }
}
