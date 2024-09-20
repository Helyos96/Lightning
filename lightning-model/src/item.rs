use crate::data::ITEMS;
use crate::modifier::{self, parse_mod, Mod, Source};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

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
    Shield,
    Sceptre,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMinMax {
    min: u32,
    max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Properties {
    armour: Option<PropertyMinMax>,
    physical_damage_max: Option<i64>,
    physical_damage_min: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseItem {
    name: String,
    tags: FxHashSet<String>,
    implicits: Vec<String>,
    item_class: String,
    properties: Properties,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Rarity {
    #[default]
    Normal,
    Magic,
    Rare,
    Unique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub base_item: String,
    pub name: String,
    pub rarity: Rarity,
    pub mods_impl: Vec<String>,
    pub mods_expl: Vec<String>,
}

struct LocalModMatch {
    stat: String,
    typ: modifier::Type,
}

impl LocalModMatch {
    fn matches(&self, m: &Mod) -> bool {
        if m.stat == self.stat && m.typ == self.typ {
            return true;
        }
        false
    }
}

lazy_static! {
    static ref LOCAL_MODS: Vec<LocalModMatch> = vec![
        LocalModMatch { stat: "physical minimum damage".to_string(), typ: modifier::Type::Base },
        LocalModMatch { stat: "physical maximum damage".to_string(), typ: modifier::Type::Base },
        LocalModMatch { stat: "physical damage".to_string(), typ: modifier::Type::Inc },
    ];
}

fn match_local(m: &Mod) -> bool {
    for local_mod_match in LOCAL_MODS.iter() {
        if local_mod_match.matches(m) {
            return true;
        }
    }
    false
}

impl Item {
    pub fn data(&self) -> &'static BaseItem {
        &ITEMS[&self.base_item]
    }

    pub fn calc_dmg(&self, dt: &str) -> (i64, i64) {
        let base_item = self.data();

        if !base_item.tags.contains("weapon") {
            eprintln!("Calling calc_dmg on non-weapon item {}", base_item.name);
            return (0, 0);
        }

        if dt == "physical" {
            if let Some(min) = base_item.properties.physical_damage_min {
                if let Some(max) = base_item.properties.physical_damage_max {
                    return (min, max);
                }
            }
        }

        (0, 0)
    }

    pub fn calc_local_dmg_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        for m in &self.mods_impl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers.into_iter().filter(|parsed_mod| match_local(parsed_mod)));
            }
        }
        for m in &self.mods_expl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers.into_iter().filter(|parsed_mod| match_local(parsed_mod)));
            }
        }

        mods
    }

    pub fn calc_nonlocal_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        for m in &self.mods_impl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers.into_iter().filter(|parsed_mod| !match_local(parsed_mod)));
            }
        }
        for m in &self.mods_expl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers.into_iter().filter(|parsed_mod| !match_local(parsed_mod)));
            }
        }

        mods
    }
}
