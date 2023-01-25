use crate::data::TREE;
use crate::modifier::{parse_mod, Mod, Source};
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum Class {
    Scion,
    Marauder,
    Duelist,
    Ranger,
    Shadow,
    Witch,
    Templar,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16
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

#[derive(Debug, Serialize,Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
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
    pub ascendancy_name: Option<String>,
    pub class_start_index: Option<i32>,
    #[serde(default)]
    pub mastery_effects: Vec<MasteryEffect>,
    pub group: Option<u16>,
    pub orbit: Option<u16>,
    pub orbit_index: Option<u16>,
    pub out: Option<Vec<u16>>,
}

impl Node {
    pub fn node_type(&self) -> NodeType {
        if self.ascendancy_name.is_some() {
            if self.is_notable {
                return NodeType::AscendancyNotable
            } else {
                return NodeType::AscendancyNormal
            }
        }
        if self.is_notable {
            NodeType::Notable
        } else if self.is_keystone {
            NodeType::Keystone
        } else if self.is_mastery {
            NodeType::Mastery
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
pub struct Constants {
    #[serde(rename = "skillsPerOrbit")]
    pub skills_per_orbit: Vec<u16>,
    #[serde(rename = "orbitRadii")]
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
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PassiveTree {
    pub nodes: Vec<u16>,
    pub nodes_ex: Vec<u16>,
    pub masteries: Vec<(u16, u16)>,
}

impl PassiveTree {
    pub fn data() -> &'static TreeData {
        &TREE
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
