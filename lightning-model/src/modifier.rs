use crate::build::stat::StatId;
use crate::build::{property, Slot};
use crate::data::base_item::ItemClass;
use crate::data::gem::GemTag;
use crate::gem::Gem;
use crate::data::ActiveSkillTypes;
use crate::item::{self, Item};
use crate::stackvec::StackVec;
use enumflags2::{make_bitflags as flags, BitFlags};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use rustc_hash::FxHashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ops::Neg;
use std::str::FromStr;
use std::sync::Mutex;

lazy_static! {
    // Currently limited to one word,
    // need to change parse_stat otherwise.
    static ref TAGS: FxHashMap<&'static str, GemTag> = {
        let mut map = FxHashMap::default();
        map.insert("spell", GemTag::Spell);
        map.insert("melee", GemTag::Melee);
        map.insert("attack", GemTag::Attack);
        map.insert("projectile", GemTag::Projectile);
        map.insert("brand", GemTag::Brand);
        map.insert("mine", GemTag::Mine);
        map.insert("trap", GemTag::Trap);
        map.insert("curse", GemTag::Curse);
        map.insert("minion", GemTag::Minion);
        map.insert("totem", GemTag::Totem);
        map.insert("area", GemTag::Area);
        map
    };
}

const ENDINGS: [(&str, Mutation); 4] = [
    ("per level", Mutation::MultiplierProperty((1, property::Int::Level))),
    ("per frenzy charge", Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))),
    ("per power charge", Mutation::MultiplierProperty((1, property::Int::PowerCharges))),
    ("per endurance charge", Mutation::MultiplierProperty((1, property::Int::EnduranceCharges))),
];

const ENDINGS_GEMTAGS: [(&str, GemTag); 15] = [
    ("of aura skills", GemTag::Aura),
    ("of curse skills", GemTag::Curse),
    ("of hex skills", GemTag::Hex),
    ("with attack skills", GemTag::Attack),
    ("to attacks", GemTag::Attack),
    ("of attacks", GemTag::Attack),
    ("of skills", GemTag::Active_Skill),
    ("with mines", GemTag::Mine),
    ("with traps", GemTag::Trap),
    ("with bow skills", GemTag::Bow),
    ("with totem skills", GemTag::Totem),
    ("for spell damage", GemTag::Spell),
    ("with cold skills", GemTag::Cold),
    ("with fire skills", GemTag::Fire),
    ("with lightning skills", GemTag::Lightning),
];

// Parses a string like '1.75' into i64 '175'
fn parse_val100(val: &str) -> Option<i64> {
    let dec = Decimal::from_str(val).ok()?;
    match dec.scale() {
        0 => Some((dec.mantissa() * 100) as i64),
        1 => Some((dec.mantissa() * 10) as i64),
        2 => Some(dec.mantissa() as i64),
        _ => None,
    }
}

const ENDINGS_WEAPON_RESTRICTIONS: &[(&'static str, BitFlags<ItemClass>)] = &[
    ("with axes", flags!(ItemClass::{OneHandAxe | TwoHandAxe})),
    ("with swords", flags!(ItemClass::{OneHandSword | TwoHandSword | ThrustingOneHandSword})),
    ("with maces", flags!(ItemClass::{OneHandMace | TwoHandMace})),
    ("with two handed melee weapons", flags!(ItemClass::{TwoHandSword | TwoHandMace | TwoHandAxe | Warstaff | Staff})),
    ("with one handed melee weapons", flags!(ItemClass::{OneHandSword | OneHandMace | OneHandAxe | ThrustingOneHandSword})),
    ("with one handed weapons", flags!(ItemClass::{OneHandSword | OneHandMace | OneHandAxe | ThrustingOneHandSword})),
    ("with staves", flags!(ItemClass::{Staff | Warstaff})),
    ("with bows", flags!(ItemClass::Bow)),
    ("with claws", flags!(ItemClass::Claw)),
    ("with wands", flags!(ItemClass::Wand)),
    ("with daggers", flags!(ItemClass::{Dagger | RuneDagger})),
    ("with maces or sceptres", flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre})),
];

