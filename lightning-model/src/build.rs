use crate::data::TREE;
use crate::gem::{Gem, GemTag};
use crate::item::{Item, ItemClass};
use crate::modifier::{Condition, Mod, Mutation, PropertyBool, PropertyInt, Type};
use crate::tree::{Class, PassiveTree, TreeData};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use lazy_static::lazy_static;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub enum Slot {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StatId {
    #[default]
    Strength,
    Dexterity,
    Intelligence,
    Attributes,
    AttackSpeed,
    CastSpeed,
    WarcrySpeed,
    CooldownRecoverySpeed,
    ProjectileSpeed,
    TrapThrowingSpeed,
    ChanceToBlockAttackDamage,
    ChanceToBlockSpellDamage,
    ChanceToSuppressSpellDamage,
    FireDamageOverTimeMultiplier,
    ColdDamageOverTimeMultiplier,
    ChaosDamageOverTimeMultiplier,
    PhysicalDamageOverTimeMultiplier,
    DamageOverTimeMultiplier,
    FireDamageOverTime,
    ColdDamageOverTime,
    ChaosDamageOverTime,
    PhysicalDamageOverTime,
    DamageOverTime,
    MinFireDamage,
    MaxFireDamage,
    FireDamage,
    ColdDamage,
    LightningDamage,
    ChaosDamage,
    MinPhysicalDamage,
    MaxPhysicalDamage,
    PhysicalDamage,
    Damage,
    AreaOfEffect,
    AccuracyRating,
    MovementSpeed,
    SkillEffectDuration,
    Duration,
    ImpaleEffect,
    MinimumFrenzyCharges,
    MinimumPowerCharges,
    MinimumEnduranceCharges,
    MaximumFrenzyCharges,
    MaximumPowerCharges,
    MaximumEnduranceCharges,
    MaximumLife,
    MaximumMana,
    MinimumRage,
    MaximumRage,
    MaximumEnergyShield,
    EnergyShieldRechargeRate,
    LifeRegenerationRate,
    ManaRegenerationRate,
    ManaReservationEfficiency,
    CriticalStrikeChance,
    CriticalStrikeMultiplier,
    Armour,
    EvasionRating,
    StunThreshold,
    ChanceToAvoidBeingStunned,
    MaximumFireResistance,
    MaximumColdResistance,
    MaximumLightningResistance,
    MaximumChaosResistance,
    FireResistance,
    ColdResistance,
    LightningResistance,
    ChaosResistance,
    FlaskChargesGained,
    FlaskEffectDuration,
    FlaskRecoveryRate,
    FlaskChargesUsed,
    ManaCost,
    LifeCost,
    Cost,
    LifeRegeneration,
    LifeRegenerationPct,
    PassiveSkillPoints,
}

#[derive(Serialize, Deserialize)]
pub struct GemLink {
    pub active_gems: Vec<Gem>,
    pub support_gems: Vec<Gem>,
    pub slot: Slot,
}

#[derive(Debug, Clone)]
pub struct Stat {
    base: i64,
    inc: i64,
    more: i64,
    mods: Vec<Mod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, AsRefStr, EnumIter)]
pub enum BanditChoice {
    Alira,
    Kraityn,
    Oak,
    #[default]
    KillAll,
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
}

#[serde_as]
#[derive(Default, Serialize, Deserialize)]
pub struct Build {
    pub name: String,
    pub ascendancy: i32,
    pub gem_links: Vec<GemLink>,
    #[serde_as(as = "FxHashMap<serde_with::json::JsonString, _>")]
    pub equipment: FxHashMap<Slot, Item>,
    pub inventory: Vec<Item>,
    pub tree: PassiveTree,
    #[serde(default)]
    pub bandit_choice: BanditChoice,
    properties_int: FxHashMap<PropertyInt, i64>,
    properties_bool: FxHashMap<PropertyBool, bool>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    stats: FxHashMap<StatId, Stat>,
}

