use crate::data::tree::{Ascendancy, Class, NodeType, TreeData, Node};
use crate::data::TREE;
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
    pub nodes_data: im::HashMap<u32, Node>,
    pub masteries: FxHashMap<u32, u32>,
}

fn init_data() -> im::HashMap<u32, Node> {
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
        };
        pt.nodes.push(get_class_node(pt.class));
        pt
    }
}

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

struct FindDisconnectedNodes {
    pub nodes_search_remove: Vec<u32>,
    class: Class,
    bloodline: Option<Ascendancy>,
}

impl FindDisconnectedNodes {
    fn new(nodes_search_remove: Vec<u32>, class: Class, bloodline: Option<Ascendancy>) -> Self {
        Self {
            nodes_search_remove,
            class,
            bloodline,
        }
    }

    fn successors_allocated(&self, node: u32) -> Vec<u32> {
        let mut v: Vec<u32> = TREE.nodes[&node]
            .out
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| self.nodes_search_remove.contains(id))
            .copied()
            .collect();
        if !TREE.nodes[&node].is_mastery {
            let nodes_in: Vec<u32> = TREE.nodes[&node]
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
    pub fn data() -> &'static TreeData {
        &TREE
    }

    fn successors(&self, node: u32) -> Vec<u32> {
        if TREE.nodes[&node].class_start_index.is_some() {
            return vec![node];
        }
        let mut v: Vec<u32> = TREE.nodes[&node].r#in
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| (!PATH_OF_THE.contains(*id) && (!TREE.nodes[id].is_ascendancy_start || TREE.nodes[id].ascendancy == self.ascendancy)) || self.nodes.contains(id)).copied()
            .collect();

        if PATH_OF_THE.contains(&node) {
            let nodes_out: Vec<u32> = TREE.nodes[&node]
                .out
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| TREE.nodes[id].ascendancy.is_some() && !self.nodes.contains(id)).copied()
                .collect();
            v.extend(nodes_out);
        } else {
            let nodes_out: Vec<u32> = TREE.nodes[&node]
                .out
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| !TREE.nodes[id].is_mastery).copied()
                .collect();
            v.extend(nodes_out);
        }
        v
    }

    pub fn passives_count(&self) -> usize {
        self.nodes.iter().filter(|n| TREE.nodes[n].ascendancy.is_none() && TREE.nodes[n].class_start_index.is_none()).count()
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
        let fdn: FindDisconnectedNodes = FindDisconnectedNodes::new(nodes, self.class, self.bloodline);
        let mut to_remove = fdn.find_nodes_remove();
        to_remove.push(node);
        to_remove
    }

    /// Flip a node status (allocated <-> non-allocated)
    pub fn flip_node(&mut self, node: u32) {
        if self.nodes.contains(&node) {
            let to_remove = self.find_path_remove(node);
            for node_remove in &to_remove {
                if TREE.nodes[node_remove].is_mastery {
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
        self.nodes.iter().filter(|n| TREE.nodes[n].node_type() == NodeType::JewelSocket).copied().collect()
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
}