const ENDINGS_CONDITIONS: &[(&'static str, Condition)] = &[
    ("while fortified", Condition::PropertyBool((true, property::Bool::Fortified))),
    ("if you've dealt a critical strike recently", Condition::PropertyBool((true, property::Bool::DealtCritRecently))),
    ("while leeching", Condition::PropertyBool((true, property::Bool::Leeching))),
    ("when on full life", Condition::PropertyBool((true, property::Bool::OnFullLife))),
    ("while on low life", Condition::PropertyBool((true, property::Bool::OnLowLife))),
    ("while holding a shield", Condition::WhileWielding(flags!(ItemClass::Shield))),
    ("while wielding a staff", Condition::WhileWielding(flags!(ItemClass::{Staff | Warstaff}))),
    ("while wielding a sword", Condition::WhileWielding(flags!(ItemClass::{OneHandSword | TwoHandSword | ThrustingOneHandSword}))),
    ("while wielding a dagger", Condition::WhileWielding(flags!(ItemClass::{Dagger | RuneDagger}))),
    ("while wielding a mace or sceptre", Condition::WhileWielding(flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre}))),
    ("while wielding a claw or dagger", Condition::WhileWielding(flags!(ItemClass::{Dagger | RuneDagger | Claw}))),
];

lazy_static! {
    static ref BEGINNINGS: Vec<(Regex, Box<dyn Fn(&Captures) -> Option<Vec<Mod>> + Send + Sync>)> = vec![
        (
            regex!(r"^(minions (?:have|deal) )?([0-9]+)% (increased|reduced) ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[4])?;
                let insert_minion_tag = c.get(1).is_some();
                let mut amount = i64::from_str(&c[2]).unwrap();
                amount = match &c[3] {
                    "reduced" => amount.neg(),
                    "increased" => amount,
                    _ => panic!(),
                };
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Inc,
                        amount,
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect())
            })
        ), (
            regex!(r"^(minions (?:have|deal) )?([0-9]+)% (increased|reduced) ([a-z ]+) and ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat(&c[4])?;
                let stat_tags_2 = parse_stat(&c[5])?;
                let insert_minion_tag = c.get(1).is_some();
                let mut amount = i64::from_str(&c[2]).unwrap();
                amount = match &c[3] {
                    "reduced" => amount.neg(),
                    "increased" => amount,
                    _ => panic!(),
                };
                let mut ret: Vec<Mod> = stat_tags_1.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Inc,
                        amount,
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect();
                ret.extend(stat_tags_2.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Inc,
                        amount,
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }));
                Some(ret)
            })
        ), (
            regex!(r"^(minions have )?([+-]?[0-9]+)%? (?:to )?(?:all )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                let insert_minion_tag = c.get(1).is_some();
                let amount = i64::from_str(&c[2]).unwrap();
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Base,
                        amount,
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% more ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0,
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap(),
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% less ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0,
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: s.1.clone(),
                        global: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z ]+) and ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[2])?;
                let stat_tags_2 = parse_stat_nomulti(&c[3])?;
                Some(vec![Mod {
                    stat: stat_tags_1.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_1.1,
                    global: stat_tags_1.2,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_2.1,
                    global: stat_tags_2.2,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z]+) and ([a-z]+) resistances$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("{} resistance", &c[2]).as_str()).cloned()?;
                let stat_tags_2 = STATS_MAP.get(format!("{} resistance", &c[3]).as_str()).cloned()?;
                Some(vec![Mod {
                    stat: stat_tags_1.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_1.1,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_2.1,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^adds ([0-9]+) to ([0-9]+) ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("added minimum {}", &c[3]).as_str()).cloned()?;
                let stat_tags_2 = STATS_MAP.get(format!("added maximum {}", &c[3]).as_str()).cloned()?;
                Some(vec![Mod {
                    stat: stat_tags_1.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_1.1,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[2]).unwrap(),
                    tags: stat_tags_2.1,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^regenerate ([0-9]+) life per second$"),
            Box::new(|c| {
                Some(vec![Mod {
                    stat: StatId::LifeRegeneration,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^regenerate ([0-9.]+)% of life per second$"),
            Box::new(|c| {
                Some(vec![Mod {
                    stat: StatId::LifeRegenerationPct,
                    typ: Type::Base,
                    amount: parse_val100(&c[1])?,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^damage penetrates ([0-9]+)% ([a-z]+) resistance$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("{} damage penetration", &c[2]).as_str()).cloned()?;
                Some(vec![Mod {
                    stat: stat_tags_1.0,
                    typ: Type::Base,
                    amount: parse_val100(&c[1])?,
                    tags: stat_tags_1.1,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^grants ([0-9]+) ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[2])?;
                Some(vec![Mod {
                    stat: stat_tags_1.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    global: stat_tags_1.2,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^your hits can't be evaded$"),
            Box::new(|_| {
                Some(vec![Mod {
                    stat: StatId::ChanceToHit,
                    typ: Type::Override,
                    amount: 100,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^never deal critical strikes$"),
            Box::new(|_| {
                Some(vec![Mod {
                    stat: StatId::CriticalStrikeChance,
                    typ: Type::Override,
                    amount: 0,
                    ..Default::default()
                }])
            })
        ),
    ];

    static ref MULTISTATS: FxHashMap<&'static str, Vec<StatId>> = {
        let mut map = FxHashMap::default();
        map.insert("attributes", vec![StatId::Strength, StatId::Dexterity, StatId::Intelligence]);
        map.insert("maximum elemental resistances", vec![StatId::MaximumFireResistance, StatId::MaximumColdResistance, StatId::MaximumLightningResistance]);
        map.insert("elemental resistances", vec![StatId::FireResistance, StatId::ColdResistance, StatId::LightningResistance]);
        map.insert("maximum resistances", vec![StatId::MaximumFireResistance, StatId::MaximumColdResistance, StatId::MaximumLightningResistance, StatId::MaximumChaosResistance]);
        map.insert("resistances", vec![StatId::FireResistance, StatId::ColdResistance, StatId::LightningResistance, StatId::ChaosResistance]);
        map.insert("elemental damage", vec![StatId::FireDamage, StatId::ColdDamage, StatId::LightningDamage]);
        map
    };

    // Order is important for overlapping stats
    // like "area of effect" and "effect"
    static ref STATS: Vec<(&'static str, StatId, BitFlags<GemTag>)> = vec![
        ("strength", StatId::Strength, BitFlags::empty()),
        ("dexterity", StatId::Dexterity, BitFlags::empty()),
        ("intelligence", StatId::Intelligence, BitFlags::empty()),
        ("attributes", StatId::Attributes, BitFlags::empty()),
        ("attack speed", StatId::AttackSpeed, BitFlags::empty()),
        ("cast speed", StatId::CastSpeed, BitFlags::empty()),
        ("warcry speed", StatId::WarcrySpeed, BitFlags::empty()),
        ("cooldown recovery speed", StatId::CooldownRecoverySpeed, BitFlags::empty()),
        ("projectile speed", StatId::ProjectileSpeed, BitFlags::empty()),
        ("trap throwing speed", StatId::TrapThrowingSpeed, BitFlags::empty()),
        ("chance to block attack damage", StatId::ChanceToBlockAttackDamage, BitFlags::empty()),
        ("chance to block spell damage", StatId::ChanceToBlockSpellDamage, BitFlags::empty()),
        ("chance to suppress spell damage", StatId::ChanceToSuppressSpellDamage, BitFlags::empty()),
        ("fire damage over time multiplier", StatId::FireDamageOverTimeMultiplier, BitFlags::empty()),
        ("cold damage over time multiplier", StatId::ColdDamageOverTimeMultiplier, BitFlags::empty()),
        ("chaos damage over time multiplier", StatId::ChaosDamageOverTimeMultiplier, BitFlags::empty()),
        ("physical damage over time multiplier", StatId::PhysicalDamageOverTimeMultiplier, BitFlags::empty()),
        ("damage over time multiplier", StatId::DamageOverTimeMultiplier, BitFlags::empty()),
        ("fire damage penetration", StatId::FireDamagePen, BitFlags::empty()),
        ("lightning damage penetration", StatId::LightningDamagePen, BitFlags::empty()),
        ("cold damage penetration", StatId::ColdDamagePen, BitFlags::empty()),
        ("chaos damage penetration", StatId::ChaosDamagePen, BitFlags::empty()),
        ("fire damage over time", StatId::FireDamageOverTime, BitFlags::empty()),
        ("cold damage over time", StatId::ColdDamageOverTime, BitFlags::empty()),
        ("chaos damage over time", StatId::ChaosDamageOverTime, BitFlags::empty()),
        ("physical damage over time", StatId::PhysicalDamageOverTime, BitFlags::empty()),
        ("damage over time", StatId::DamageOverTime, BitFlags::empty()),
        ("fire damage", StatId::FireDamage, BitFlags::empty()),
        ("cold damage", StatId::ColdDamage, BitFlags::empty()),
        ("lightning damage", StatId::LightningDamage, BitFlags::empty()),
        ("chaos damage", StatId::ChaosDamage, BitFlags::empty()),
        ("minimum physical attack damage", StatId::MinPhysicalDamage, GemTag::Attack.into()),
        ("maximum physical attack damage", StatId::MaxPhysicalDamage, GemTag::Attack.into()),
        ("added minimum physical damage", StatId::AddedMinPhysicalDamage, BitFlags::empty()),
        ("added maximum physical damage", StatId::AddedMaxPhysicalDamage, BitFlags::empty()),
        ("physical attack damage", StatId::PhysicalDamage, GemTag::Attack.into()),
        ("physical damage", StatId::PhysicalDamage, BitFlags::empty()),
        ("damage", StatId::Damage, BitFlags::empty()),
        ("area of effect", StatId::AreaOfEffect, BitFlags::empty()),
        ("accuracy rating", StatId::AccuracyRating, BitFlags::empty()),
        ("movement speed", StatId::MovementSpeed, BitFlags::empty()),
        ("skill effect duration", StatId::SkillEffectDuration, BitFlags::empty()),
        ("duration", StatId::Duration, BitFlags::empty()),
        ("impale effect", StatId::ImpaleEffect, BitFlags::empty()),
        ("minimum frenzy charges", StatId::MinimumFrenzyCharges, BitFlags::empty()),
        ("minimum power charges", StatId::MinimumPowerCharges, BitFlags::empty()),
        ("minimum endurance charges", StatId::MinimumEnduranceCharges, BitFlags::empty()),
        ("maximum frenzy charges", StatId::MaximumFrenzyCharges, BitFlags::empty()),
        ("maximum power charges", StatId::MaximumPowerCharges, BitFlags::empty()),
        ("maximum endurance charges", StatId::MaximumEnduranceCharges, BitFlags::empty()),
        ("maximum life", StatId::MaximumLife, BitFlags::empty()),
        ("maximum mana", StatId::MaximumMana, BitFlags::empty()),
        ("minimum rage", StatId::MinimumRage, BitFlags::empty()),
        ("maximum rage", StatId::MaximumRage, BitFlags::empty()),
        ("maximum energy shield", StatId::MaximumEnergyShield, BitFlags::empty()),
        ("energy shield recharge rate", StatId::EnergyShieldRechargeRate, BitFlags::empty()),
        ("energy shield", StatId::EnergyShield, BitFlags::empty()),
        ("life regeneration rate", StatId::LifeRegenerationRate, BitFlags::empty()),
        ("mana regeneration rate", StatId::ManaRegenerationRate, BitFlags::empty()),
        ("mana reservation efficiency", StatId::ManaReservationEfficiency, BitFlags::empty()),
        ("critical strike chance", StatId::CriticalStrikeChance, BitFlags::empty()),
        ("critical strike multiplier", StatId::CriticalStrikeMultiplier, BitFlags::empty()),
        ("armour", StatId::Armour, BitFlags::empty()),
        ("evasion rating", StatId::EvasionRating, BitFlags::empty()),
        ("stun threshold", StatId::StunThreshold, BitFlags::empty()),
        ("chance to avoid being stunned", StatId::ChanceToAvoidBeingStunned, BitFlags::empty()),
        ("maximum fire resistance", StatId::MaximumFireResistance, BitFlags::empty()),
        ("maximum cold resistance", StatId::MaximumColdResistance, BitFlags::empty()),
        ("maximum lightning resistance", StatId::MaximumLightningResistance, BitFlags::empty()),
        ("maximum chaos resistance", StatId::MaximumChaosResistance, BitFlags::empty()),
        ("fire resistance", StatId::FireResistance, BitFlags::empty()),
        ("cold resistance", StatId::ColdResistance, BitFlags::empty()),
        ("lightning resistance", StatId::LightningResistance, BitFlags::empty()),
        ("chaos resistance", StatId::ChaosResistance, BitFlags::empty()),
        ("flask charges gained", StatId::FlaskChargesGained, BitFlags::empty()),
        ("flask effect duration", StatId::FlaskEffectDuration, BitFlags::empty()),
        ("flask recovery rate", StatId::FlaskRecoveryRate, BitFlags::empty()),
        ("flask charges used", StatId::FlaskChargesUsed, BitFlags::empty()),
        ("mana cost", StatId::ManaCost, BitFlags::empty()),
        ("life cost", StatId::LifeCost, BitFlags::empty()),
        ("cost", StatId::Cost, BitFlags::empty()),
        ("passive skill points", StatId::PassiveSkillPoints, BitFlags::empty()),
        ("passive skill point", StatId::PassiveSkillPoints, BitFlags::empty()),
    ];

    static ref STATS_MAP: FxHashMap<&'static str, (StatId, BitFlags<GemTag>)> = {
        let mut map = FxHashMap::default();
        for entry in STATS.iter() {
            map.insert(entry.0, (entry.1, entry.2));
        }
        map
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Type {
    #[default]
    Base,
    Inc,
    More,
    Override,
}

pub enum Ending {
    Mutation(Mutation),
    Tag(GemTag),
    Weapon(BitFlags<ItemClass>),
    Condition(Condition),
}

#[derive(Debug, Clone, Copy)]
pub enum Mutation {
    MultiplierStat((i64, StatId)),
    MultiplierProperty((i64, property::Int)),
}

#[derive(Debug, Clone, Copy)]
pub enum Condition {
    GreaterEqualProperty((i64, property::Int)),
    GreaterEqualStat((i64, StatId)),
    LesserEqualProperty((i64, property::Int)),
    LesserEqualStat((i64, StatId)),
    PropertyBool((bool, property::Bool)),
    WhileWielding(BitFlags<ItemClass>),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    #[default]
    Innate,
    Node(u16),
    Mastery((u16, u16)),
    Item(Slot),
    Gem,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Mod {
    pub stat: StatId,
    pub typ: Type,
    pub amount: i64,
    pub mutations: StackVec<Mutation, 5>,
    pub conditions: StackVec<Condition, 5>,
    pub tags: BitFlags<GemTag>,
    pub source: Source,
    pub weapons: BitFlags<ItemClass>,
    pub global: bool,
}

impl Mod {
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

fn parse_ending(m: &str) -> Option<(usize, Ending)> {
    for ending in ENDINGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Mutation(ending.1.clone())));
        }
    }
    for ending in ENDINGS_GEMTAGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Tag(ending.1)));
        }
    }
    for ending in ENDINGS_WEAPON_RESTRICTIONS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Weapon(ending.1.clone())));
        }
    }
    for ending in ENDINGS_CONDITIONS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Condition(ending.1.clone())));
        }
    }

    None
}

fn parse_stat_nomulti(input: &str) -> Option<(StatId, BitFlags<GemTag>, bool)> {
    let mut tags = BitFlags::empty();
    let mut global = false;

    let stat = STATS.iter().find(|s| {
        if input.ends_with(s.0) {
            return true;
        }
        false
    })?;

    let remainder = &input[0..input.len() - stat.0.len()];

    for chunk in remainder.split_terminator(' ') {
        if let Some(t) = TAGS.get(chunk) {
            tags.insert(*t);
        } else if chunk == "global" {
            global = true;
        } else {
            return None;
        }
    }

    Some((stat.1, tags, global))
}

/// Attempts to parse a chunk like "melee physical damage"
fn parse_stat(input: &str) -> Option<Vec<(StatId, BitFlags<GemTag>, bool)>> {
    if let Some(stats) = MULTISTATS.get(input) {
        return Some(stats.iter().map(|id| (*id, BitFlags::empty(), false)).collect());
    }

    if let Some(stat) = parse_stat_nomulti(input) {
        return Some(vec![stat]);
    }

    None
}

lazy_static! {
    pub static ref CACHE: Mutex<FxHashMap<String, Option<Vec<Mod>>>> = Mutex::new(FxHashMap::default());
}

/// Attempts to parse a modifier like "30℅ increased poison damage while focussed"
/// 1. todo: try to match the entire string against SPECIALS
/// 2. if not special, parse right to left:
///    2.1. any amount of ENDINGS
///    2.2. a BEGINNING
pub fn parse_mod(input: &str, source: Source) -> Option<Vec<Mod>> {
    let mut cache = CACHE.lock().expect("Unable to lock CACHE");

    if let Some(mods_opt) = cache.get(input) {
        return mods_opt.to_owned();
    }

    let mut m = &input[0..];
    let mut mutations: StackVec<Mutation, 5> = Default::default();
    let mut tags = BitFlags::empty();
    let mut weapons = BitFlags::empty();
    let mut conditions: StackVec<Condition, 5> = Default::default();

    while let Some(ending) = parse_ending(&m.to_lowercase()) {
        m = &m[0..m.len() - ending.0 - 1];
        match ending.1 {
            Ending::Mutation(mutation) => {
                mutations.push(mutation);
            }
            Ending::Tag(tag) => {
                tags.insert(tag);
            }
            Ending::Weapon(weapon) => {
                weapons.insert(weapon);
            },
            Ending::Condition(condition) => {
                conditions.push(condition);
            },
        }
    }

    for begin in BEGINNINGS.iter() {
        if let Some(cap) = begin.0.captures(&m.to_lowercase()) {
            if let Some(mut mods) = begin.1(&cap) {
                for modifier in &mut mods {
                    modifier.tags.insert(tags);
                    modifier.mutations.extend_from_slice(&mutations);
                    modifier.weapons.insert(weapons);
                    modifier.conditions.extend_from_slice(&conditions);
                    modifier.source = source;
                }
                cache.insert(input.to_string(), Some(mods.clone()));
                return Some(mods);
            }
        }
    }

    cache.insert(input.to_string(), None);
    None
}

#[test]
fn test_parse() {
    assert!(parse_mod("50% increased damage", Source::Innate).is_some());
    assert!(parse_mod("50% reduced damage", Source::Innate).is_some());
    assert!(parse_mod("50% more damage", Source::Innate).is_some());
    assert!(parse_mod("50% less damage", Source::Innate).is_some());
    assert!(parse_mod("+5 damage", Source::Innate).is_some());
    assert!(parse_mod("-5 damage", Source::Innate).is_some());
    assert!(parse_mod("+1 maximum life per level", Source::Innate).is_some());
    assert!(parse_mod("+5% to cold resistance", Source::Innate).is_some());
    assert!(parse_mod("-5% fire resistance", Source::Innate).is_some());
    assert!(parse_mod("50% increased melee physical damage", Source::Innate).is_some());
    assert!(parse_mod("50% increased melee physical damage per level", Source::Innate).is_some());
}

#[test]
fn count_tree_parses() {
    use crate::data::tree::NodeType;

    let mut nb_mods = 0;
    let mut nb_mods_success = 0;

    let mut func = |stat| {
        nb_mods += 1;
        if parse_mod(stat, Source::Innate).is_some() {
            nb_mods_success += 1;
        }
    };

    for node in crate::data::TREE.nodes.values() {
        match node.node_type() {
            NodeType::Mastery => {
                for effect in &node.mastery_effects {
                    for stat in &effect.stats {
                        func(stat);
                    }
                }
            }
            _ => {
                for stat in &node.stats {
                    func(stat);
                }
            }
        }
    }
    println!("Tree mods parsed: {}/{}", nb_mods_success, nb_mods);
}
