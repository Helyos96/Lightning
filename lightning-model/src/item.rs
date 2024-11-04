use crate::build::{calc_stat, StatId};
use crate::data::ITEMS;
use crate::modifier::{self, parse_mod, Mod, Source, Type};
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
    attack_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseItem {
    name: String,
    tags: FxHashSet<String>,
    implicits: Vec<String>,
    pub item_class: ItemClass,
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
    pub mods_enchant: Vec<String>,
    pub quality: i64,
}

struct LocalModMatch {
    stat: StatId,
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
    static ref LOCAL_MODS_WEAPON: Vec<LocalModMatch> = vec![
        LocalModMatch { stat: StatId::MinPhysicalDamage, typ: modifier::Type::Base },
        LocalModMatch { stat: StatId::MaxPhysicalDamage, typ: modifier::Type::Base },
        LocalModMatch { stat: StatId::PhysicalDamage, typ: modifier::Type::Inc },
        LocalModMatch { stat: StatId::AttackSpeed, typ: modifier::Type::Inc },
    ];
}

fn match_local(m: &Mod, is_weapon: bool) -> bool {
    if !m.conditions.is_empty() || !m.flags.is_empty() {
        return false;
    }
    if is_weapon {
        for local_mod_match in LOCAL_MODS_WEAPON.iter() {
            if local_mod_match.matches(m) {
                return true;
            }
        }
    }
    false
}

impl Item {
    pub fn data(&self) -> &'static BaseItem {
        &ITEMS[&self.base_item]
    }

    /// Compute the damage range for a specific damage type dt
    pub fn calc_dmg(&self, dt: &str) -> Option<(i64, i64)> {
        let base_item = self.data();

        if !base_item.tags.contains("weapon") {
            return None;
        }

        let mods = self.calc_local_mods();

        if dt == "physical" {
            if let Some(min) = base_item.properties.physical_damage_min {
                if let Some(max) = base_item.properties.physical_damage_max {
                    let mut min_stat = calc_stat(StatId::MinPhysicalDamage, &mods, &hset!());
                    let mut max_stat = calc_stat(StatId::MaxPhysicalDamage, &mods, &hset!());
                    let mut dmg = calc_stat(StatId::PhysicalDamage, &mods, &hset!());
                    min_stat.adjust(Type::Base, min, &Mod { ..Default::default() });
                    max_stat.adjust(Type::Base, max, &Mod { ..Default::default() });
                    dmg.adjust(Type::More, self.quality, &Mod { ..Default::default() });
                    min_stat.assimilate(&dmg);
                    max_stat.assimilate(&dmg);
                    return Some((min_stat.val(), max_stat.val()));
                }
            }
        }

        None
    }

    pub fn attack_speed(&self) -> Option<i64> {
        if let Some(attack_time) = self.data().properties.attack_time {
            let mods = self.calc_local_mods();
            let stat_attack_speed = calc_stat(StatId::AttackSpeed, &mods, &hset!());
            return Some(stat_attack_speed.val_custom_inv(attack_time));
        }
        None
    }

    fn calc_mods(&self, local: bool) -> Vec<Mod> {
        let mut mods = vec![];
        let is_weapon = self.data().tags.contains("weapon");

        for m in self.mods_impl.iter().chain(&self.mods_expl).chain(&self.mods_enchant) {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers.into_iter().filter(|m| (local && match_local(m, is_weapon)) || (!local && !match_local(m, is_weapon))));
            }
        }

        mods
    }

    pub fn calc_local_mods(&self) -> Vec<Mod> {
        self.calc_mods(true)
    }

    pub fn calc_nonlocal_mods(&self) -> Vec<Mod> {
        self.calc_mods(false)
    }
}
