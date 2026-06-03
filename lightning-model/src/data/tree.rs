use std::ops::Neg;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumString, IntoStaticStr, EnumIter};
use serde_with::{serde_as, DisplayFromStr};
use lazy_static::lazy_static;
use enumflags2::bitflags;

use crate::{data::TREE, item::{JewelRadius, JewelRadiusData}};

#[derive(Default, Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, EnumString, AsRefStr, EnumIter)]
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

impl Class {
    pub fn ascendancies(&self) -> Vec<Ascendancy> {
        use Class::*;
        use Ascendancy::*;
        match self {
            Scion => vec![Ascendant, Reliquarian],
            Marauder => vec![Berserker, Chieftain, Juggernaut],
            Ranger => vec![Deadeye, Raider, Pathfinder],
            Witch => vec![Necromancer, Occultist, Elementalist],
            Duelist => vec![Slayer, Gladiator, Champion],
            Templar => vec![Inquisitor, Hierophant, Guardian],
            Shadow => vec![Assassin, Saboteur, Trickster],
        }
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, EnumString, IntoStaticStr, EnumIter)]
pub enum Ascendancy {
    Inquisitor,
    Hierophant,
    Guardian,
    Slayer,
    Gladiator,
    Champion,
    Assassin,
    Saboteur,
    Trickster,
    Juggernaut,
    Berserker,
    Chieftain,
    Necromancer,
    Occultist,
    Elementalist,
    Deadeye,
    Raider,
    Pathfinder,
    Ascendant,
    Reliquarian,
    Aul,
    Farrul,
    Catarina,
    Oshabi,
    Olroth,
    KingInTheMists,
    Delirious,
    Lycia,
    Trialmaster,
    Necromantic,
    Breachlord,
    Warlock,
    Primalist,
    Warden,
}

