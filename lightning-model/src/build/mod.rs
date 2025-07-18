pub mod property;
pub mod stat;

use std::{fs, io};
use std::path::Path;

use crate::data::base_item::ItemClass;
use crate::data::gem::GemTag;
use crate::data::{MONSTER_STATS, TREE};
use crate::gem::Gem;
use crate::item::Item;
use crate::modifier::{Condition, Mod, Mutation, Source, Type};
use crate::tree::PassiveTree;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use lazy_static::lazy_static;
use stat::{Stat, StatId, Stats};
use strum_macros::{AsRefStr, EnumIter};

#[derive(Serialize, Deserialize, Default, Eq, PartialEq, Hash, Clone, Copy, Debug)]
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
    TreeJewel(u16), // u16 -> Tree node holding the jewel
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
            _ => Err(())
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GemLink {
    //pub active_gems: Vec<Gem>,
    //pub support_gems: Vec<Gem>,
    pub gems: Vec<Gem>,
    pub slot: Slot,
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
            Mod {
                stat: StatId::FireResistance,
                typ: Type::Base,
                amount: 15,
                ..Default::default()
            },
            Mod {
                stat: StatId::ColdResistance,
                typ: Type::Base,
                amount: 15,
                ..Default::default()
            },
            Mod {
                stat: StatId::LightningResistance,
                typ: Type::Base,
                amount: 15,
                ..Default::default()
            },
        ]);
        ret.insert(BanditChoice::Kraityn, vec![
            Mod {
                stat: StatId::MovementSpeed,
                typ: Type::Inc,
                amount: 8,
                ..Default::default()
            },
        ]);
        ret.insert(BanditChoice::Oak, vec![
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: 40,
                ..Default::default()
            },
        ]);
        ret.insert(BanditChoice::KillAll, vec![
            Mod {
                stat: StatId::PassiveSkillPoints,
                typ: Type::Base,
                amount: 1,
                ..Default::default()
            },
        ]);
        ret
    };

    pub static ref CAMPAIGN_STATS: FxHashMap<CampaignChoice, Vec<Mod>> = {
        let mut ret = FxHashMap::default();
        ret.insert(CampaignChoice::Beach, vec![]);
        ret.insert(CampaignChoice::ActFive, vec![
            Mod {
                stat: StatId::FireResistance,
                typ: Type::Base,
                amount: -30,
                ..Default::default()
            },
            Mod {
                stat: StatId::ColdResistance,
                typ: Type::Base,
                amount: -30,
                ..Default::default()
            },
            Mod {
                stat: StatId::LightningResistance,
                typ: Type::Base,
                amount: -30,
                ..Default::default()
            },
            Mod {
                stat: StatId::ChaosResistance,
                typ: Type::Base,
                amount: -30,
                ..Default::default()
            },
        ]);
        ret.insert(CampaignChoice::ActTen, vec![
            Mod {
                stat: StatId::FireResistance,
                typ: Type::Base,
                amount: -60,
                ..Default::default()
            },
            Mod {
                stat: StatId::ColdResistance,
                typ: Type::Base,
                amount: -60,
                ..Default::default()
            },
            Mod {
                stat: StatId::LightningResistance,
                typ: Type::Base,
                amount: -60,
                ..Default::default()
            },
            Mod {
                stat: StatId::ChaosResistance,
                typ: Type::Base,
                amount: -60,
                ..Default::default()
            },
        ]);
        ret
    };
}

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Build {
    pub name: String,
    pub ascendancy: i32,
    pub gem_links: Vec<GemLink>,
    #[serde_as(as = "FxHashMap<serde_with::json::JsonString, _>")]
    // usize is index into inventory
    pub equipment: FxHashMap<Slot, usize>,
    pub inventory: Vec<Item>,
    pub tree: PassiveTree,
    #[serde(default)]
    pub bandit_choice: BanditChoice,
    #[serde(default)]
    pub campaign_choice: CampaignChoice,
    properties_int: FxHashMap<property::Int, i64>,
    properties_bool: FxHashMap<property::Bool, bool>,
}

impl Build {
    pub fn new_player() -> Build {
        let mut ret = Build {
            name: "Untitled Build".to_string(),
            ascendancy: 0,
            ..Default::default()
        };
        ret.set_property_int(property::Int::Level, 1);
        ret
    }

