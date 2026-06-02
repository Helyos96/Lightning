pub mod property;
pub mod stat;
pub mod evaluator;
pub mod buff;

use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::{fs, io};
use std::path::Path;

use crate::build::evaluator::Evaluator;
use crate::data::base_item::ItemClass;
use crate::data::gem::{ActiveSkillType, GemTag};
use crate::data::{MONSTER_STATS, TREE};
use crate::gem::Gem;
use crate::item::Item;
use crate::modifier::{Condition, Mod, ModEffect, ModFlag, Mutation, Source, Type};
use crate::modparser::parse_mod;
use crate::stackvec;
use crate::tree::PassiveTree;
use base64::prelude::*;
use enumflags2::{BitFlags, make_bitflags};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use lazy_static::lazy_static;
use stat::{Stat, StatId, Stats};
use strum::EnumCount;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Serialize, Deserialize, Default, Eq, PartialEq, Hash, Clone, Copy, Debug, strum_macros::Display)]
pub enum Slot {
    #[default]
    Helm,
    BodyArmour,
    Gloves,
    Boots,
    Belt,
    Amulet,
    Weapon,
    Offhand,
    Ring(u16),
    Flask(u16), // u16 -> Flask slot
    TreeJewel(u32), // u32 -> Tree node holding the jewel
    AbyssalJewel(u16), // u16 -> Number of abyssal socket
}

impl Slot {
    pub fn compatible(&self, other: Slot) -> bool {
        match (self, other) {
            (Slot::Ring(_), Slot::Ring(_)) => true,
            (Slot::Flask(_), Slot::Flask(_)) => true,
            (Slot::TreeJewel(_), Slot::TreeJewel(_)) => true,
            (Slot::AbyssalJewel(_), Slot::AbyssalJewel(_)) => true,
            _ => self == &other,
        }
    }
}

impl TryFrom<(&str, u16)> for Slot {
    type Error = ();

