use crate::data::TREE;
use crate::modifier::{parse_mod, Mod, Source};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use pathfinding::prelude::bfs;
use lazy_static::lazy_static;

#[derive(Default, Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum Class {
    #[default]
    Scion,
    Marauder,
    Ranger,
    Witch,
    Duelist,
    Templar,
    Shadow,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sprite {
    pub filename: String,
    pub w: u16,
    pub h: u16,
    pub coords: FxHashMap<String, Rect>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassData {
    pub base_str: i64,
    pub base_dex: i64,
    pub base_int: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MasteryEffect {
    pub effect: u16,
    pub stats: Vec<String>,
}

#[derive(Copy, Clone)]
pub enum NodeType {
    Normal,
    Notable,
    Keystone,
    Mastery,
    AscendancyNormal,
    AscendancyNotable,
    JewelSocket,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub skill: u16,
    pub stats: Vec<String>,
    pub icon: String,
    pub name: String,
    pub inactive_icon: Option<String>,
    #[serde(default)]
    pub is_mastery: bool,
    #[serde(default)]
    pub is_notable: bool,
    #[serde(default)]
    pub is_keystone: bool,
    #[serde(default)]
    pub is_ascendancy_start: bool,
    #[serde(default)]
    pub is_jewel_socket: bool,
    pub ascendancy_name: Option<String>,
    pub class_start_index: Option<i32>,
    #[serde(default)]
    pub mastery_effects: Vec<MasteryEffect>,
    pub group: Option<u16>,
    pub orbit: Option<u16>,
    pub orbit_index: Option<u16>,
    pub out: Option<Vec<u16>>,
    pub r#in: Option<Vec<u16>>,
}

impl Node {
    pub fn node_type(&self) -> NodeType {
        if self.ascendancy_name.is_some() {
            if self.is_notable {
                return NodeType::AscendancyNotable;
            } else {
                return NodeType::AscendancyNormal;
            }
        }
        if self.is_notable {
            NodeType::Notable
        } else if self.is_keystone {
            NodeType::Keystone
        } else if self.is_mastery {
            NodeType::Mastery
        } else if self.is_jewel_socket {
            NodeType::JewelSocket
        } else {
            NodeType::Normal
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Background {
    pub image: String,
    #[serde(rename = "isHalfImage")]
    pub is_half_image: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub x: f32,
    pub y: f32,
    pub orbits: Vec<u8>,
    pub nodes: Vec<u16>,
    pub background: Option<Background>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constants {
    pub skills_per_orbit: Vec<u16>,
    pub orbit_radii: Vec<u16>,
}

/// Root struct for tree.json
#[derive(Debug, Serialize, Deserialize)]
pub struct TreeData {
    pub classes: FxHashMap<Class, ClassData>,
    pub nodes: FxHashMap<u16, Node>,
    pub sprites: FxHashMap<String, Sprite>,
    pub groups: FxHashMap<u16, Group>,
    pub constants: Constants,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

/// Player tree used in Build
#[derive(Debug, Serialize, Deserialize)]
pub struct PassiveTree {
    pub class: Class,
    pub nodes: Vec<u16>,
    pub nodes_ex: Vec<u16>,
    pub masteries: Vec<(u16, u16)>,
}

impl Default for PassiveTree {
    fn default() -> Self {
        let mut pt = Self {
            class: Default::default(),
            nodes: Default::default(),
            nodes_ex: Default::default(),
            masteries: Default::default(),
        };
        pt.set_class(pt.class);
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

lazy_static! {
    static ref PATH_OF_THE: Vec<u16> =
        TREE.nodes.values().filter(|n| n.name.starts_with("Path of the") && n.ascendancy_name.is_some()).map(|n| n.skill).collect();
}

fn successors(node: u16) -> Vec<u16> {
    if TREE.nodes[&node].class_start_index.is_some() { return vec![node]; }
    let mut v: Vec<u16> = TREE.nodes[&node].out.as_ref().unwrap().iter().filter(|id| !TREE.nodes[id].is_mastery).map(|id| *id).collect();
    let nodes_in: Vec<u16> = TREE.nodes[&node].r#in.as_ref().unwrap().iter().filter(|id| !PATH_OF_THE.contains(*id)).map(|id| *id).collect();
    v.extend(nodes_in);
    v
}

impl PassiveTree {
    pub fn data() -> &'static TreeData {
        &TREE
    }

    /// Find the shortest path to link a node to
    /// the rest of the tree. Using Breadth-First-Search.
    pub fn find_path(&self, node: u16) -> Option<Vec<u16>> {
        bfs(&node, |p| successors(*p), |p| self.nodes.contains(p))
    }

    /// Find a path of nodes to remove when a single node gets deallocated
    pub fn find_path_remove(&self, node: u16) -> Vec<u16> {
        // Todo
        vec![node]
    }

    /// Flip a node status (allocated <-> non-allocated)
    pub fn flip_node(&mut self, node: u16) {
        if self.nodes.contains(&node) {
            let to_remove = self.find_path_remove(node);
            self.nodes = self.nodes.iter().map(|id| *id).filter(|id| !to_remove.contains(id)).collect();
        } else {
            if let Some(path) = self.find_path(node) {
                self.nodes.extend_from_slice(&path[0..path.len() - 1]);
            }
        }
    }

    pub fn set_class(&mut self, class: Class) {
        if let Some(index) = self.nodes.iter().position(|id| *id == get_class_node(self.class)) {
            self.nodes.remove(index);
        }
        self.nodes.push(get_class_node(class));
        self.class = class;
    }

    pub fn calc_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        for node_id in &self.nodes {
            for mod_lines in &Self::data().nodes[node_id].stats {
                for mod_str in mod_lines.split('\n') {
                    if let Some(mut modifiers) = parse_mod(mod_str) {
                        for m in &mut modifiers {
                            m.source = Source::Node(*node_id);
                        }
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
                    if let Some(mut modifiers) = parse_mod(mod_str) {
                        for m in &mut modifiers {
                            m.source = Source::Mastery((*node_id, *effect_id));
                        }
                        mods.extend(modifiers);
                    }
                }
            }
        }

        mods
    }
}
