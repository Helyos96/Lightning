use crate::build::stat::{calc_stat, Stat, StatId};
use crate::build::Slot;
use crate::data::base_item::{BaseItem, Rarity};
use crate::data::tree::Node;
use crate::data::{DAMAGE_GROUPS, DamageType, ITEMS, TREE};
use crate::modifier::{self, parse_mod, Mod, Source, Type};
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::str::FromStr;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Item {
    pub base_item: String,
    pub name: String,
    pub rarity: Rarity,
    pub mods_impl: Vec<String>,
    pub mods_expl: Vec<String>,
    pub mods_enchant: Vec<String>,
    pub quality: i64,
    #[serde(default)]
    pub corrupted: bool,
    #[serde(default)]
    pub item_level: i64,
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

const LOCAL_MODS_WEAPON: &[LocalModMatch] = &[
    LocalModMatch { stat: StatId::AddedMinPhysicalDamage, typ: modifier::Type::Base },
    LocalModMatch { stat: StatId::AddedMaxPhysicalDamage, typ: modifier::Type::Base },
    LocalModMatch { stat: StatId::PhysicalDamage, typ: modifier::Type::Inc },
    LocalModMatch { stat: StatId::AttackSpeed, typ: modifier::Type::Inc },
    LocalModMatch { stat: StatId::AccuracyRating, typ: modifier::Type::Base },
    LocalModMatch { stat: StatId::AccuracyRating, typ: modifier::Type::Override },
    LocalModMatch { stat: StatId::CriticalStrikeChance, typ: modifier::Type::Inc },
];

const LOCAL_MODS_ARMOUR: &[LocalModMatch] = &[
    LocalModMatch { stat: StatId::EvasionRating, typ: modifier::Type::Base },
    LocalModMatch { stat: StatId::EvasionRating, typ: modifier::Type::Inc },
    LocalModMatch { stat: StatId::Armour, typ: modifier::Type::Base },
    LocalModMatch { stat: StatId::Armour, typ: modifier::Type::Inc },
    LocalModMatch { stat: StatId::EnergyShield, typ: modifier::Type::Base },
    // TODO: corrupted implicits max ES are global
    LocalModMatch { stat: StatId::EnergyShield, typ: modifier::Type::Inc },
    LocalModMatch { stat: StatId::ChanceToBlockAttackDamage, typ: modifier::Type::Inc },
];

fn match_local(m: &Mod, match_table: &[LocalModMatch]) -> bool {
    if !m.conditions.is_empty() || !m.mutations.is_empty() || m.global {
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
    pub block_chance: Stat,
}

impl Item {
    pub fn data(&self) -> &'static BaseItem {
        &ITEMS[&self.base_item]
    }

    // Attempt to parse a cluster jewel
    pub fn get_cluster(&self) -> Option<(i64, u32, Vec<&Node>)> {
        if !self.data().name.ends_with("Cluster Jewel") {
            return None;
        }

        let passive_skills_amount = calc_stat(StatId::AllocatesPassiveSkills, &self.calc_nonlocal_mods(Slot::Helm)).val();
        if passive_skills_amount == 0 {
            return None;
        }
        let small_passive_node = calc_stat(StatId::AddedPassiveSkillsGrantNode, &self.calc_nonlocal_mods(Slot::Helm)).val();
        if small_passive_node == 0 {
            return None;
        }

        let notables: Vec<&Node> = self.mods_expl.iter().filter_map(|m| {
            let m = m.strip_prefix("1 Added Passive Skill is ")?;
            TREE.nodes.values().find(|n| &n.name == m)
        }).collect();

        Some((passive_skills_amount - notables.len() as i64 - 2, small_passive_node as u32, notables))
    }

    pub fn name(&self) -> &str {
        if !self.name.is_empty() {
            &self.name
        } else {
            &self.data().name
        }
    }

    /// Compute the damage range for a specific damage type dt
    pub fn calc_dmg(&self, dt: DamageType) -> Option<(i64, i64)> {
        let base_item = self.data();

        if !base_item.tags.contains("weapon") {
            return None;
        }

        let mods = self.calc_local_mods();
        let group = DAMAGE_GROUPS.iter().find(|dg| dg.damage_type == dt).unwrap();
        let mut min_stat = calc_stat(group.added_min_id, &mods);
        let mut max_stat = calc_stat(group.added_max_id, &mods);
        let mut dmg = calc_stat(group.stat_id, &mods);
        if dt == DamageType::Physical &&
           let Some(min) = base_item.properties.physical_damage_min &&
           let Some(max) = base_item.properties.physical_damage_max {
            min_stat.adjust(Type::Base, min);
            max_stat.adjust(Type::Base, max);
            dmg.adjust(Type::More, self.quality);
        }
        min_stat.assimilate(&dmg);
        max_stat.assimilate(&dmg);
        let ret = (min_stat.val(), max_stat.val());

        if ret != (0, 0) {
            return Some(ret);
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
        ret.armour.assimilate(&calc_stat(StatId::Armour, &mods));
        ret.energy_shield.assimilate(&calc_stat(StatId::MaximumEnergyShield, &mods));
        ret.evasion.assimilate(&calc_stat(StatId::EvasionRating, &mods));
        ret.armour.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});
        ret.energy_shield.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});
        ret.evasion.adjust_mod(&Mod { typ: Type::More, amount: self.quality, ..Default::default()});
        ret.block_chance.adjust_mod(&Mod { typ: Type::Base, amount: self.block_chance().unwrap_or(0), ..Default::default()});

        return ret;
    }

    pub fn accuracy(&self) -> Stat {
        let mods = self.calc_local_mods();
        calc_stat(StatId::AccuracyRating, &mods)
    }

    pub fn attack_speed(&self) -> Option<i64> {
        if let Some(attack_time) = self.data().properties.attack_time {
            let mods = self.calc_local_mods();
            let stat_attack_speed = calc_stat(StatId::AttackSpeed, &mods);
            return Some(stat_attack_speed.val_custom_inv(attack_time));
        }
        None
    }

    pub fn crit_chance(&self) -> Option<i64> {
        if let Some(crit_chance) = self.data().properties.critical_strike_chance {
            let mods = self.calc_local_mods();
            let mut stat_crit_chance = calc_stat(StatId::CriticalStrikeChance, &mods);
            stat_crit_chance.adjust_mod(&Mod { typ: Type::Base, amount: crit_chance, ..Default::default() });
            return Some(stat_crit_chance.val());
        }
        None
    }

    pub fn block_chance(&self) -> Option<i64> {
        if let Some(block_chance) = self.data().properties.block {
            let mods = self.calc_local_mods();
            let mut stat_block_chance = calc_stat(StatId::ChanceToBlockAttackDamage, &mods);
            stat_block_chance.adjust_mod(&Mod { typ: Type::Base, amount: block_chance, ..Default::default() });
            return Some(stat_block_chance.val());
        }
        None
    }

    fn calc_mods(&self, local: bool) -> Vec<Mod> {
        let mut mods = Vec::with_capacity(12);
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

    // Parse an item from CTRL+C text
    pub fn from_str(text: &str) -> Option<Item> {
        let mut item = Item::default();
        let mut found_name = false;
        let mut found_class = false;
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty() && l != &"--------").collect();

        for line in lines {
            let line = line.strip_suffix(" (augmented)").unwrap_or(line);
            if let Some(rarity) = line.strip_prefix("Rarity: ") {
                item.rarity = Rarity::from_str(rarity).unwrap_or_default();
                continue;
            }
            if !found_class {
                let potentiel_base_item = line.strip_prefix("Synthesised ").unwrap_or(line);
                if ITEMS.contains_key(potentiel_base_item) {
                    item.base_item = potentiel_base_item.to_owned();
                    found_class = true;
                    continue;
                }
            }
            if line == "Corrupted" {
                item.corrupted = true;
                continue;
            }
            if let Some(item_level_str) = line.strip_prefix("Item Level: ") {
                item.item_level = i64::from_str(item_level_str).unwrap_or_default();
                continue;
            }
            if let Some(quality_str) = line.strip_prefix("Quality: +") {
                if let Some(quality_str) = quality_str.strip_suffix("%") {
                    item.quality = i64::from_str(quality_str).unwrap_or_default();
                }
                continue;
            }
            if line == "Requirements:" || line.starts_with("Level:") || line.starts_with("Str:") ||
               line.starts_with("Dex:") || line.starts_with("Int:") || line.starts_with("Sockets:") ||
               line.starts_with("Note:") || line.starts_with("Item Class:") || line.starts_with("Armour:") ||
               line.starts_with("Energy Shield:") || line.starts_with("Evasion Rating:") || line.starts_with("Physical Damage:") ||
               line.starts_with("Elemental Damage:") || line.starts_with("Attacks per Second:")  ||
               line.starts_with("Critical Strike Chance:") || line.starts_with("Weapon Range:") {
                continue;
            }
            if let Some(enchant) = line.strip_suffix(" (enchant)") {
                item.mods_enchant.push(enchant.to_owned());
                continue;
            }
            if let Some(implicit) = line.strip_suffix(" (implicit)") {
                item.mods_impl.push(implicit.to_owned());
                continue;
            }
            if !found_class && !found_name {
                item.name = line.to_owned();
                found_name = true;
                continue;
            }
            let line = line.strip_suffix(" (crafted)").unwrap_or(line);
            item.mods_expl.push(line.to_owned());
        }

        match found_class {
            true => Some(item),
            false => None,
        }
    }

    pub fn to_str(&self) -> String {
        let mut output: String = Default::default();

        output += format!("Rarity: {:?}\n", self.rarity).as_str();
        if !self.name.is_empty() {
            output += format!("{}\n", self.name).as_str();
        }
        output += format!("{}\n", self.data().name).as_str();
        output += "--------\n";
        output += format!("Item Level: {}\n", self.item_level).as_str();
        output += "--------\n";

        if let Some(reqs) = &self.data().requirements {
            if reqs.level > 0 || reqs.strength > 0 || reqs.dexterity > 0 || reqs.intelligence > 0 {
                output += "Requirements:\n";
                if reqs.level > 0 {
                output += format!("Level: {}\n", reqs.level).as_str();
                }
                if reqs.strength > 0 {
                    output += format!("Str: {}\n", reqs.strength).as_str();
                }
                if reqs.dexterity > 0 {
                    output += format!("Dex: {}\n", reqs.dexterity).as_str();
                }
                if reqs.intelligence > 0 {
                    output += format!("Int: {}\n", reqs.intelligence).as_str();
                }
            }
            output += "--------\n";
        }

        for m in &self.mods_enchant {
            output += format!("{} (enchant)\n", m).as_str();
        }
        if !self.mods_enchant.is_empty() {
            output += "--------\n";
        }
        for m in &self.mods_impl {
            output += format!("{} (implicit)\n", m).as_str();
        }
        if !self.mods_impl.is_empty() {
            output += "--------\n";
        }
        for m in &self.mods_expl {
            output += format!("{}\n", m).as_str();
        }
        if !self.mods_expl.is_empty() {
            output += "--------\n";
        }
        if self.corrupted {
            output += "Corrupted\n";
        }

        output
    }
}
