use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, IntoStaticStr};

#[derive(Default, Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, EnumString, AsRefStr)]
pub enum Class {
    #[default]
    Ranger,
    Witch,
    Warrior,
    Mercenary,
    Monk,
    Sorceress,
}

impl Class {
    pub fn ascendancies(&self) -> Vec<Ascendancy> {
        use Class::*;
        use Ascendancy::*;
        match self {
            Ranger => vec![Deadeye, Pathfinder],
            Witch => vec![BloodMage, Infernalist],
            Warrior => vec![Titan, Warbringer],
            Mercenary => vec![Witchhunter, GemlingLegionnaire],
            Monk => vec![Invoker, AcolyteOfChayula],
            Sorceress => vec![Stormweaver, Chronomancer],
        }
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, EnumString, IntoStaticStr)]
pub enum Ascendancy {
    Deadeye,
    Pathfinder,
    BloodMage,
    Infernalist,
    Titan,
    Warbringer,
    Witchhunter,
    #[serde(rename = "Gemling Legionnaire")]
    #[strum(serialize = "Gemling Legionnaire")]
    GemlingLegionnaire,
    Invoker,
    #[serde(rename = "Acolyte of Chayula")]
    #[strum(serialize = "Acolyte of Chayula")]
    AcolyteOfChayula,
    Stormweaver,
    Chronomancer,
}

impl Ascendancy {
    pub fn class(&self) -> Class {
        use Class::*;
        use Ascendancy::*;
        match self {
            Deadeye => Ranger,
            Pathfinder => Ranger,
            BloodMage => Witch,
            Infernalist => Witch,
            Titan => Warrior,
            Warbringer => Warrior,
            Witchhunter => Mercenary,
            GemlingLegionnaire => Mercenary,
            Invoker => Monk,
            AcolyteOfChayula => Monk,
            Stormweaver => Sorceress,
            Chronomancer => Sorceress,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum NodeType {
    Normal,
    Notable,
    Keystone,
    AscendancyNormal,
    AscendancyNotable,
    JewelSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub skill: u16,
    pub stats: Vec<String>,
    pub icon: String,
    pub name: String,
    #[serde(default)]
    pub is_notable: bool,
    #[serde(default)]
    pub is_keystone: bool,
    #[serde(default)]
    pub is_ascendancy_start: bool,
    #[serde(default)]
    pub is_jewel_socket: bool,
    #[serde(default)]
    pub is_just_icon: bool,
    #[serde(rename = "ascendancyName")]
    pub ascendancy: Option<Ascendancy>,
    pub class_start_index: Option<i32>,
    #[serde(default)]
    pub group: Option<u16>,
    pub orbit: Option<u16>,
    pub orbit_index: Option<u16>,
    pub out: Option<Vec<u16>>,
    pub r#in: Option<Vec<u16>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.skill == other.skill
    }
}

impl Node {
    pub fn node_type(&self) -> NodeType {
        if self.ascendancy.is_some() {
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
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub x: f32,
    pub y: f32,
    pub orbits: Vec<u8>,
    pub nodes: Vec<u16>,
    pub background: Option<Background>,
    #[serde(default)]
    pub is_proxy: bool,
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
    #[serde(rename = "jewelSlots")]
    pub jewel_slots: Vec<u16>,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}