    fn try_from((inventory_id, x): (&str, u16)) -> Result<Self, Self::Error> {
        match inventory_id {
            "Helm" => Ok(Slot::Helm),
            "BodyArmour" => Ok(Slot::BodyArmour),
            "Gloves" => Ok(Slot::Gloves),
            "Boots" => Ok(Slot::Boots),
            "Belt" => Ok(Slot::Belt),
            "Amulet" => Ok(Slot::Amulet),
            "Weapon" => Ok(Slot::Weapon),
            "Offhand" => Ok(Slot::Offhand),
            "Ring" => Ok(Slot::Ring(0)),
            "Ring2" => Ok(Slot::Ring(1)),
            "Ring3" => Ok(Slot::Ring(2)),
            "Flask" => {
                if x <= 4 {
                    Ok(Slot::Flask(x))
                } else {
                    Err(())
                }
            }
            "PassiveJewels" => {
                if let Some(node) = TREE.jewel_slots.get(x as usize) {
                    Ok(Slot::TreeJewel(*node))
                } else {
                    Err(())
                }
            }
            "AbyssalJewel" => Ok(Slot::AbyssalJewel(x)),
            _ => Err(())
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Defence {
    Armour,
    Evasion,
    EnergyShield,
    Block,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GemLink {
    pub gems: Vec<Gem>,
    pub slot: Option<Slot>,
}

impl GemLink {
    pub fn active_gems(&self) -> impl Iterator<Item = &Gem> {
        self.gems.iter().filter(|g| g.data().active_skill.is_some())
    }
    pub fn support_gems(&self) -> impl Iterator<Item = &Gem> {
        self.gems.iter().filter(|g| g.data().is_support)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr, EnumIter)]
pub enum BanditChoice {
    Alira,
    Kraityn,
    Oak,
    #[default]
    KillAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr, EnumIter)]
pub enum CampaignChoice {
    #[default]
    Beach,
    ActFive,
    ActTen,
}

lazy_static! {
pub static ref BANDIT_STATS: FxHashMap<BanditChoice, Vec<Mod>> = {
        let mut ret = FxHashMap::default();
        ret.insert(BanditChoice::Alira, vec![
            Mod::stat(StatId::FireResistance, Type::Base, 15),
            Mod::stat(StatId::ColdResistance, Type::Base, 15),
            Mod::stat(StatId::LightningResistance, Type::Base, 15),
        ]);
        ret.insert(BanditChoice::Kraityn, vec![
            Mod::stat(StatId::MovementSpeed, Type::Inc, 8),
        ]);
        ret.insert(BanditChoice::Oak, vec![
            Mod::stat(StatId::MaximumLife, Type::Base, 40),
        ]);
        ret.insert(BanditChoice::KillAll, vec![
            Mod::stat(StatId::PassiveSkillPoints, Type::Base, 1),
        ]);
        ret
    };

    pub static ref CAMPAIGN_STATS: FxHashMap<CampaignChoice, Vec<Mod>> = {
        let mut ret = FxHashMap::default();
        ret.insert(CampaignChoice::Beach, vec![]);
        ret.insert(CampaignChoice::ActFive, vec![
            Mod::stat(StatId::FireResistance, Type::Base, -30),
            Mod::stat(StatId::ColdResistance, Type::Base, -30),
            Mod::stat(StatId::LightningResistance, Type::Base, -30),
            Mod::stat(StatId::ChaosResistance, Type::Base, -30),
        ]);
        ret.insert(CampaignChoice::ActTen, vec![
            Mod::stat(StatId::FireResistance, Type::Base, -60),
            Mod::stat(StatId::ColdResistance, Type::Base, -60),
            Mod::stat(StatId::LightningResistance, Type::Base, -60),
            Mod::stat(StatId::ChaosResistance, Type::Base, -60),
        ]);
        ret
    };

    static ref BASE_MODES: Vec<Mod> = vec![
        Mod::stat(StatId::MaximumLife, Type::Base, 12).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::Level))]),
        Mod::stat(StatId::MaximumLife, Type::Base, 38),
        Mod::stat(StatId::MaximumLife, Type::Base, 1).with_mutations(stackvec![Mutation::MultiplierStat((2, StatId::Strength))]),
        Mod::stat(StatId::MaximumEnergyShield, Type::Inc, 1).with_mutations(stackvec![Mutation::MultiplierStat((10, StatId::Intelligence))]),
        Mod::stat(StatId::MaximumMana, Type::Base, 6).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::Level))]),
        Mod::stat(StatId::MaximumMana, Type::Base, 34),
        Mod::stat(StatId::MaximumMana, Type::Base, 1).with_mutations(stackvec![Mutation::MultiplierStat((2, StatId::Intelligence))]),
        Mod::stat(StatId::ManaRegenerationPct, Type::Base, 180),
        Mod::stat(StatId::MaximumFrenzyCharges, Type::Base, 3),
        Mod::stat(StatId::MaximumPowerCharges, Type::Base, 3),
        Mod::stat(StatId::MaximumEnduranceCharges, Type::Base, 3),
        Mod::stat(StatId::MaximumRage, Type::Base, 30),
        Mod::stat(StatId::Damage, Type::More, 1)
            .with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::Rage))])
            .with_tags(GemTag::Attack),
        Mod::stat(StatId::PassiveSkillPoints, Type::Base, 1).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::Level))]),
        Mod::stat(StatId::PassiveSkillPoints, Type::Base, 22), // 23 from quests -1 for level 1
        Mod::stat(StatId::PhysicalDamage, Type::Inc, 1)
            .with_mutations(stackvec![Mutation::MultiplierStat((5, StatId::Strength))])
            .with_tags(GemTag::Melee)
            .with_flags(ModFlag::Hit),
        Mod::stat(StatId::Damage, Type::More, 4).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))]),
        Mod::stat(StatId::AttackSpeed, Type::Inc, 4).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))]),
        Mod::stat(StatId::CastSpeed, Type::Inc, 4).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))]),
        Mod::stat(StatId::CriticalStrikeChance, Type::Inc, 50).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::PowerCharges))]),
        Mod::stat(StatId::MaximumFireResistance, Type::Base, 75),
        Mod::stat(StatId::MaximumColdResistance, Type::Base, 75),
        Mod::stat(StatId::MaximumLightningResistance, Type::Base, 75),
        Mod::stat(StatId::MaximumChaosResistance, Type::Base, 75),
        Mod::stat(StatId::AccuracyRating, Type::Base, 2).with_mutations(stackvec![Mutation::MultiplierStat((1, StatId::Dexterity))]),
        Mod::stat(StatId::AccuracyRating, Type::Base, 2),
        Mod::stat(StatId::AccuracyRating, Type::Base, 2).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::Level))]),
        Mod::stat(StatId::EvasionRating, Type::Base, 15),
        Mod::stat(StatId::EvasionRating, Type::Inc, 1).with_mutations(stackvec![Mutation::MultiplierStat((5, StatId::Dexterity))]),
        Mod::stat(StatId::CriticalStrikeMultiplier, Type::Base, 150),
        Mod::stat(StatId::MaximumFortification, Type::Base, 20),
        Mod::stat(StatId::AttackSpeed, Type::More, 10).with_conditions(stackvec![Condition::WhileDualWielding]),
        Mod::stat(StatId::ChanceToBlockAttackDamage, Type::Base, 20).with_conditions(stackvec![Condition::WhileDualWielding]),
        Mod::stat(StatId::MaximumChanceToBlockAttackDamage, Type::Base, 75),
        Mod::stat(StatId::MaximumChanceToBlockSpellDamage, Type::Base, 75),
        Mod::stat(StatId::RingSlots, Type::Base, 2),
    ];
}

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Build {
    pub name: String,
    pub gem_links: Vec<GemLink>,
    #[serde_as(as = "FxHashMap<serde_with::json::JsonString, _>")]
    equipment: FxHashMap<Slot, usize>, // usize is index into inventory
    pub inventory: Vec<Arc<Item>>,
    pub tree: PassiveTree,
    #[serde(default)]
    pub bandit_choice: BanditChoice,
    #[serde(default)]
    pub campaign_choice: CampaignChoice,
    properties_int: FxHashMap<property::Int, i64>,
    properties_bool: FxHashMap<property::Bool, bool>,
    #[serde(default)]
    properties_always_max: FxHashSet<property::Int>,
    pub import_account: Option<(String, String)>,
    #[serde(default)]
    pub custom_mods: Arc<Vec<String>>,
    #[serde(default)]
    flask_enabled: FxHashSet<u16>,
    #[serde(default)]
    pub gemlink_cur: usize, // Currently selected gemlink
    #[serde(default)]
    pub active_skill_cur: usize, // Currently selected active skill within gemlink
}

