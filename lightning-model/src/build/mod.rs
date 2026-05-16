pub mod property;
pub mod stat;
pub mod evaluator;

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
use crate::modifier::{Condition, Mod, ModFlag, Mutation, Source, Type, parse_mod};
use crate::stackvec;
use crate::tree::PassiveTree;
use enumflags2::BitFlags;
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
    Ring,
    Ring2,
    Flask(u16), // u16 -> Flask slot
    TreeJewel(u32), // u32 -> Tree node holding the jewel
    AbyssalJewel(u16), // u16 -> Number of abyssal socket
}

impl Slot {
    pub fn compatible(&self, other: Slot) -> bool {
        match (self, other) {
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
            "Ring" => Ok(Slot::Ring),
            "Ring2" => Ok(Slot::Ring2),
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
    EnergyShield
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct GemLink {
    //pub active_gems: Vec<Gem>,
    //pub support_gems: Vec<Gem>,
    pub gems: Vec<Arc<Gem>>,
    pub slot: Slot,
}

impl GemLink {
    pub fn active_gems(&self) -> impl Iterator<Item = &Arc<Gem>> {
        self.gems.iter().filter(|g| g.data().active_skill.is_some())
    }
    pub fn support_gems(&self) -> impl Iterator<Item = &Arc<Gem>> {
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
        Mod::stat(StatId::CriticalStrikeMultiplier, Type::Base, 150),
        Mod::stat(StatId::MaximumFortification, Type::Base, 20),
        Mod::stat(StatId::AttackSpeed, Type::More, 10).with_conditions(stackvec![Condition::WhileDualWielding]),
        Mod::stat(StatId::ChanceToBlockAttackDamage, Type::Base, 20).with_conditions(stackvec![Condition::WhileDualWielding]),
    ];
}

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Build {
    pub name: String,
    pub gem_links: Vec<GemLink>,
    #[serde_as(as = "FxHashMap<serde_with::json::JsonString, _>")]
    // usize is index into inventory
    equipment: FxHashMap<Slot, usize>,
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

    pub fn calc_buffs_auras(&self) -> Vec<Mod> {
        let mut best_gems: FxHashMap<&str, &Gem> = FxHashMap::default();
        for link in &self.gem_links {
            for active_gem in link.active_gems().filter(|gem| gem.enabled && (gem.data().active_skill.as_ref().unwrap().types.contains(&ActiveSkillType::Aura) || gem.data().active_skill.as_ref().unwrap().types.contains(&ActiveSkillType::Buff))) {
                if let Some(existing_gem) = best_gems.get(active_gem.id.as_str()) {
                    if existing_gem.level >= active_gem.level {
                        continue;
                    }
                }
                best_gems.insert(active_gem.id.as_str(), active_gem);
            }
        }

        let mut ret = vec![];
        for gem in best_gems.values() {
            ret.extend_from_slice(&gem.calc_mods(true));
        }
        ret
    }

    /// Returns mods from the following sources:
    /// Innate, Passive Tree, Items, Global Skills (Auras..)
    pub fn calc_mods(&self, include_global: bool) -> Vec<Mod> {
        let class_data = &TREE.classes[&self.tree.class];
        let mut mods = Vec::with_capacity(600);
        mods.extend_from_slice(&BASE_MODES);
        mods.extend_from_slice(&[
            Mod::stat(StatId::Strength, Type::Base, class_data.base_str),
            Mod::stat(StatId::Dexterity, Type::Base, class_data.base_dex),
            Mod::stat(StatId::Intelligence, Type::Base, class_data.base_int),
        ]);
        mods.append(&mut BANDIT_STATS.get(&self.bandit_choice).unwrap().clone());
        mods.append(&mut CAMPAIGN_STATS.get(&self.campaign_choice).unwrap().clone());
        let jewels: FxHashMap<u32, Arc<Item>> = self.equipment.iter().filter_map(|(k, v)| {
            if let Slot::TreeJewel(id) = k {
                Some((*id, self.inventory[*v].clone()))
            } else {
                None
            }
        }).collect();
        mods.extend_from_slice(&self.tree.calc_mods(&jewels));
        for (slot, idx) in &self.equipment {
            let item = &self.inventory[*idx];
            if let Slot::TreeJewel(node_id) = slot {
                if self.tree.nodes.contains(node_id) {
                    let effect = item.jewel_effect_distance_class();
                    let distance = if effect.is_some() {
                        self.tree.distance_to_class_start(*node_id)
                    } else {
                        0
                    } as i64;

                    for m in item.calc_nonlocal_mods().iter() {
                        let mut new_mod = m.to_owned();
                        new_mod.source = Source::Item(*slot);

                        // Apply increased effect for tree jewels with the "distance to class node" modifier
                        if let Some(effect) = effect &&
                           let Some(stat) = new_mod.as_stat_mut() &&
                           distance > 2
                        {
                            stat.mutations.push(Mutation::IncreasedEffect(effect * (distance - 2)));
                        }

                        mods.push(new_mod);
                    }
                }
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
                    let mut new_mod = m.to_owned();
                    new_mod.source = Source::Item(*slot);
                    mods.push(new_mod);
                }
                let defence = item.calc_defence();
                if defence.armour.val() != 0 {
                    mods.push(Mod::stat(StatId::Armour, Type::Base, defence.armour.val()).with_source(Source::Item(*slot)));
                }
                if defence.energy_shield.val() != 0 {
                    mods.push(Mod::stat(StatId::MaximumEnergyShield, Type::Base, defence.energy_shield.val()).with_source(Source::Item(*slot)));
                }
                if defence.evasion.val() != 0 {
                    mods.push(Mod::stat(StatId::EvasionRating, Type::Base, defence.evasion.val()).with_source(Source::Item(*slot)));
                }
                if defence.block_chance.val() != 0 {
                    mods.push(Mod::stat(StatId::ChanceToBlockAttackDamage, Type::Base, defence.block_chance.val()).with_source(Source::Item(*slot)));
                }
            }
        }
        for mod_str in self.custom_mods.iter() {
            if let Some(mut config_mods) = parse_mod(mod_str, Source::Custom("Config")) {
                mods.append(&mut config_mods);
            }
        }
        if include_global {
            mods.append(&mut self.calc_buffs_auras());
        }
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

        if slot == Slot::Weapon && ItemClass::TWO_HANDED.contains(self.inventory[item_idx].data().item_class) {
            self.unequip(Slot::Offhand);
        }

        if slot == Slot::Offhand &&
           let Some(item) = self.get_equipped(Slot::Weapon) &&
           ItemClass::TWO_HANDED.contains(item.data().item_class)
        {
            self.unequip(Slot::Weapon);
        }

        if let Slot::TreeJewel(jewel_node_id) = slot {
            self.tree.add_jewel(jewel_node_id, &self.inventory[item_idx]);
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
            let removed_sockets = self.tree.remove_jewel(node_id, &self.inventory[item_idx]);
            for socket in removed_sockets {
                self.unequip(Slot::TreeJewel(socket));
            }
            self.equipment.retain(|k, _| {
                if let Slot::TreeJewel(node_id) = k && !self.tree.nodes_data.contains_key(node_id) {
                    false
                } else {
                    true
                }
            });
        }
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
        let min = match property::int_data(p).min {
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
        let mut evaluator = Evaluator::new(self, mods, tags, flags, None);

        let stat_ids: Vec<StatId> = evaluator.mods_by_stat.keys().copied().collect();

        for stat_id in stat_ids {
            evaluator.eval_stat(stat_id);
        }

        Stats { stats: evaluator.resolved_stats }
    }

    pub fn calc_stats_slot(&self, mods: &[Mod], tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>, slot: Slot) -> Stats {
        let mut evaluator = Evaluator::new(self, mods, tags, flags, Some(slot));

        let stat_ids: Vec<StatId> = evaluator.mods_by_stat.keys().copied().collect();

        for stat_id in stat_ids {
            evaluator.eval_stat(stat_id);
        }

        Stats { stats: evaluator.resolved_stats }
    }

    pub fn calc_stat(&self, stat_id: StatId, mods: &[Mod], tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>) -> Stat {
        let mut evaluator = Evaluator::new(self, mods, tags, flags, None);

        evaluator.eval_stat(stat_id).clone()
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        let mut file_path = dir.join(&self.name);
        file_path.set_extension("json");
        serde_json::to_writer(&fs::File::create(file_path)?, &self)?;
        Ok(())
    }
}

#[test]
fn test_build() {
    let player = Build::new_player();
    let stats = player.calc_stats(&player.calc_mods(true), BitFlags::EMPTY, BitFlags::EMPTY);

    assert_eq!(stats.stat(StatId::MaximumLife).val(), 60);
}