impl Stats {
    pub fn stat(&self, s: StatId) -> Stat {
        self.stats.get(&s).cloned().unwrap_or_default()
    }
}

impl Build {
    pub fn new_player() -> Build {
        let mut ret = Build {
            name: "Untitled Build".to_string(),
            ascendancy: 0,
            ..Default::default()
        };
        ret.set_property_int(PropertyInt::Level, 1);
        ret
    }

    /// Returns mods from the following sources:
    /// Innate, Passive Tree, Items, Global Skills (Auras..)
    /// todo: add some caching to not parse & collect all mods
    /// every time.
    pub fn calc_mods(&self, include_global: bool) -> Vec<Mod> {
        let class_data = &TREE.classes[&self.tree.class];
        let mut mods = vec![
            Mod {
                stat: StatId::MaximumLife,
                typ: Type::Base,
                amount: 12,
                flags: vec![Mutation::MultiplierProperty((1, PropertyInt::Level))],
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
                flags: vec![Mutation::MultiplierStat((2, StatId::Strength))],
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
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::Rage)),
                ],
                tags: hset![GemTag::Attack],
                ..Default::default()
            },
            Mod {
                stat: StatId::PassiveSkillPoints,
                typ: Type::Base,
                amount: 1,
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::Level)),
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
                flags: vec![Mutation::MultiplierStat((5, StatId::Strength))],
                tags: hset![GemTag::Melee],
                ..Default::default()
            },
            Mod {
                stat: StatId::Damage,
                typ: Type::More,
                amount: 4,
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::AttackSpeed,
                typ: Type::Inc,
                amount: 4,
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::CastSpeed,
                typ: Type::Inc,
                amount: 4,
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::FrenzyCharges)),
                ],
                ..Default::default()
            },
            Mod {
                stat: StatId::CriticalStrikeChance,
                typ: Type::Inc,
                amount: 50,
                flags: vec![
                    Mutation::MultiplierProperty((1, PropertyInt::PowerCharges)),
                ],
                ..Default::default()
            },
        ];
        mods.extend(BANDIT_STATS.get(&self.bandit_choice).unwrap().clone());
        mods.extend(self.tree.calc_mods());
        for item in self.equipment.values() {
            mods.extend(item.calc_nonlocal_mods());
        }
        if include_global {
            for gl in &self.gem_links {
                for ag in gl.active_gems.iter().filter(|g| g.data().tags.contains(&GemTag::Aura)) {
                    mods.extend(ag.calc_mods());
                }
            }
        }
        mods
    }

    pub fn property_int(&self, p: PropertyInt) -> i64 {
        return self.properties_int.get(&p).copied().unwrap_or(0);
    }

    pub fn property_bool(&self, p: PropertyBool) -> bool {
        return self.properties_bool.get(&p).copied().unwrap_or(false);
    }

    pub fn set_property_int(&mut self, p: PropertyInt, val: i64) {
        self.properties_int.insert(p, val);
    }

    pub fn set_property_bool(&mut self, p: PropertyBool, val: bool) {
        self.properties_bool.insert(p, val);
    }

    pub fn is_holding(&self, item_classes: &FxHashSet<ItemClass>) -> bool {
        self.equipment.iter().find(|(_, item)| item_classes.contains(&item.data().item_class)).is_some()
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
            if !m.flags.is_empty() {
                mods_sec_pass.push(m);
                continue;
            }

            stats.entry(m.stat).or_default().adjust(m.typ, m.amount, m);
        }

        for m in mods_sec_pass {
            let mut amount = m.amount;
            for f in &m.flags {
                match f {
                    Mutation::MultiplierProperty(mutation) => {
                        amount *= self.property_int(mutation.1) / mutation.0;
                    },
                    Mutation::MultiplierStat(mutation) => {
                        amount *= match stats.get(&mutation.1) {
                            Some(stat) => stat.val() / mutation.0,
                            None => 1,
                        }
                    },
                }
            }
            stats.entry(m.stat).or_default().adjust(m.typ, amount, m);
        }

        'outer: for m in mods_third_pass {
            let mut amount = m.amount;
            for f in &m.flags {
                match f {
                    Mutation::MultiplierProperty(mutation) => {
                        amount *= self.property_int(mutation.1) / mutation.0;
                    },
                    Mutation::MultiplierStat(mutation) => {
                        amount *= match stats.get(&mutation.1) {
                            Some(stat) => stat.val() / mutation.0,
                            None => 1,
                        }
                    },
                }
            }
            for f in &m.conditions {
                match f {
                    // All the conditions are matched negatively
                    // (if they don't match, continue to outer and disregard mod)
                    Condition::GreaterEqualProperty(mutation) => {
                        if self.property_int(mutation.1) < mutation.0 {
                            continue 'outer;
                        }
                    },
                    Condition::LesserEqualProperty(mutation) => {
                        if self.property_int(mutation.1) > mutation.0 {
                            continue 'outer;
                        }
                    },
                    Condition::GreaterEqualStat(mutation) => {
                        if let Some(stat) = stats.get(&mutation.1) {
                            if stat.val() < mutation.0 {
                                continue 'outer;
                            }
                        }
                    },
                    Condition::LesserEqualStat(mutation) => {
                        if let Some(stat) = stats.get(&mutation.1) {
                            if stat.val() > mutation.0 {
                                continue 'outer;
                            }
                        } else {
                            continue 'outer;
                        }
                    },
                    Condition::PropertyBool(mutation) => {
                        if self.property_bool(mutation.1) != mutation.0 {
                            continue 'outer;
                        }
                    },
                    Condition::WhileWielding(weapons) => {
                        if !self.is_holding(weapons) {
                            continue 'outer;
                        }
                    }
                }
            }
            stats.entry(m.stat).or_default().adjust(m.typ, amount, m);
        }

        Stats { stats }
    }
}

