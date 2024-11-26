use crate::build::{calc_stat, Slot, Stat, StatId};
use crate::data::base_item::{BaseItem, Rarity};
use crate::data::ITEMS;
use crate::modifier::{self, parse_mod, Mod, Source, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

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
    static ref LOCAL_MODS_ARMOUR: Vec<LocalModMatch> = vec![
        LocalModMatch { stat: StatId::EvasionRating, typ: modifier::Type::Base },
        LocalModMatch { stat: StatId::EvasionRating, typ: modifier::Type::Inc },
        LocalModMatch { stat: StatId::Armour, typ: modifier::Type::Base },
        LocalModMatch { stat: StatId::Armour, typ: modifier::Type::Inc },
        LocalModMatch { stat: StatId::EnergyShield, typ: modifier::Type::Base },
        LocalModMatch { stat: StatId::EnergyShield, typ: modifier::Type::Inc },
    ];
}

fn match_local(m: &Mod, match_table: &[LocalModMatch]) -> bool {
    if !m.conditions.is_empty() || !m.flags.is_empty() {
        return false;
    }
    for local_mod_match in match_table {
        if local_mod_match.matches(m) {
            return true;
        }
    }
    false
}

#[derive(Debug, Default)]
pub struct DefenceCalc {
    pub armour: Stat,
    pub evasion: Stat,
    pub energy_shield: Stat,
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

    pub fn calc_defence(&self) -> DefenceCalc {
        let mut ret = DefenceCalc::default();
        let base_item = self.data();
        if !base_item.tags.contains("armour") {
            return ret;
        }
        let mods = self.calc_local_mods();

        // TODO: sacred orb defence adjusting instead of average
        if let Some(armour_prop) = &base_item.properties.armour {
            ret.armour.adjust_mod(&Mod { typ: Type::Base, amount: ((armour_prop.min + armour_prop.max) / 2) as i64, ..Default::default() });
        }
        if let Some(energy_shield) = base_item.properties.energy_shield {
            ret.energy_shield.adjust_mod(&Mod { typ: Type::Base, amount: ((energy_shield.min + energy_shield.max) / 2) as i64, ..Default::default() });
        }
        if let Some(evasion) = base_item.properties.evasion {
            ret.evasion.adjust_mod(&Mod { typ: Type::Base, amount: ((evasion.min + evasion.max) / 2) as i64, ..Default::default() });
        }
        ret.armour.assimilate(&calc_stat(StatId::Armour, &mods, &hset!()));
        ret.energy_shield.assimilate(&calc_stat(StatId::MaximumEnergyShield, &mods, &hset!()));
        ret.evasion.assimilate(&calc_stat(StatId::EvasionRating, &mods, &hset!()));
        ret.armour.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});
        ret.energy_shield.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});
        ret.evasion.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});

        return ret;
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
        let mut match_table: &[LocalModMatch] = &[];
        let tags = &self.data().tags;

        if tags.contains("weapon") {
            match_table = &LOCAL_MODS_WEAPON;
        } else if tags.contains("armour") {
            match_table = &LOCAL_MODS_ARMOUR;
        }

        for m in self.mods_impl.iter().chain(&self.mods_expl).chain(&self.mods_enchant) {
            if let Some(modifiers) = parse_mod(m, Source::Innate) {
                mods.extend(modifiers.into_iter().filter(|m| (local && match_local(m, match_table)) || (!local && !match_local(m, match_table))));
            }
        }

        mods
    }

    fn calc_local_mods(&self) -> Vec<Mod> {
        self.calc_mods(true)
    }

    pub fn calc_nonlocal_mods(&self, slot: Slot) -> Vec<Mod> {
        let mut mods = self.calc_mods(false);
        for m in &mut mods {
            m.source = Source::Item(slot);
        }
        mods
    }
}