impl Ascendancy {
    pub fn display_name(&self) -> &'static str {
        match self {
            Ascendancy::Aul => "Aul Bloodline",
            Ascendancy::Farrul => "Farrul Bloodline",
            Ascendancy::Catarina => "Catarina Bloodline",
            Ascendancy::Oshabi => "Oshabi Bloodline",
            Ascendancy::KingInTheMists => "Nameless Bloodline",
            Ascendancy::Olroth => "Olroth Bloodline",
            Ascendancy::Delirious => "Delirious Bloodline",
            Ascendancy::Lycia => "Lycia Bloodline",
            Ascendancy::Trialmaster => "Chaos Bloodline",
            Ascendancy::Breachlord => "Breachlord Bloodline",
            Ascendancy::Necromantic => "Necromantic Bloodline",
            Ascendancy::Warlock => "Warlock of the Mists",
            Ascendancy::Primalist => "Wildwood Primalist",
            Ascendancy::Warden => "Warden of the Maji",
            _ => (*self).into(), // Fallback
        }
    }

    pub fn class(&self) -> Option<Class> {
        use Class::*;
        use Ascendancy::*;
        match self {
            Inquisitor => Some(Templar),
            Hierophant => Some(Templar),
            Guardian => Some(Templar),
            Slayer => Some(Duelist),
            Gladiator => Some(Duelist),
            Champion => Some(Duelist),
            Assassin => Some(Shadow),
            Saboteur => Some(Shadow),
            Trickster => Some(Shadow),
            Juggernaut => Some(Marauder),
            Berserker => Some(Marauder),
            Chieftain => Some(Marauder),
            Necromancer => Some(Witch),
            Occultist => Some(Witch),
            Elementalist => Some(Witch),
            Deadeye => Some(Ranger),
            Raider => Some(Ranger),
            Pathfinder => Some(Ranger),
            Ascendant => Some(Scion),
            Reliquarian => Some(Scion),
            Aul => None,
            Farrul => None,
            Catarina => None,
            Oshabi => None,
            Olroth => None,
            KingInTheMists => None,
            Delirious => None,
            Lycia => None,
            Trialmaster => None,
            Necromantic => None,
            Breachlord => None,
            Warlock => None,
            Primalist => None,
            Warden => None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryEffect {
    pub effect: u32,
    pub stats: Vec<String>,
}

#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeType {
    Normal,
    Notable,
    Keystone,
    Mastery,
    AscendancyNormal,
    AscendancyNotable,
    JewelSocket,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionJewel {
    pub size: u32,
    pub index: u32,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    pub proxy: u32,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    pub parent: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub skill: u32,
    pub stats: Vec<String>,
    pub icon: String,
    pub name: String,
    pub active_icon: Option<String>,
    pub inactive_icon: Option<String>,
    pub active_effect_image: Option<String>,
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
    #[serde(default)]
    pub is_proxy: bool,
    #[serde(default)]
    pub is_bloodline: bool,
    #[serde(default)]
    pub is_tattoo: bool,
    #[serde(default)]
    pub is_blighted: bool,
    #[serde(rename = "ascendancyName")]
    pub ascendancy: Option<Ascendancy>,
    pub class_start_index: Option<i32>,
    #[serde(default)]
    pub mastery_effects: Vec<MasteryEffect>,
    pub group: Option<u16>,
    pub orbit: Option<u16>,
    pub orbit_index: Option<u16>,
    pub out: Option<Vec<u32>>,
    pub r#in: Option<Vec<u32>>,
    pub expansion_jewel: Option<ExpansionJewel>
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
        } else if self.is_mastery {
            NodeType::Mastery
        } else if self.is_jewel_socket {
            NodeType::JewelSocket
        } else {
            NodeType::Normal
        }
    }

    pub fn distance_squared(&self, node: &Node) -> f32 {
        let (x1, y1) = node_pos(self);
        let (x2, y2) = node_pos(node);
        let dx = (x1 - x2).abs();
        let dy = (y1 - y2).abs();
        (dx * dx) + (dy * dy)
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AlternateAscendancy {
    pub id: String,
}

/// Root struct for tree.json
#[derive(Debug, Serialize, Deserialize)]
pub struct TreeData {
    pub classes: FxHashMap<Class, ClassData>,
    pub nodes: imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>,
    pub sprites: FxHashMap<String, Sprite>,
    pub groups: FxHashMap<u16, Group>,
    pub constants: Constants,
    #[serde(rename = "jewelSlots")]
    pub jewel_slots: Vec<u32>,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub alternate_ascendancies: Vec<AlternateAscendancy>,
}

impl TreeData {
    /// Returns nodes in radius (or ring) of a node
    pub fn _nodes_in_radius(&self, center_id: u32, radius_data: &JewelRadiusData, include_blighted: bool) -> Vec<u32> {
        let center_node = &self.nodes[&center_id];
        let (inner_squared, outer_squared) = (radius_data.inner * radius_data.inner, radius_data.outer * radius_data.outer);
        self.nodes.values().filter(|n| {
            if n.group.is_none() || n.skill >= u16::MAX as u32 || (!include_blighted && n.is_blighted) {
                return false;
            }
            let distance = center_node.distance_squared(n);
            distance >= inner_squared as f32 && distance <= outer_squared as f32
        }).map(|n| n.skill).collect()
    }

    /// Returns nodes in radius (or ring) of a node. Uses precomputed values when available.
    pub fn nodes_in_radius(&self, center_id: u32, radius_data: &JewelRadiusData, include_blighted: bool) -> Vec<u32> {
        if let Some(ret) = NODES_IN_RADIUS.get(&(center_id, *radius_data)) {
            return ret.clone();
        }
        self._nodes_in_radius(center_id, radius_data, include_blighted)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ClusterOrbitData {
    pub passives: &'static [u16],
    pub notable: &'static [u16],
    pub orbit: u16,
}

fn calc_angles() -> Vec<Vec<f32>> {
    let mut ret = vec![];
    for skills in &TREE.constants.skills_per_orbit {
        ret.push({
            let angles = match skills {
                16 => vec![0, 30, 45, 60, 90, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330],
                40 => vec![
                    0, 10, 20, 30, 40, 45, 50, 60, 70, 80, 90, 100, 110, 120, 130, 135, 140, 150, 160, 170, 180, 190,
                    200, 210, 220, 225, 230, 240, 250, 260, 270, 280, 290, 300, 310, 315, 320, 330, 340, 350,
                ],
                n => (0..*n).map(|i| (360 * i) / n).collect(),
            };
            angles.into_iter().map(|a| (a as f32).to_radians()).collect()
        });
    }
    ret
}

fn _get_class_node(class: Class) -> u32 {
    TREE.nodes
        .values()
        .find(|n| n.class_start_index == Some(class as i32))
        .unwrap()
        .skill
}

lazy_static! {
    pub static ref ORBIT_ANGLES: Vec<Vec<f32>> = calc_angles();
    pub static ref CLASS_START_NODES: FxHashMap<Class, u32> = {
        let mut ret = FxHashMap::default();
        for class in Class::iter() {
            ret.insert(class, _get_class_node(class));
        }
        ret
    };
    pub static ref NODES_IN_RADIUS: FxHashMap<(u32, JewelRadiusData), Vec<u32>> = {
        let mut ret = FxHashMap::default();

        let mut positions: FxHashMap<u32, (f32, f32)> = FxHashMap::default();
        for (node_id, node) in &TREE.nodes {
            if node.group.is_some() && node.orbit.is_some() && node.orbit_index.is_some() {
                positions.insert(*node_id, node_pos(node));
            }
        }

        let valid_targets: Vec<(&Node, (f32, f32))> = TREE.nodes.values()
            .filter(|n| n.group.is_some() && n.skill < u16::MAX as u32 && !n.is_blighted)
            .filter_map(|n| positions.get(&n.skill).map(|&pos| (n, pos)))
            .collect();

        let radii_configs = [
            JewelRadiusData { inner: 0, outer: 960 },
            JewelRadiusData { inner: 0, outer: 1440 },
            JewelRadiusData { inner: 0, outer: 1800 },
            JewelRadiusData { inner: 0, outer: 2400 },
            JewelRadiusData { inner: 0, outer: 2880 },
            JewelRadiusData { inner: 960, outer: 1320 },
            JewelRadiusData { inner: 1320, outer: 1680 },
            JewelRadiusData { inner: 1680, outer: 2040 },
            JewelRadiusData { inner: 2040, outer: 2400 },
            JewelRadiusData { inner: 2400, outer: 2880 },
        ];

        let radii_squared: Vec<(f32, f32)> = radii_configs.iter()
            .map(|r| ((r.inner * r.inner) as f32, (r.outer * r.outer) as f32))
            .collect();

        for (center_id, center_node) in TREE.nodes.iter().filter(|(_, v)| v.is_jewel_socket || v.is_keystone) {
            if center_node.name == "Medium Jewel Socket" || center_node.name == "Small Jewel Socket" {
                continue;
            }

            let Some(&center_pos) = positions.get(center_id) else { continue };

            let mut buckets: [Vec<u32>; 10] = Default::default();

            for (target_node, target_pos) in &valid_targets {
                if target_node.skill == *center_id {
                    continue;
                }
                let dx = center_pos.0 - target_pos.0;
                let dy = center_pos.1 - target_pos.1;
                let dist_sq = dx * dx + dy * dy;

                for (i, (inner_sq, outer_sq)) in radii_squared.iter().enumerate() {
                    if dist_sq >= *inner_sq && dist_sq <= *outer_sq {
                        buckets[i].push(target_node.skill);
                    }
                }
            }

            for (i, config) in radii_configs.iter().enumerate() {
                ret.insert((*center_id, config.clone()), buckets[i].clone()); 
            }
        }

        ret
    };
}

pub fn node_pos(node: &Node) -> (f32, f32) {
    let group = node.group.unwrap();
    let orbit = node.orbit.unwrap() as usize;
    let angle = ORBIT_ANGLES[orbit][node.orbit_index.unwrap() as usize];
    let orbit_radius = TREE.constants.orbit_radii[orbit];

    (
        TREE.groups[&group].x + (angle.sin() * orbit_radius as f32),
        TREE.groups[&group].y.neg() + (angle.cos() * orbit_radius as f32),
    )
}