pub fn calc_stat(stat_id: StatId, mods: &[Mod], tags: &FxHashSet<GemTag>) -> Stat {
    let mut stat = Stat::default();

    for m in mods
        .iter()
        .filter(|m| m.stat == stat_id && tags.is_superset(&m.tags))
    {
        let mut amount = m.amount;
        for f in &m.flags {
            match f {
                Mutation::MultiplierProperty(_mp) => {
                    amount *= 1
                }
                Mutation::MultiplierStat(_) => {
                    // todo
                }
            }
        }
        stat.adjust(m.typ, amount, m);
    }

    stat
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            base: 0,
            inc: 0,
            more: 100,
            mods: vec![],
        }
    }
}

impl Stat {
    pub fn adjust(&mut self, t: Type, amount: i64, m: &Mod) {
        match t {
            Type::Base => self.base += amount,
            Type::Inc => self.inc += amount,
            Type::More => self.more = (self.more * (100 + amount)) / 100,
        }
        self.mods.push(m.to_owned());
    }

    fn mult(&self) -> i64 {
        (100 + self.inc) * self.more
    }

    fn val100(&self) -> i64 {
        (self.base * self.mult()) / 100
    }

    pub fn val(&self) -> i64 {
        self.val100() / 100
    }

    pub fn val_rounded_up(&self) -> i64 {
        (self.val100() as f64 / 100.0).ceil() as i64
    }

    pub fn assimilate(&mut self, stat: &Stat) {
        self.base += stat.base;
        self.inc += stat.inc;
        self.more = (self.more * stat.more) / 100;
        self.mods.extend(stat.mods.clone());
    }

    pub fn val_custom(&self, val: i64) -> i64 {
        (val * self.mult()) / 10000
    }

    pub fn val_custom_inv(&self, val: i64) -> i64 {
        (val * 10000) / self.mult()
    }
}

#[test]
fn test_build() {
    let player = Build::new_player();
    let stats = player.calc_stats(&player.calc_mods(true), &hset![]);

    assert_eq!(stats.stat(StatId::MaximumLife).val(), 60);
}
