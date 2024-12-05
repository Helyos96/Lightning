use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::build::Slot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemClass {
    Unarmed,
    Ring,
    Amulet,
    Claw,
    Dagger,
    Wand,
    Bow,
    Staff,
    Warstaff,
    Shield,
    Sceptre,
    FishingRod,
    Quiver,
    Boots,
    Belt,
    Helmet,
    Gloves,
    LifeFlask,
    ManaFlask,
    HybridFlask,
    UtilityFlask,
    AbyssJewel,
    Jewel,
    #[serde(rename = "Body Armour")]
    BodyArmour,
    #[serde(rename = "Rune Dagger")]
    RuneDagger,
    #[serde(rename = "One Hand Sword")]
    OneHandSword,
    #[serde(rename = "Thrusting One Hand Sword")]
    ThrustingOneHandSword,
    #[serde(rename = "One Hand Axe")]
    OneHandAxe,
    #[serde(rename = "One Hand Mace")]
    OneHandMace,
    #[serde(rename = "Two Hand Sword")]
    TwoHandSword,
    #[serde(rename = "Two Hand Axe")]
    TwoHandAxe,
    #[serde(rename = "Two Hand Mace")]
    TwoHandMace,
}

impl ItemClass {
    pub fn allowed_slots(&self) -> Vec<Slot> {
        use ItemClass::*;
        match self {
            TwoHandSword|TwoHandAxe|TwoHandMace|Warstaff|Staff|Bow => vec![Slot::Weapon],
            OneHandAxe|OneHandMace|OneHandSword|RuneDagger|Sceptre|ThrustingOneHandSword => vec![Slot::Weapon, Slot::Offhand],
            Quiver|Shield => vec![Slot::Offhand],
            Helmet => vec![Slot::Helm],
            Amulet => vec![Slot::Amulet],
            BodyArmour => vec![Slot::BodyArmour],
            Belt => vec![Slot::Belt],
            Gloves => vec![Slot::Gloves],
            Boots => vec![Slot::Boots],
            Ring => vec![Slot::Ring, Slot::Ring2],
            Jewel => vec![Slot::TreeJewel(0)],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PropertyMinMax {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Properties {
    pub armour: Option<PropertyMinMax>,
    pub physical_damage_max: Option<i64>,
    pub physical_damage_min: Option<i64>,
    pub attack_time: Option<i64>,
    pub evasion: Option<PropertyMinMax>,
    pub energy_shield: Option<PropertyMinMax>,
    pub critical_strike_chance: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseItem {
    name: String,
    pub tags: FxHashSet<String>,
    implicits: Vec<String>,
    pub item_class: ItemClass,
    pub properties: Properties,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Rarity {
    #[default]
    Normal,
    Magic,
    Rare,
    Unique,
}