    /// Returns mods from the following sources:
    /// Innate, Passive Tree, Items, Global Skills (Auras..)
    /// todo: add some caching to not parse & collect all mods
    /// every time.
    pub fn calc_mods(&self, _include_global: bool) -> Vec<Mod> {
        let class_data = &TREE.classes[&self.tree.class];
        let mut mods = vec![
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: 12,
                mutations: vec![Mutation::MultiplierProperty((1, property::Int::Level))],
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: 38,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: 1,
                mutations: vec![Mutation::MultiplierStat((2, StatId::Strength))],
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumFrenzyCharges,
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumPowerCharges,
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumEnduranceCharges,
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumRage,
                typ: Type::Base,
                amount: 30,
                ..Default::default()
            },
            Mod {
                stat: StatId::Strength,
                typ: Type::Base,
                amount: class_data.base_str,
                ..Default::default()
            },
            Mod {
                stat: StatId::Dexterity,
                typ: Type::Base,
                amount: class_data.base_dex,
                ..Default::default()
            },
            Mod {
                stat: StatId::Intelligence,
                typ: Type::Base,
                amount: class_data.base_int,
                ..Default::default()
            },
            Mod {
                stat: StatId::Damage,
                typ: Type::More,
                amount: 1,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::Rage)),
                ],
                tags: hset![GemTag::Attack],
                ..Default::default()
            },
            Mod {
                stat: StatId::PassiveSkillPoints,
                typ: Type::Base,
                amount: 1,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::Level)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::PassiveSkillPoints,
                typ: Type::Base,
                amount: 22, // 23 from quests -1 for level 1
                ..Default::default()
            },
            Mod {
                stat: StatId::PhysicalDamage,
                typ: Type::Inc,
                amount: 1,
                mutations: vec![Mutation::MultiplierStat((5, StatId::Strength))],
                tags: hset![GemTag::Melee],
                ..Default::default()
            },
            Mod {
                stat: StatId::Damage,
                typ: Type::More,
                amount: 4,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::AttackSpeed,
                typ: Type::Inc,
                amount: 4,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::CastSpeed,
                typ: Type::Inc,
                amount: 4,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::CriticalStrikeChance,
                typ: Type::Inc,
                amount: 50,
                mutations: vec![
                    Mutation::MultiplierProperty((1, property::Int::PowerCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumFireResistance,
                typ: Type::Base,
                amount: 75,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumColdResistance,
                typ: Type::Base,
                amount: 75,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumLightningResistance,
                typ: Type::Base,
                amount: 75,
                ..Default::default()
            },
            Mod {
                stat: StatId::MaximumChaosResistance,
                typ: Type::Base,
                amount: 75,
                ..Default::default()
            },
            Mod {
                stat: StatId::AccuracyRating,
                typ: Type::Base,
                amount: 2,
                mutations: vec![Mutation::MultiplierStat((1, StatId::Dexterity))],
                ..Default::default()
            },
            Mod {
                stat: StatId::CriticalStrikeMultiplier,
                typ: Type::Base,
                amount: 150,
                ..Default::default()
            },
        ];
        mods.extend(BANDIT_STATS.get(&self.bandit_choice).unwrap().clone());
        mods.extend(CAMPAIGN_STATS.get(&self.campaign_choice).unwrap().clone());
        mods.extend(self.tree.calc_mods());
        for (slot, idx) in &self.equipment {
            let item = &self.inventory[*idx];
            if let Slot::TreeJewel(node_id) = slot {
                if self.tree.nodes.contains(node_id) {
                    mods.extend(item.calc_nonlocal_mods(*slot));
                }
            } else {
                mods.extend(item.calc_nonlocal_mods(*slot));
                let defence = item.calc_defence();
                if defence.armour.val() != 0 {
                    mods.push(Mod { stat: StatId::Armour, typ: Type::Base, amount: defence.armour.val(), source: Source::Item(*slot), ..Default::default() });
                }
                if defence.energy_shield.val() != 0 {
                    mods.push(Mod { stat: StatId::MaximumEnergyShield, typ: Type::Base, amount: defence.energy_shield.val(), source: Source::Item(*slot), ..Default::default() });
                }
                if defence.evasion.val() != 0 {
                    mods.push(Mod { stat: StatId::EvasionRating, typ: Type::Base, amount: defence.evasion.val(), source: Source::Item(*slot), ..Default::default() });
                }
            }
        }
        // TODO auras
        /*if include_global {
            for gl in &self.gem_links {
                for ag in gl.active_gems().filter(|g| g.data().tags.contains(&GemTag::Aura)) {
                    mods.extend(ag.calc_mods());
                }
            }
        }*/
        mods
    }

    pub fn calc_mods_monster(&self, level: i64) -> Vec<Mod> {
        let default_stats = MONSTER_STATS.get(&level).unwrap();
        let mods = vec![
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: default_stats.life,
                ..Default::default()
            },
            Mod {
                stat: StatId::EvasionRating,
                typ: Type::Base,
                amount: default_stats.evasion,
                ..Default::default()
            },
            Mod {
                stat: StatId::Armour,
                typ: Type::Base,
                amount: default_stats.armour,
                ..Default::default()
            },
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
            self.equipment.remove(&slot);
        }
        // Adjust slot idx in remaining equipment
        for idx in self.equipment.values_mut() {
            if *idx >= idx_remove {
                *idx -= 1;
            }
        }
        self.inventory.remove(idx_remove);
    }

    pub fn get_equipped(&self, slot: Slot) -> Option<&Item> {
        if let Some(idx) = self.equipment.get(&slot) {
            return Some(&self.inventory[*idx]);
        }
        None
    }

    pub fn property_int_stats(&self, p: property::Int, stats: &FxHashMap<StatId, Stat>) -> i64 {
        let min = {
            match property::int_data(p).min {
                property::Val::Val(i) => i,
                property::Val::Stat(s) => {
                    if let Some(stat) = stats.get(&s) {
                        stat.val()
                    } else {
                        0
                    }
                }
            }
        };
        let max = {
            match property::int_data(p).max {
                property::Val::Val(i) => i,
                property::Val::Stat(s) => {
                    if let Some(stat) = stats.get(&s) {
                        stat.val()
                    } else {
                        0
                    }
                }
            }
        };
        self.property_int(p).clamp(min, max)
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

    pub fn property_bool(&self, p: property::Bool) -> bool {
        return self.properties_bool.get(&p).copied().unwrap_or(false);
    }

    pub fn set_property_int(&mut self, p: property::Int, val: i64) {
        self.properties_int.insert(p, val);
    }

    pub fn set_property_bool(&mut self, p: property::Bool, val: bool) {
        self.properties_bool.insert(p, val);
    }

    pub fn is_holding(&self, item_classes: &FxHashSet<ItemClass>) -> bool {
        self.equipment.iter().find(|(_, idx)| item_classes.contains(&self.inventory[**idx].data().item_class)).is_some()
    }

    fn check_conditions(&self, stats: &FxHashMap<StatId, Stat>, m: &Mod) -> bool {
        for c in &m.conditions {
            match c {
                Condition::GreaterEqualProperty(mutation) => {
                    if self.property_int_stats(mutation.1, stats) < mutation.0 {
                        return false;
                    }
                },
                Condition::LesserEqualProperty(mutation) => {
                    if self.property_int_stats(mutation.1, stats) > mutation.0 {
                        return false;
                    }
                },
                Condition::GreaterEqualStat(mutation) => {
                    if let Some(stat) = stats.get(&mutation.1) {
                        if stat.val() < mutation.0 {
                            return false;
                        }
                    }
                },
                Condition::LesserEqualStat(mutation) => {
                    if let Some(stat) = stats.get(&mutation.1) {
                        if stat.val() > mutation.0 {
                            return false;
                        }
                    } else {
                        return false;
                    }
                },
                Condition::PropertyBool(mutation) => {
                    if self.property_bool(mutation.1) != mutation.0 {
                        return false;
                    }
                },
                Condition::WhileWielding(weapons) => {
                    if !self.is_holding(weapons) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn apply_mutations(&self, stats: &FxHashMap<StatId, Stat>, m: &Mod) -> i64 {
        let mut amount = m.amount;
        for f in &m.mutations {
            match f {
                Mutation::MultiplierProperty(mutation) => {
                    amount = (amount * self.property_int_stats(mutation.1, stats)) / mutation.0;
                },
                Mutation::MultiplierStat(mutation) => {
                    amount = match stats.get(&mutation.1) {
                        Some(stat) => (amount * stat.val()) / mutation.0,
                        None => amount,
                    }
                },
            }
        }
        amount
    }

    pub fn calc_stats(&self, mods: &[Mod], tags: &FxHashSet<GemTag>) -> Stats {
        let mut stats: FxHashMap<StatId, Stat> = Default::default();
        let mut mods_sec_pass = vec![];
        let mut mods_third_pass = vec![];

        for m in mods {
            if !tags.is_superset(&m.tags) {
                continue;
            }
            if !m.weapons.is_empty() && !self.is_holding(&m.weapons) {
                continue;
            }
            if !m.conditions.is_empty() {
                mods_third_pass.push(m);
                continue;
            }
            if !m.mutations.is_empty() {
                mods_sec_pass.push(m);
                continue;
            }
            stats.entry(m.stat).or_default().adjust(m.typ, m.amount, m);
        }

        for m in mods_sec_pass {
            let amount = self.apply_mutations(&stats, m);
            stats.entry(m.stat).or_default().adjust(m.typ, amount, m);
        }

        for m in mods_third_pass {
            let amount = self.apply_mutations(&stats, m);
            if !self.check_conditions(&stats, m) {
                continue;
            }
            stats.entry(m.stat).or_default().adjust(m.typ, amount, m);
        }

        Stats { stats }
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
    let stats = player.calc_stats(&player.calc_mods(true), &hset![]);

    assert_eq!(stats.stat(StatId::MaximumLife).val(), 60);
}