impl Build {
    pub fn new_player() -> Build {
        let mut ret = Build {
            name: "Untitled Build".to_string(),
            ..Default::default()
        };
        ret.set_property_int(property::Int::Level, 1);
        ret
    }

    pub fn set_custom_mods(&mut self, mods: Vec<String>) {
        self.custom_mods = Arc::new(mods);
    }

    pub fn is_flask_enabled(&self, idx: u16) -> bool {
        self.flask_enabled.contains(&idx)
    }

    pub fn set_flask_enabled(&mut self, idx: u16, enabled: bool) {
        if idx > 4 {
            return;
        }

        if enabled {
            self.flask_enabled.insert(idx);
        } else {
            self.flask_enabled.remove(&idx);
        }
    }

    pub fn update_item_allocations(&mut self) {
        self.tree.nodes_additional.clear();
        self.tree.invalidate_modcache();
        let mut max_abyssal_sockets = 0;
        let equipment_slots: Vec<(Slot, usize)> = self.equipment.iter().map(|(k, v)| (*k, *v)).collect();
        for (slot, idx) in equipment_slots {
            if matches!(slot, Slot::AbyssalJewel(_)) {
                continue;
            }
            let item_mods = self.inventory[idx].calc_nonlocal_mods();

            for m in item_mods.iter() {
                if let Some(stat) = m.as_stat() &&
                   stat.stat == stat::StatId::AbyssalSockets
                {
                    max_abyssal_sockets += stat.amount;
                }
                if let Some(n) = m.as_allocate() {
                    if !self.tree.nodes_additional.contains(&n) {
                        self.tree.nodes_additional.push(n);
                    }
                }
            }
        }

        self.equipment.retain(|k, _| {
            if let Slot::AbyssalJewel(idx) = k {
                *idx < max_abyssal_sockets as u16
            } else {
                true
            }
        });
    }

