use crate::data::tree::{Ascendancy, Class, TreeData};
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
    pub nodes: Vec<u16>,
    pub nodes_ex: Vec<u16>,
    pub masteries: FxHashMap<u16, u16>,
}

impl Default for PassiveTree {
    fn default() -> Self {
        let mut pt = Self {
            class: Default::default(),
            ascendancy: None,
            nodes: Default::default(),
            nodes_ex: Default::default(),
            masteries: Default::default(),
        };
        pt.nodes.push(get_class_node(pt.class));
        pt
    }
}

fn get_class_node(class: Class) -> u16 {
    TREE.nodes
        .values()
        .find(|n| n.class_start_index.is_some() && n.class_start_index.unwrap() == class as i32)
        .unwrap()
        .skill
}

fn get_ascendancy_node(ascendancy: Ascendancy) -> u16 {
    TREE.nodes
        .values()
        .find(|n| n.is_ascendancy_start && n.ascendancy == Some(ascendancy))
        .unwrap()
        .skill
}

lazy_static! {
    static ref PATH_OF_THE: Vec<u16> = TREE
        .nodes
        .values()
        .filter(|n| n.name.starts_with("Path of the") && n.ascendancy.is_some())
        .map(|n| n.skill)
        .collect();
}

struct FindDisconnectedNodes {
    pub nodes_search_remove: Vec<u16>,
    class: Class,
}

impl FindDisconnectedNodes {
    fn new(nodes_search_remove: Vec<u16>, class: Class) -> Self {
        Self {
            nodes_search_remove,
            class,
        }
    }

    fn successors_allocated(&self, node: u16) -> Vec<u16> {
        let mut v: Vec<u16> = TREE.nodes[&node]
            .out
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| self.nodes_search_remove.contains(id))
            .copied()
            .collect();
        if !TREE.nodes[&node].is_mastery {
            let nodes_in: Vec<u16> = TREE.nodes[&node]
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
    pub fn find_nodes_remove(&self) -> Vec<u16> {
        let groups = strongly_connected_components::strongly_connected_components_from(&get_class_node(self.class), |p| self.successors_allocated(*p));
        let mut col = vec![];
        for group in groups {
            col.extend(group);
        }
        let ret: Vec<u16> = self.nodes_search_remove.iter().filter(|id| !col.contains(id)).copied().collect();
        ret
    }
}

impl PassiveTree {
    pub fn data() -> &'static TreeData {
        &TREE
    }

    fn successors(&self, node: u16) -> Vec<u16> {
        if TREE.nodes[&node].class_start_index.is_some() {
            return vec![node];
        }
        let mut v: Vec<u16> = TREE.nodes[&node].r#in
            .as_ref()
            .unwrap()
            .iter()
            .filter(|id| (!PATH_OF_THE.contains(*id) && (!TREE.nodes[id].is_ascendancy_start || TREE.nodes[id].ascendancy == self.ascendancy)) || self.nodes.contains(id)).copied()
            .collect();

        if PATH_OF_THE.contains(&node) {
            let nodes_out: Vec<u16> = TREE.nodes[&node]
                .out
                .as_ref()
                .unwrap()
                .iter()
                .filter(|id| TREE.nodes[id].ascendancy.is_some() && !self.nodes.contains(id)).copied()
                .collect();
            v.extend(nodes_out);
        } else {
            let nodes_out: Vec<u16> = TREE.nodes[&node]
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
    pub fn find_path(&self, node: u16) -> Option<Vec<u16>> {
        bfs(&node, |p| self.successors(*p), |p| self.nodes.contains(p))
    }

    /// Find a group of nodes to remove when a single node gets deallocated
    pub fn find_path_remove(&self, node: u16) -> Vec<u16> {
        let mut nodes = self.nodes.clone();
        nodes.retain(|&x| x != node);
        let fdn: FindDisconnectedNodes = FindDisconnectedNodes::new(nodes, self.class);
        let mut to_remove = fdn.find_nodes_remove();
        to_remove.push(node);
        to_remove
    }

    /// Flip a node status (allocated <-> non-allocated)
    pub fn flip_node(&mut self, node: u16) {
        if self.nodes.contains(&node) {
            let to_remove = self.find_path_remove(node);
            for node_remove in &to_remove {
                if TREE.nodes[&node_remove].is_mastery {
                    self.masteries.remove(&node_remove);
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
            self.set_class(ascendancy.class());
            self.nodes.push(get_ascendancy_node(ascendancy));
        }

        self.ascendancy = ascendancy;
    }

    pub fn calc_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        for node_id in &self.nodes {
            for mod_lines in &Self::data().nodes[node_id].stats {
                for mod_str in mod_lines.split('\n') {
                    if let Some(modifiers) = parse_mod(mod_str, Source::Node(*node_id)) {
                        mods.extend(modifiers);
                    }
                }
            }
        }

        for (node_id, effect_id) in &self.masteries {
            if let Some(effect) = Self::data().nodes[node_id]
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
