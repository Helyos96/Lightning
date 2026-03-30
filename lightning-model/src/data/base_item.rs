use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use enumflags2::bitflags;

use crate::build::Slot;

#[bitflags]
#[repr(u64)]
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
    pub fn allowed_slots(&self) -> &'static [Slot] {
        use ItemClass::*;
        match self {
            TwoHandSword|TwoHandAxe|TwoHandMace|Warstaff|Staff|Bow => &[Slot::Weapon],
            OneHandAxe|OneHandMace|OneHandSword|RuneDagger|Sceptre|ThrustingOneHandSword|Wand => &[Slot::Weapon, Slot::Offhand],
            Quiver|Shield => &[Slot::Offhand],
            Helmet => &[Slot::Helm],
            Amulet => &[Slot::Amulet],
            BodyArmour => &[Slot::BodyArmour],
            Belt => &[Slot::Belt],
            Gloves => &[Slot::Gloves],
            Boots => &[Slot::Boots],
            Ring => &[Slot::Ring, Slot::Ring2],
            Jewel | AbyssJewel => &[Slot::TreeJewel(0)],
            LifeFlask | ManaFlask | HybridFlask | UtilityFlask => &[Slot::Flask(0)],
            _ => &[],
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
    pub block: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    pub level: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseItem {
    pub name: String,
    pub tags: FxHashSet<String>,
    implicits: Vec<String>,
    pub item_class: ItemClass,
    pub properties: Properties,
    pub requirements: Option<Requirements>,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Rarity {
    #[default]
    Normal,
    Magic,
    Rare,
    Unique,
}

impl Rarity {
    pub fn from_str(s: &str) -> Option<Rarity> {
        use Rarity::*;
        match s.to_lowercase().as_str() {
            "normal" => Some(Normal),
            "magic" => Some(Magic),
            "rare" => Some(Rare),
            "unique" => Some(Unique),
            _ => None
        }
    }
}