    /// Returns mods from the following sources:
    /// Innate, Passive Tree, Items, Global Skills (Auras..)
    pub fn calc_mods(&self, _include_global: bool) -> Vec<Mod> {
        let class_data = &TREE.classes[&self.tree.class];
        let mut mods = Vec::with_capacity(600);
        mods.extend_from_slice(&BASE_MODES);
        mods.extend_from_slice(&[
            Mod::stat(StatId::Strength, Type::Base, class_data.base_str),
            Mod::stat(StatId::Dexterity, Type::Base, class_data.base_dex),
            Mod::stat(StatId::Intelligence, Type::Base, class_data.base_int),
        ]);
        mods.extend_from_slice(BANDIT_STATS.get(&self.bandit_choice).unwrap());
        mods.extend_from_slice(CAMPAIGN_STATS.get(&self.campaign_choice).unwrap());
        mods.extend_from_slice(&self.tree.calc_mods());
        for (slot, idx) in &self.equipment {
            let item = &self.inventory[*idx];
            if let Slot::TreeJewel(_) = slot {
                // Mods from jewels are added by tree.calc_mods()
                continue;
            } else if let Slot::Flask(flask_idx) = slot {
                if self.is_flask_enabled(*flask_idx) {
                    for m in item.calc_nonlocal_mods().iter() {
                        let mut new_mod = m.to_owned();
                        new_mod.source = Source::Item(*slot);
                        mods.push(new_mod);
                    }
                }
            } else {
                for m in item.calc_nonlocal_mods().iter() {
                    if let ModEffect::ReflectOppositeRing = m.effect &&
                       let Slot::Ring(i) = slot && *i < 2 &&
                       let Some(opposite_ring) = self.get_equipped(Slot::Ring(1 - i))
                    {
                        for m in opposite_ring.calc_nonlocal_mods().iter() {
                            let mut new_mod = m.to_owned();
                            new_mod.source = Source::Item(*slot);
                            mods.push(new_mod);
                        }
                    } else {
                        let mut new_mod = m.to_owned();
                        new_mod.source = Source::Item(*slot);
                        mods.push(new_mod);
                    }
                }
            }
        }
        for mod_str in self.custom_mods.iter() {
            if let Some(mut config_mods) = parse_mod(mod_str, Source::Custom("Config")) {
                mods.append(&mut config_mods);
            }
        }
        mods.extend(self.calc_mods_gem_buffs_auras(&mods));
        mods
    }

    pub fn calc_mods_monster(level: i64) -> Vec<Mod> {
        let default_stats = MONSTER_STATS.get(&level).unwrap();
        let mods = vec![
            Mod::stat(StatId::MaximumLife, Type::Base, default_stats.life),
            Mod::stat(StatId::EvasionRating, Type::Base, default_stats.evasion),
            Mod::stat(StatId::Armour, Type::Base, default_stats.armour),
        ];
        mods
    }

    pub fn calc_mods_gem_buffs_auras(&self, mods: &[Mod]) -> Vec<Mod> {
        // Find best unique active auras
        let mut best_gems: FxHashMap<&str, (&Gem, &GemLink)> = FxHashMap::default();
        for link in &self.gem_links {
            for active_gem in link.active_gems().filter(|gem| gem.enabled) {
                if let Some((existing_gem, _)) = best_gems.get(active_gem.id.as_str()) {
                    if existing_gem.level >= active_gem.level {
                        continue;
                    }
                }
                best_gems.insert(active_gem.id.as_str(), (active_gem, link));
            }
        }

        let mut ret = vec![];
        for (gem, link) in best_gems.values() {
            let skill_types = if let Some(active_skill) = &gem.data().active_skill {
                &active_skill.types
            } else {
                &FxHashSet::default()
            };
            let mut eval = Evaluator::new(self, mods, gem.data().tags, make_bitflags!(ModFlag::{Aura | Buff | Curse}), skill_types, None, link.slot);
            //eval.resolve();
            let mut best_supports: FxHashMap<&str, &Gem> = FxHashMap::default();

            // Find best unique support gems in link
            for support_gem in link.support_gems() {
                if support_gem.can_support(gem) {
                    if let Some(existing_gem) = best_supports.get(support_gem.id.as_str()) {
                        if existing_gem.level >= support_gem.level {
                            continue;
                        }
                    }
                    best_supports.insert(support_gem.id.as_str(), support_gem);
                }
            }

            for support in best_supports.values().filter(|gem| gem.enabled) {
                eval.ctx.extra_mods.extend_from_slice(&support.calc_mods(false, eval.gem_level_extra(support.data().tags), 0));
            }

            let extra_level = eval.gem_level_extra(gem.data().tags);
            eval.ctx.extra_mods.extend_from_slice(&gem.calc_mods(false, extra_level, 0));
            let mut mods = (*gem.calc_mods(true, extra_level, 0)).clone();
            for m in mods.iter_mut() {
                if m.flags.contains(ModFlag::Aura) &&
                   let Some(mstat) = m.as_stat_mut()
                {
                    mstat.mutations.push(Mutation::CustomMult(eval.eval_stat(StatId::AuraEffect).mult()));
                }
                if m.flags.contains(ModFlag::Curse) &&
                   let Some(mstat) = m.as_stat_mut()
                {
                    mstat.mutations.push(Mutation::CustomMult(eval.eval_stat(StatId::CurseEffect).mult()));
                }
                if m.flags.contains(ModFlag::Buff) &&
                   let Some(mstat) = m.as_stat_mut()
                {
                    mstat.mutations.push(Mutation::CustomMult(eval.eval_stat(StatId::BuffEffect).mult()));
                }
            }
            ret.extend(mods);
        }
        ret
    }

