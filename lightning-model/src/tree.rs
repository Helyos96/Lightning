use crate::data::tree::{Ascendancy, Class, ClusterOrbitData, Node, NodeType, TreeData};
use crate::data::TREE;
use crate::item::ClusterData;
use crate::modifier::{parse_mod, Mod, Source};
use lazy_static::lazy_static;
use pathfinding::directed::strongly_connected_components;
use pathfinding::prelude::bfs;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::convert::AsRef;

/// Player tree used in Build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveTree {
    pub class: Class,
    pub ascendancy: Option<Ascendancy>,
    pub bloodline: Option<Ascendancy>,
    pub nodes: Vec<u32>,
    // Additional nodes come mostly from "Allocates <xxx>" mods
    #[serde(skip)]
    pub nodes_additional: Vec<u32>,
    #[serde(skip, default = "init_data")]
    pub nodes_data: imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>,
    #[serde(default)]
    pub nodes_cluster: Vec<(u32, Node)>,
    pub masteries: FxHashMap<u32, u32>,
}

fn init_data() -> imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK> {
    TREE.nodes.clone()
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
            nodes_data: TREE.nodes.clone(),
            nodes_cluster: Default::default(),
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
                .filter(|id| !self.nodes_data[id].is_mastery).copied()
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
            self.nodes.push(get_bloodline_node(bloodline));
        }
        if let Some(old_bloodline) = old_bloodline {
            self.flip_node(get_bloodline_node(old_bloodline));
        }
    }

    pub fn calc_mods(&self) -> Vec<Mod> {
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

        mods
    }

    fn _remove_jewel(&mut self, node_id: u32) {
        let node_ids: Vec<u32> = self.nodes_cluster.iter().filter_map(|(jewel_node_id, node)| {
            if *jewel_node_id == node_id {
                Some(node.skill)
            } else {
                None
            }
        }).collect();

        if node_ids.is_empty() {
            return;
        }

        self.nodes_cluster.retain(|(jewel_node_id, _)| *jewel_node_id != node_id);

        for id in node_ids {
            self._remove_jewel(id);
            self.nodes_data.remove(&id);
        }

        // Insert back jewel node if it's a cluster jewel
        if node_id <= u16::MAX as u32 &&
           let Some(node) = TREE.nodes.get(&node_id).to_owned() &&
           node.expansion_jewel.is_some() {
            self.nodes_data.insert(node_id, TREE.nodes[&node_id].clone());
        }
    }

    pub fn remove_jewel(&mut self, node_id: u32) {
        self._remove_jewel(node_id);
        self.clean_unk_allocated_nodes();
    }

    // Typically after removing a cluster jewel
    pub fn clean_unk_allocated_nodes(&mut self) {
        self.nodes.retain(|node_id| self.nodes_data.contains_key(node_id));
    }

    pub fn add_cluster(&mut self, mut cluster_data: ClusterData, orbit_data: &ClusterOrbitData, jewel_node_id: u32, base_item: &str) {
        if let Some(group_id) = self.get_proxy_group(jewel_node_id) {
            let mut id_counter = u16::MAX as u32 + 1 + self.nodes_cluster.len() as u32;
            let mut new_nodes = vec![];

            let small_node = if cluster_data.small_passives_node_id == NOTHINGNESS_NODE_ID {
                // Voices
                cluster_data.small_passives_amount += 3;
                Some(&Node {
                    name: "Nothingness".to_string(),
                    icon: "Art/2DArt/SkillIcons/passives/flaskdex.png".to_string(),
                    ..Default::default()
                })
            } else {
                TREE.nodes.get(&cluster_data.small_passives_node_id)
            };

            if let Some(small_node) = small_node {
                for i in 0..(cluster_data.small_passives_amount - cluster_data.added_sockets_amount - cluster_data.notables.len() as u32) {
                    let mut node = small_node.clone();
                    node.skill = id_counter;
                    node.group = Some(group_id);
                    node.orbit = Some(orbit_data.orbit);
                    node.orbit_index = Some(orbit_data.passives[i as usize]);
                    if i == 0 {
                        node.r#in = Some(vec![jewel_node_id]);
                    } else {
                        node.r#in = Some(vec![]);
                    }
                    node.out = Some(vec![]);
                    node.stats.extend_from_slice(&cluster_data.added_stats);
                    new_nodes.push(node);
                    id_counter += 1;
                }
            }

            let indices = if base_item == "Large Cluster Jewel" {
                &[0, 2, 1]
            } else {
                &[0, 1, 2]
            };
            for i in 0..cluster_data.added_sockets_amount {
                let node = self.nodes_data.values().find(|n| n.group == Some(group_id) && n.is_jewel_socket && n.expansion_jewel.as_ref().unwrap().index == indices[i as usize]).cloned();
                if let Some(mut node) = node {
                    node.skill = id_counter;
                    node.group = Some(group_id);
                    node.r#in = Some(vec![]);
                    node.out = Some(vec![]);
                    new_nodes.push(node);
                    id_counter += 1;
                }
            }

            for (i, notable) in cluster_data.notables.into_iter().enumerate() {
                let mut node = notable.clone();
                node.skill = id_counter;
                node.group = Some(group_id);
                node.orbit = Some(orbit_data.orbit);
                node.orbit_index = Some(orbit_data.notable[i as usize]);
                node.r#in = Some(vec![]);
                node.out = Some(vec![]);
                new_nodes.push(node);
                id_counter += 1;
            }

            let mut jewel_node = self.nodes_data[&jewel_node_id].clone();
            jewel_node.out.as_mut().unwrap().push(new_nodes[0].skill);
            self.nodes_data.insert(jewel_node.skill, jewel_node.clone());

            // Sort new_nodes by orbit_index so consecutive nodes in orbit order are adjacent.
            new_nodes.sort_by_key(|n| n.orbit_index.unwrap_or(0));
            // Connect nodes with closest orbit index
            for i in 0..new_nodes.len().saturating_sub(1) {
                let (left, right) = new_nodes.split_at_mut(i + 1);
                let a = &mut left[i];
                let b_skill = right[0].skill;
                a.out.get_or_insert_with(Vec::new).push(b_skill);

                let (left2, right2) = new_nodes.split_at_mut(i + 1);
                let a_skill = left2[i].skill;
                let b = &mut right2[0];
                b.r#in.get_or_insert_with(Vec::new).push(a_skill);
            }
            // Connect first and last node
            if new_nodes.len() >= 2 {
                let first_skill = new_nodes[0].skill;
                let last_skill = new_nodes[new_nodes.len() - 1].skill;
                new_nodes.last_mut().unwrap().out.get_or_insert_with(Vec::new).push(first_skill);
                new_nodes[0].r#in.get_or_insert_with(Vec::new).push(last_skill);
            }

            for node in &new_nodes {
                self.nodes_cluster.push((jewel_node_id, node.to_owned()));
            }
            self.nodes_cluster.push((jewel_node_id, jewel_node));

            for node in new_nodes {
                self.nodes_data.insert(node.skill, node);
            }
        }
    }

    pub fn get_proxy_group(&self, cluster_jewel_node_id: u32) -> Option<u16> {
        let proxy_node = self.nodes_data.get(&cluster_jewel_node_id)?.expansion_jewel.as_ref()?.proxy;
        self.nodes_data.get(&proxy_node)?.group
    }
}
