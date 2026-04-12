use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, IntoStaticStr, EnumIter};
use serde_with::{serde_as, DisplayFromStr};

#[derive(Default, Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, EnumString, AsRefStr)]
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

#[derive(Copy, Clone, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug)]
pub struct ClusterOrbitData {
    pub passives: &'static [u16],
    pub notable: &'static [u16],
    pub orbit: u16,
}

const ORBIT_DATA_SMALL: ClusterOrbitData = ClusterOrbitData{
    passives: &[0, 3, 5],
    notable: &[5],
    orbit: 1,
};

const ORBIT_DATA_MEDIUM: ClusterOrbitData = ClusterOrbitData{
    passives: &[7, 12, 1, 13, 9, 4],
    notable: &[9, 4],
    orbit: 2,
};

const ORBIT_DATA_LARGE: ClusterOrbitData = ClusterOrbitData{
    passives: &[9, 3, 0, 13, 5, 11, 1, 7, 12],
    notable: &[1, 7, 12],
    orbit: 3,
};

pub fn get_cluster_orbit_data(base_type: &str) -> Option<&ClusterOrbitData> {
    match base_type {
        "Small Cluster Jewel" => Some(&ORBIT_DATA_SMALL),
        "Medium Cluster Jewel" => Some(&ORBIT_DATA_MEDIUM),
        "Large Cluster Jewel" => Some(&ORBIT_DATA_LARGE),
        _ => None
    }
}