    pub fn remove_inventory(&mut self, idx_remove: usize) {
        if idx_remove >= self.inventory.len() {
            eprintln!("Trying to remove inventory item {idx_remove} but len is {}", self.inventory.len());
            return;
        }
        // Remove slots where the item is equipped 
        let equipped_slots: Vec<Slot> = self.equipment.iter().filter(|(_, v)| **v == idx_remove).map(|(k, _)| k).copied().collect();
        for slot in equipped_slots {
            self.unequip(slot);
        }
        // Adjust slot idx in remaining equipment
        for idx in self.equipment.values_mut() {
            if *idx >= idx_remove {
                *idx -= 1;
            }
        }
        self.inventory.remove(idx_remove);
    }

    pub fn equipment(&self) -> &FxHashMap<Slot, usize> {
        &self.equipment
    }

    pub fn swap_item_inventory(&mut self, item_idx: usize, item: Arc<Item>) {
        assert!(item_idx < self.inventory.len());
        let to_reequip: Vec<Slot> = self.equipment.iter().filter_map(|(slot, idx)| {
            if *idx == item_idx {
                Some(*slot)
            } else {
                None
            }
        }).collect();

        for slot in &to_reequip {
            self.unequip(*slot);
        }

        self.inventory[item_idx] = item;

        for slot in to_reequip {
            self.equip(slot, item_idx);
        }
    }

    pub fn equip(&mut self, slot: Slot, item_idx: usize) {
        assert!(item_idx < self.inventory.len());
        self.unequip(slot);
        self.equipment.insert(slot, item_idx);

        if self.inventory[item_idx].allocates_nodes() {
            self.update_item_allocations();
        }

        let new_item_class = self.inventory[item_idx].data().item_class;
        if slot == Slot::Weapon && ItemClass::TWO_HANDED.contains(new_item_class) {
            let has_quiver = self.get_equipped(Slot::Offhand)
                .is_some_and(|offhand| offhand.data().item_class == ItemClass::Quiver);

            if !(new_item_class == ItemClass::Bow && has_quiver) {
                self.unequip(Slot::Offhand);
            }
        }
        if slot == Slot::Offhand && let Some(weapon) = self.get_equipped(Slot::Weapon) {
            let weapon_class = weapon.data().item_class;

            if ItemClass::TWO_HANDED.contains(weapon_class) {
                let is_valid_bow_setup = weapon_class == ItemClass::Bow && new_item_class == ItemClass::Quiver;

                if !is_valid_bow_setup {
                    self.unequip(Slot::Weapon);
                }
            }
        }

        if let Slot::TreeJewel(jewel_node_id) = slot {
            self.tree.add_jewel(jewel_node_id, self.inventory[item_idx].clone(), false);
        }

        for (gem_id, level) in self.inventory[item_idx].calc_nonlocal_mods().iter().filter_map(|m| m.as_support_gem()) {
            for link in self.gem_links.iter_mut().filter(|link| link.slot == Some(slot)) {
                let mut gem = Gem::new(gem_id.to_string(), true, level, 0, 0);
                gem.granted_by = Some(slot);
                link.gems.push(gem);
            }
        }

        for (gem_id, level) in self.inventory[item_idx].calc_nonlocal_mods().iter().filter_map(|m| m.as_active_skill()) {
            let mut gem = Gem::new(gem_id.to_string(), true, level, 0, 0);
            gem.granted_by = Some(slot);
            self.gem_links.push(GemLink { gems: vec![gem], slot: None });
        }
    }

