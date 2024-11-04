use crate::build::StatId;
/// 2 ways to parse a mod:
///
/// 1. "Automatic": make sure all parts of your mod are declared
///    in TAGS, STATS, BEGINNINGS and ENDINGS.
/// 2. (todo) "Exotic": one-shot parsing of the entire mod
///    through ONESHOTS.
///
/// All strings need to be lowercase.

use crate::gem::{Gem, GemTag};
use crate::data::ActiveSkillTypes;
use crate::item::{self, Item, ItemClass};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
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
        map
    };
}

const ENDINGS: [(&str, Mutation); 4] = [
    ("per level", Mutation::MultiplierProperty((1, PropertyInt::Level))),
    (
        "per frenzy charge",
        Mutation::MultiplierProperty((1, PropertyInt::FrenzyCharges)),
    ),
    (
        "per power charge",
        Mutation::MultiplierProperty((1, PropertyInt::PowerCharges)),
    ),
    (
        "per endurance charge",
        Mutation::MultiplierProperty((1, PropertyInt::EnduranceCharges)),
    ),
];

const ENDINGS_GEMTAGS: [(&str, GemTag); 14] = [
    ("of aura skills", GemTag::Aura),
    ("of curse skills", GemTag::Curse),
    ("of hex skills", GemTag::Hex),
    ("with attack skills", GemTag::Attack),
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

lazy_static! {
    static ref ENDINGS_CONDITIONS: [(&'static str, Condition); 11] = [
        ("while fortified", Condition::PropertyBool((true, PropertyBool::Fortified))),
        ("if you've dealt a critical strike recently", Condition::PropertyBool((true, PropertyBool::DealtCritRecently))),
        ("while leeching", Condition::PropertyBool((true, PropertyBool::Leeching))),
        ("when on full life", Condition::PropertyBool((true, PropertyBool::OnFullLife))),
        ("while on low life", Condition::PropertyBool((true, PropertyBool::OnLowLife))),
        ("while holding a shield", Condition::WhileWielding(hset![ItemClass::Shield])),
        ("while wielding a staff", Condition::WhileWielding(hset![ItemClass::Staff, ItemClass::Warstaff])),
        ("while wielding a sword", Condition::WhileWielding(hset![ItemClass::OneHandSword, ItemClass::TwoHandSword, ItemClass::ThrustingOneHandSword])),
        ("while wielding a dagger", Condition::WhileWielding(hset![ItemClass::Dagger, ItemClass::RuneDagger])),
        ("while wielding a mace or sceptre", Condition::WhileWielding(hset![ItemClass::OneHandMace, ItemClass::TwoHandMace, ItemClass::Sceptre])),
        ("while wielding a claw or dagger", Condition::WhileWielding(hset![ItemClass::Dagger, ItemClass::RuneDagger, ItemClass::Claw])),
    ];

    static ref ENDINGS_WEAPON_RESTRICTIONS: [(&'static str, FxHashSet<ItemClass>); 12] = [
        ("with axes", hset![ItemClass::OneHandAxe, ItemClass::TwoHandAxe]),
        ("with swords", hset![ItemClass::OneHandSword, ItemClass::TwoHandSword, ItemClass::ThrustingOneHandSword]),
        ("with maces", hset![ItemClass::OneHandMace, ItemClass::TwoHandMace]),
        ("with two handed melee weapons", hset![ItemClass::TwoHandSword, ItemClass::TwoHandMace, ItemClass::TwoHandAxe]),
        ("with one handed melee weapons", hset![ItemClass::OneHandSword, ItemClass::OneHandMace, ItemClass::OneHandAxe, ItemClass::ThrustingOneHandSword]),
        ("with one handed weapons", hset![ItemClass::OneHandSword, ItemClass::OneHandMace, ItemClass::OneHandAxe, ItemClass::ThrustingOneHandSword]),
        ("with staves", hset![ItemClass::Staff, ItemClass::Warstaff]),
        ("with bows", hset![ItemClass::Bow]),
        ("with claws", hset![ItemClass::Claw]),
        ("with wands", hset![ItemClass::Wand]),
        ("with daggers", hset![ItemClass::Dagger, ItemClass::RuneDagger]),
        ("with maces or sceptres", hset![ItemClass::OneHandMace, ItemClass::TwoHandMace, ItemClass::Sceptre]),
    ];

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
            regex!(r"^(minions have )?((\+|-)?[0-9]+)%? (?:to )?(?:all )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[4])?;
                let insert_minion_tag = c.get(1).is_some();
                let mut amount = i64::from_str(&c[2]).unwrap();
                if let Some(capture) = c.get(3) {
                    amount = match capture.as_str() {
                        "-" => amount.neg(),
                        _ => amount,
                    };
                }
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Base,
                        amount,
                        tags: s.1.clone(),
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
                let stat_tags_1 = STATS_MAP.get(format!("minimum {}", &c[3]).as_str()).cloned()?;
                let stat_tags_2 = STATS_MAP.get(format!("maximum {}", &c[3]).as_str()).cloned()?;
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
    static ref STATS: Vec<(&'static str, StatId, FxHashSet<GemTag>)> = vec![
        ("strength", StatId::Strength, hset!()),
        ("dexterity", StatId::Dexterity, hset!()),
        ("intelligence", StatId::Intelligence, hset!()),
        ("attributes", StatId::Attributes, hset!()),
        ("attack speed", StatId::AttackSpeed, hset!()),
        ("cast speed", StatId::CastSpeed, hset!()),
        ("warcry speed", StatId::WarcrySpeed, hset!()),
        ("cooldown recovery speed", StatId::CooldownRecoverySpeed, hset!()),
        ("projectile speed", StatId::ProjectileSpeed, hset!()),
        ("trap throwing speed", StatId::TrapThrowingSpeed, hset!()),
        ("chance to block attack damage", StatId::ChanceToBlockAttackDamage, hset!()),
        ("chance to block spell damage", StatId::ChanceToBlockSpellDamage, hset!()),
        ("chance to suppress spell damage", StatId::ChanceToSuppressSpellDamage, hset!()),
        ("fire damage over time multiplier", StatId::FireDamageOverTimeMultiplier, hset!()),
        ("cold damage over time multiplier", StatId::ColdDamageOverTimeMultiplier, hset!()),
        ("chaos damage over time multiplier", StatId::ChaosDamageOverTimeMultiplier, hset!()),
        ("physical damage over time multiplier", StatId::PhysicalDamageOverTimeMultiplier, hset!()),
        ("damage over time multiplier", StatId::DamageOverTimeMultiplier, hset!()),
        ("fire damage over time", StatId::FireDamageOverTime, hset!()),
        ("cold damage over time", StatId::ColdDamageOverTime, hset!()),
        ("chaos damage over time", StatId::ChaosDamageOverTime, hset!()),
        ("physical damage over time", StatId::PhysicalDamageOverTime, hset!()),
        ("damage over time", StatId::DamageOverTime, hset!()),
        ("minimum fire damage", StatId::MinFireDamage, hset!()),
        ("maximum fire damage", StatId::MaxFireDamage, hset!()),
        ("fire damage", StatId::FireDamage, hset!()),
        ("cold damage", StatId::ColdDamage, hset!()),
        ("lightning damage", StatId::LightningDamage, hset!()),
        ("chaos damage", StatId::ChaosDamage, hset!()),
        ("minimum physical attack damage", StatId::MinPhysicalDamage, hset!(GemTag::Attack)),
        ("maximum physical attack damage", StatId::MaxPhysicalDamage, hset!(GemTag::Attack)),
        ("minimum physical damage", StatId::MinPhysicalDamage, hset!()),
        ("maximum physical damage", StatId::MaxPhysicalDamage, hset!()),
        ("physical attack damage", StatId::PhysicalDamage, hset!(GemTag::Attack)),
        ("physical damage", StatId::PhysicalDamage, hset!()),
        ("damage", StatId::Damage, hset!()),
        ("area of effect", StatId::AreaOfEffect, hset!()),
        ("accuracy rating", StatId::AccuracyRating, hset!()),
        ("movement speed", StatId::MovementSpeed, hset!()),
        ("skill effect duration", StatId::SkillEffectDuration, hset!()),
        ("duration", StatId::Duration, hset!()),
        ("impale effect", StatId::ImpaleEffect, hset!()),
        ("minimum frenzy charges", StatId::MinimumFrenzyCharges, hset!()),
        ("minimum power charges", StatId::MinimumPowerCharges, hset!()),
        ("minimum endurance charges", StatId::MinimumEnduranceCharges, hset!()),
        ("maximum frenzy charges", StatId::MaximumFrenzyCharges, hset!()),
        ("maximum power charges", StatId::MaximumPowerCharges, hset!()),
        ("maximum endurance charges", StatId::MaximumEnduranceCharges, hset!()),
        ("maximum life", StatId::MaximumLife, hset!()),
        ("maximum mana", StatId::MaximumMana, hset!()),
        ("minimum rage", StatId::MinimumRage, hset!()),
        ("maximum rage", StatId::MaximumRage, hset!()),
        ("maximum energy shield", StatId::MaximumEnergyShield, hset!()),
        ("energy shield recharge rate", StatId::EnergyShieldRechargeRate, hset!()),
        ("life regeneration rate", StatId::LifeRegenerationRate, hset!()),
        ("mana regeneration rate", StatId::ManaRegenerationRate, hset!()),
        ("mana reservation efficiency", StatId::ManaReservationEfficiency, hset!()),
        ("critical strike chance", StatId::CriticalStrikeChance, hset!()),
        ("critical strike multiplier", StatId::CriticalStrikeMultiplier, hset!()),
        ("armour", StatId::Armour, hset!()),
        ("evasion rating", StatId::EvasionRating, hset!()),
        ("stun threshold", StatId::StunThreshold, hset!()),
        ("chance to avoid being stunned", StatId::ChanceToAvoidBeingStunned, hset!()),
        ("maximum fire resistance", StatId::MaximumFireResistance, hset!()),
        ("maximum cold resistance", StatId::MaximumColdResistance, hset!()),
        ("maximum lightning resistance", StatId::MaximumLightningResistance, hset!()),
        ("maximum chaos resistance", StatId::MaximumChaosResistance, hset!()),
        ("fire resistance", StatId::FireResistance, hset!()),
        ("cold resistance", StatId::ColdResistance, hset!()),
        ("lightning resistance", StatId::LightningResistance, hset!()),
        ("chaos resistance", StatId::ChaosResistance, hset!()),
        ("flask charges gained", StatId::FlaskChargesGained, hset!()),
        ("flask effect duration", StatId::FlaskEffectDuration, hset!()),
        ("flask recovery rate", StatId::FlaskRecoveryRate, hset!()),
        ("flask charges used", StatId::FlaskChargesUsed, hset!()),
        ("mana cost", StatId::ManaCost, hset!()),
        ("life cost", StatId::LifeCost, hset!()),
        ("cost", StatId::Cost, hset!()),
        ("passive skill points", StatId::PassiveSkillPoints, hset!()),
        ("passive skill point", StatId::PassiveSkillPoints, hset!()),
    ];

    static ref STATS_MAP: FxHashMap<&'static str, (StatId, FxHashSet<GemTag>)> = {
        let mut map = FxHashMap::default();
        for entry in STATS.iter() {
            map.insert(entry.0, (entry.1, entry.2.clone()));
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
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum PropertyInt {
    Level,
    PowerCharges,
    FrenzyCharges,
    EnduranceCharges,
    Rage,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum PropertyBool {
    Blinded,
    Onslaught,
    Fortified,
    DealtCritRecently,
    Leeching,
    OnFullLife,
    OnLowLife,
}

pub enum Ending {
    Mutation(Mutation),
    Tag(GemTag),
    Weapon(FxHashSet<ItemClass>),
    Condition(Condition),
}

#[derive(Debug, Clone)]
pub enum Mutation {
    MultiplierStat((i64, StatId)),
    MultiplierProperty((i64, PropertyInt)),
}

#[derive(Debug, Clone)]
pub enum Condition {
    GreaterEqualProperty((i64, PropertyInt)),
    GreaterEqualStat((i64, StatId)),
    LesserEqualProperty((i64, PropertyInt)),
    LesserEqualStat((i64, StatId)),
    PropertyBool((bool, PropertyBool)),
    WhileWielding(FxHashSet<ItemClass>),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    #[default]
    Innate,
    Node(u16),
    Mastery((u16, u16)),
    Item,
    Gem,
}

#[derive(Default, Debug, Clone)]
pub struct Mod {
    pub stat: StatId,
    pub typ: Type,
    pub amount: i64,
    pub flags: Vec<Mutation>,
    pub conditions: Vec<Condition>,
    pub tags: FxHashSet<GemTag>,
    pub source: Source,
    pub weapons: FxHashSet<ItemClass>
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

fn parse_stat_nomulti(input: &str) -> Option<(StatId, FxHashSet<GemTag>)> {
    let mut tags = hset![];

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
        } else {
            return None;
        }
    }

    Some((stat.1, tags))
}

/// Attempts to parse a chunk like "melee physical damage"
fn parse_stat(input: &str) -> Option<Vec<(StatId, FxHashSet<GemTag>)>> {
    if let Some(stats) = MULTISTATS.get(input) {
        return Some(stats.iter().map(|id| (*id, hset![])).collect());
    }

    if let Some(stat) = parse_stat_nomulti(input) {
        return Some(vec![stat]);
    }

    None
}

lazy_static! {
    static ref CACHE: Mutex<FxHashMap<String, Option<Vec<Mod>>>> = Mutex::new(FxHashMap::default());
}

/// Attempts to parse a modifier like "30℅ increased poison damage while focussed"
/// 1. todo: try to match the entire string against SPECIALS
/// 2. if not special, parse right to left:
///    2.1. any amount of ENDINGS
///    2.2. a BEGINNING
pub fn parse_mod(input: &str, source: Source) -> Option<Vec<Mod>> {
    if let Some(mods_opt) = CACHE.lock().unwrap().get(input) {
        match mods_opt {
            Some(mods) => return Some(mods.to_owned()),
            None => return None,
        }
    }

    let mut m = &input[0..];
    let mut flags = vec![];
    let mut tags = hset![];
    let mut weapons = hset![];
    let mut conditions = vec![];

    while let Some(ending) = parse_ending(&m.to_lowercase()) {
        m = &m[0..m.len() - ending.0 - 1];
        match ending.1 {
            Ending::Mutation(flag) => {
                flags.push(flag);
            }
            Ending::Tag(tag) => {
                tags.insert(tag);
            }
            Ending::Weapon(weapon) => {
                weapons.extend(weapon);
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
                    modifier.tags.extend(tags.clone());
                    modifier.flags.extend(flags.clone());
                    modifier.weapons.extend(weapons.clone());
                    modifier.conditions.extend(conditions.clone());
                    modifier.source = source;
                }
                CACHE.lock().unwrap().insert(input.to_string(), Some(mods.clone()));
                return Some(mods);
            }
        }
    }

    CACHE.lock().unwrap().insert(input.to_string(), None);
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
    use crate::tree::NodeType;

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