    pub fn unequip(&mut self, slot: Slot) {
        if !self.equipment.contains_key(&slot) {
            return;
        }

        let item_idx = self.equipment[&slot];
        self.equipment.remove(&slot);
        if self.inventory[item_idx].allocates_nodes() {
            self.update_item_allocations();
        }

        if let Slot::TreeJewel(node_id) = slot {
            let removed_sockets = self.tree.remove_jewel(node_id);
            for socket in removed_sockets {
                self.equipment.remove(&Slot::TreeJewel(socket));
            }
        }

        for link in self.gem_links.iter_mut() {
            link.gems.retain(|gem| gem.granted_by != Some(slot));
        }

        self.gem_links.retain(|link| !link.gems.is_empty());
    }

    pub fn get_equipped(&self, slot: Slot) -> Option<&Item> {
        if let Some(idx) = self.equipment.get(&slot) {
            assert!(self.inventory.len() > *idx);
            return Some(&self.inventory[*idx]);
        }
        None
    }

    pub fn set_property_int_maxed(&mut self, p: property::Int, maxed: bool) {
        if maxed {
            self.properties_always_max.insert(p);
        } else {
            self.properties_always_max.remove(&p);
        }
    }

    pub fn is_property_int_maxed(&self, p: property::Int) -> bool {
        if self.properties_always_max.contains(&p) {
            true
        } else {
            false
        }
    }

    pub fn property_int(&self, p: property::Int) -> i64 {
        let min = {
            match property::int_data(p).min {
                property::Val::Val(i) => i,
                property::Val::Stat(_) => i64::MIN
            }
        };
        let max = {
            match property::int_data(p).max {
                property::Val::Val(i) => i,
                property::Val::Stat(_) => i64::MAX
            }
        };
        self.properties_int.get(&p).copied().unwrap_or(0).clamp(min, max)
    }

    pub fn property_int_stats(&self, p: property::Int, stats: &Stats) -> i64 {
        let mut min = match property::int_data(p).min {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => stats.val(s),
        };
        let max = match property::int_data(p).max {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => stats.val(s),
        };

        if self.is_property_int_maxed(p) {
            return max;
        }
        min = min.min(max);
        self.property_int(p).clamp(min, max)
    }

    pub fn property_bool(&self, p: property::Bool) -> bool {
        return self.properties_bool.get(&p).copied().unwrap_or(false);
    }

    pub fn set_property_int(&mut self, p: property::Int, val: i64) {
        self.properties_int.insert(p, val);
    }

    pub fn set_property_bool(&mut self, p: property::Bool, val: bool) {
        self.properties_bool.insert(p, val);
    }

    pub fn is_holding(&self, item_classes: &BitFlags<ItemClass>) -> bool {
        self.equipment.iter().find(|(_, idx)| item_classes.contains(self.inventory[**idx].data().item_class)).is_some()
    }

    pub fn calc_stats(&self, mods: &[Mod], tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>) -> Stats {
        let types = FxHashSet::default();
        let mut evaluator = Evaluator::new(self, mods, tags, flags, &types, None, None);
        evaluator.resolve();
        evaluator.resolve_stats();
        Stats { stats: evaluator.cache.resolved_stats }
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        let mut file_path = dir.join(&self.name);
        file_path.set_extension("json");
        serde_json::to_writer(&fs::File::create(file_path)?, &self)?;
        Ok(())
    }

    /// Base64(zstd compress(json serialize))
    pub fn code(&self) -> Result<String, Box<dyn Error>> {
        let json_bytes = serde_json::to_vec(self)?;
        let compressed_bytes = zstd::stream::encode_all(json_bytes.as_slice(), 9)?;
        Ok(BASE64_STANDARD.encode(&compressed_bytes))
    }

    pub fn decode(encoded: &str) -> Result<Self, Box<dyn Error>> {
        let compressed_bytes = BASE64_STANDARD.decode(encoded)?;
        let json_bytes = zstd::stream::decode_all(compressed_bytes.as_slice())?;
        let build = serde_json::from_slice(&json_bytes)?;
        Ok(build)
    }
}

#[test]
fn test_build() {
    let player = Build::new_player();
    let stats = player.calc_stats(&player.calc_mods(true), BitFlags::EMPTY, BitFlags::EMPTY);

    assert_eq!(stats.stat(StatId::MaximumLife).val(), 60);
}
