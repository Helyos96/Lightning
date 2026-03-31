use crate::build::stat::StatId;
use crate::build::{property, Slot};
use crate::data::base_item::ItemClass;
use crate::data::gem::GemTag;
use crate::gem::Gem;
use crate::data::TREE;
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

const ENDINGS_GEMTAGS: &[(&str, GemTag)] = &[
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
    ("with brand skills", GemTag::Brand),
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

const ENDINGS_WEAPON_RESTRICTIONS: &[(&str, BitFlags<ItemClass>)] = &[
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

const ENDINGS_CONDITIONS: &[(&str, Condition)] = &[
    ("while fortified", Condition::PropertyBool((true, property::Bool::Fortified))),
    ("if you've dealt a critical strike recently", Condition::PropertyBool((true, property::Bool::DealtCritRecently))),
    ("while leeching", Condition::PropertyBool((true, property::Bool::Leeching))),
    ("when on full life", Condition::PropertyBool((true, property::Bool::OnFullLife))),
    ("while on full life", Condition::PropertyBool((true, property::Bool::OnFullLife))),
    ("while on low life", Condition::PropertyBool((true, property::Bool::OnLowLife))),
    ("while holding a shield", Condition::WhileWielding(flags!(ItemClass::Shield))),
    ("while wielding a wand", Condition::WhileWielding(flags!(ItemClass::Wand))),
    ("while wielding a staff", Condition::WhileWielding(flags!(ItemClass::{Staff | Warstaff}))),
    ("while wielding a sword", Condition::WhileWielding(flags!(ItemClass::{OneHandSword | TwoHandSword | ThrustingOneHandSword}))),
    ("while wielding a dagger", Condition::WhileWielding(flags!(ItemClass::{Dagger | RuneDagger}))),
    ("while wielding a mace or sceptre", Condition::WhileWielding(flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre}))),
    ("while wielding a claw or dagger", Condition::WhileWielding(flags!(ItemClass::{Dagger | RuneDagger | Claw}))),
];

// Order is important for overlapping stats
// like "area of effect" and "effect"
const STATS: &[(&'static str, StatId, BitFlags<GemTag>, BitFlags<ItemClass>)] = &[
    ("strength", StatId::Strength, BitFlags::EMPTY, BitFlags::EMPTY),
    ("dexterity", StatId::Dexterity, BitFlags::EMPTY, BitFlags::EMPTY),
    ("intelligence", StatId::Intelligence, BitFlags::EMPTY, BitFlags::EMPTY),
    ("attributes", StatId::Attributes, BitFlags::EMPTY, BitFlags::EMPTY),
    ("action speed", StatId::ActionSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("attack speed", StatId::AttackSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cast speed", StatId::CastSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("warcry speed", StatId::WarcrySpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cooldown recovery speed", StatId::CooldownRecoverySpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("projectile speed", StatId::ProjectileSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("trap throwing speed", StatId::TrapThrowingSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block attack damage", StatId::ChanceToBlockAttackDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block spell damage", StatId::ChanceToBlockSpellDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block", StatId::ChanceToBlockAttackDamage, BitFlags::EMPTY, BitFlags::EMPTY), // local on shields
    ("chance to suppress spell damage", StatId::ChanceToSuppressSpellDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to deal double damage", StatId::ChanceToDealDoubleDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage over time multiplier", StatId::FireDamageOverTimeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage over time multiplier", StatId::ColdDamageOverTimeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage over time multiplier", StatId::ChaosDamageOverTimeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage over time multiplier", StatId::PhysicalDamageOverTimeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("damage over time multiplier", StatId::DamageOverTimeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage penetration", StatId::FireDamagePen, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning damage penetration", StatId::LightningDamagePen, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage penetration", StatId::ColdDamagePen, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage penetration", StatId::ChaosDamagePen, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage over time", StatId::FireDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage over time", StatId::ColdDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage over time", StatId::ChaosDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage over time", StatId::PhysicalDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY),
    ("damage over time", StatId::DamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage reduction", StatId::PhysicalDamageReduction, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage", StatId::FireDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage", StatId::ColdDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning damage", StatId::LightningDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage", StatId::ChaosDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum physical attack damage", StatId::MinPhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY),
    ("maximum physical attack damage", StatId::MaxPhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY),
    ("added minimum physical damage", StatId::AddedMinPhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("added maximum physical damage", StatId::AddedMaxPhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical attack damage", StatId::PhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY),
    ("physical damage", StatId::PhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("wand damage", StatId::Damage, BitFlags::EMPTY, flags!(ItemClass::Wand)),
    ("damage", StatId::Damage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("area of effect", StatId::AreaOfEffect, BitFlags::EMPTY, BitFlags::EMPTY),
    ("accuracy rating", StatId::AccuracyRating, BitFlags::EMPTY, BitFlags::EMPTY),
    ("movement speed", StatId::MovementSpeed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("skill effect duration", StatId::SkillEffectDuration, BitFlags::EMPTY, BitFlags::EMPTY),
    ("duration", StatId::Duration, BitFlags::EMPTY, BitFlags::EMPTY),
    ("impale effect", StatId::ImpaleEffect, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum frenzy charges", StatId::MinimumFrenzyCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum power charges", StatId::MinimumPowerCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum endurance charges", StatId::MinimumEnduranceCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum frenzy charges", StatId::MaximumFrenzyCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum power charges", StatId::MaximumPowerCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum endurance charges", StatId::MaximumEnduranceCharges, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum life", StatId::MaximumLife, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum mana", StatId::MaximumMana, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum rage", StatId::MinimumRage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum rage", StatId::MaximumRage, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum energy shield", StatId::MaximumEnergyShield, BitFlags::EMPTY, BitFlags::EMPTY),
    ("energy shield recharge rate", StatId::EnergyShieldRechargeRate, BitFlags::EMPTY, BitFlags::EMPTY),
    ("energy shield", StatId::EnergyShield, BitFlags::EMPTY, BitFlags::EMPTY),
    ("life regeneration rate", StatId::LifeRegenerationRate, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana regeneration rate", StatId::ManaRegenerationRate, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana reservation efficiency", StatId::ManaReservationEfficiency, BitFlags::EMPTY, BitFlags::EMPTY),
    ("critical strike chance", StatId::CriticalStrikeChance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("critical strike multiplier", StatId::CriticalStrikeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY),
    ("armour", StatId::Armour, BitFlags::EMPTY, BitFlags::EMPTY),
    ("evasion rating", StatId::EvasionRating, BitFlags::EMPTY, BitFlags::EMPTY),
    ("stun threshold", StatId::StunThreshold, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to avoid being stunned", StatId::ChanceToAvoidBeingStunned, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum fire resistance", StatId::MaximumFireResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum cold resistance", StatId::MaximumColdResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum lightning resistance", StatId::MaximumLightningResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum chaos resistance", StatId::MaximumChaosResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire resistance", StatId::FireResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold resistance", StatId::ColdResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning resistance", StatId::LightningResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos resistance", StatId::ChaosResistance, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask charges gained", StatId::FlaskChargesGained, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask effect duration", StatId::FlaskEffectDuration, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask recovery rate", StatId::FlaskRecoveryRate, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask charges used", StatId::FlaskChargesUsed, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana cost", StatId::ManaCost, BitFlags::EMPTY, BitFlags::EMPTY),
    ("life cost", StatId::LifeCost, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cost", StatId::Cost, BitFlags::EMPTY, BitFlags::EMPTY),
    ("passive skill points", StatId::PassiveSkillPoints, BitFlags::EMPTY, BitFlags::EMPTY),
    ("passive skill point", StatId::PassiveSkillPoints, BitFlags::EMPTY, BitFlags::EMPTY),
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
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect())
            })
        ), (
            regex!(r"^(minions have )?([+-]?[0-9]+)%? (?:additional )?(?:to )?(?:all )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                let insert_minion_tag = c.get(1).is_some();
                let amount = i64::from_str(&c[2]).unwrap();
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0,
                        typ: Type::Base,
                        amount,
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
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
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
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
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
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
            regex!(r"^([0-9]+)% more ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0,
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap(),
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
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
                        tags: s.1,
                        weapons: s.2,
                        global: s.3,
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
                    weapons: stat_tags_1.2,
                    global: stat_tags_1.3,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_2.1,
                    weapons: stat_tags_2.2,
                    global: stat_tags_2.3,
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
                    weapons: stat_tags_1.2,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0,
                    typ: Type::Base,
                    amount: i64::from_str(&c[2]).unwrap(),
                    tags: stat_tags_2.1,
                    weapons: stat_tags_2.2,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^regenerate ([0-9]+) (life|mana) per second$"),
            Box::new(|c| {
                let stat = if &c[2] == "life" {
                    StatId::LifeRegeneration
                } else {
                    StatId::ManaRegeneration
                };
                Some(vec![Mod {
                    stat,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^regenerate ([0-9.]+)% of (life|mana) per second$"),
            Box::new(|c| {
                let stat = if &c[2] == "life" {
                    StatId::LifeRegenerationPct
                } else {
                    StatId::ManaRegenerationPct
                };
                Some(vec![Mod {
                    stat,
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
                    global: stat_tags_1.3,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^allocates ([a-z ]+)$"),
            Box::new(|c| {
                let (node, _) = TREE.nodes.iter().find(|(_, v)| {
                    v.name.to_lowercase() == &c[1]
                })?;
                Some(vec![Mod {
                    allocates: Some(*node),
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^adds ([0-9]+) passive skills$"),
            Box::new(|c| {
                Some(vec![Mod {
                    stat: StatId::AllocatesPassiveSkills,
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    ..Default::default()
                }])
            })
        ),
    ];

    // amounts can be modified by parsing code
    static ref ENDINGS: Vec<(Regex, Mutation)> = vec![
        (regex!("per ([0-9]+) of your lowest attribute$"), Mutation::MultiplierStatLowest((1, &[StatId::Strength, StatId::Dexterity, StatId::Intelligence]))),
        (regex!("per level$"), Mutation::MultiplierProperty((1, property::Int::Level))),
        (regex!("per frenzy charge$"), Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))),
        (regex!("per power charge$"), Mutation::MultiplierProperty((1, property::Int::PowerCharges))),
        (regex!("per endurance charge$"), Mutation::MultiplierProperty((1, property::Int::EnduranceCharges))),
    ];

    static ref ENDING_PER_GENERIC: Regex = regex!("per ([0-9]+)?%? ([a-z ]+)$");

    static ref ONESHOTS: FxHashMap<&'static str, Vec<Mod>> = {
        let mut map = FxHashMap::default();
        map.insert("maximum life becomes 1, immune to chaos damage", vec![
            Mod { stat: StatId::MaximumLife, typ: Type::Override, amount: 1, ..Default::default()},
            Mod { stat: StatId::ChaosResistance, typ: Type::Override, amount: 100, ..Default::default()},
            Mod { stat: StatId::MaximumChaosResistance, typ: Type::Override, amount: 100, ..Default::default()},
        ]);
        map.insert("never deal critical strikes", vec![
            Mod { stat: StatId::CriticalStrikeChance, typ: Type::Override, amount: 0, ..Default::default()},
        ]);
        map.insert("your hits can't be evaded", vec![
            Mod { stat: StatId::ChanceToHit, typ: Type::Override, amount: 100, ..Default::default()},
        ]);
        map.insert("removes all mana", vec![
            Mod { stat: StatId::MaximumMana, typ: Type::Override, amount: 0, ..Default::default()},
        ]);
        map
    };

    static ref MULTISTATS: FxHashMap<&'static str, Vec<StatId>> = {
        let mut map = FxHashMap::default();
        map.insert("attributes", vec![StatId::Strength, StatId::Dexterity, StatId::Intelligence]);
        map.insert("maximum elemental resistances", vec![StatId::MaximumFireResistance, StatId::MaximumColdResistance, StatId::MaximumLightningResistance]);
        map.insert("elemental resistances", vec![StatId::FireResistance, StatId::ColdResistance, StatId::LightningResistance]);
        map.insert("maximum resistances", vec![StatId::MaximumFireResistance, StatId::MaximumColdResistance, StatId::MaximumLightningResistance, StatId::MaximumChaosResistance]);
        map.insert("resistances", vec![StatId::FireResistance, StatId::ColdResistance, StatId::LightningResistance, StatId::ChaosResistance]);
        map.insert("elemental damage", vec![StatId::FireDamage, StatId::ColdDamage, StatId::LightningDamage]);
        map.insert("attack and cast speed", vec![StatId::AttackSpeed, StatId::CastSpeed]);
        map.insert("armour and evasion", vec![StatId::Armour, StatId::EvasionRating]);
        map
    };

    // Assimilate the STATS array into a hashmap with the stat name as key
    static ref STATS_MAP: FxHashMap<&'static str, (StatId, BitFlags<GemTag>, BitFlags<ItemClass>)> = {
        let mut map = FxHashMap::default();
        for entry in STATS.iter() {
            map.insert(entry.0, (entry.1, entry.2, entry.3));
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
    MultiplierStatLowest((i64, &'static [StatId])),
    MultiplierProperty((i64, property::Int)),
}

impl Mutation {
    pub fn set_amount(&mut self, amount: i64) {
        match self {
            Mutation::MultiplierStat(mutation) => mutation.0 = amount,
            Mutation::MultiplierProperty(mutation) => mutation.0 = amount,
            Mutation::MultiplierStatLowest(mutation) => mutation.0 = amount,
        }
    }
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
    // After mutations or other amount-modifying functions
    pub revised_amount: Option<i64>,
    pub mutations: StackVec<Mutation, 5>,
    pub conditions: StackVec<Condition, 5>,
    pub tags: BitFlags<GemTag>,
    pub source: Source,
    pub weapons: BitFlags<ItemClass>,
    pub global: bool,
    pub allocates: Option<u16>,
}

impl Mod {
    pub fn final_amount(&self) -> i64 {
        if let Some(revised_amount) = self.revised_amount {
            revised_amount
        } else {
            self.amount
        }
    }
}

fn parse_ending(m: &str) -> Option<(usize, Ending)> {
    for ending in ENDINGS.iter() {
        if let Some(cap) = ending.0.captures(&m) {
            let mut mutation = ending.1;
            if let Some(amount) = cap.get(1) {
                mutation.set_amount(i64::from_str(amount.as_str()).unwrap());
            }
            return Some((cap.get_match().len(), Ending::Mutation(mutation)));
        }
    }
    if let Some(cap) = ENDING_PER_GENERIC.captures(&m) {
        if let Some(stat) = parse_stat_nomulti(cap.get(2).unwrap().as_str()) {
            let amount = match cap.get(1) {
                Some(amount_str) => i64::from_str(amount_str.as_str()).unwrap(),
                None => 1,
            };
            return Some((cap.get_match().len(), Ending::Mutation(Mutation::MultiplierStat((amount, stat.0)))));
        }
    }
    for ending in ENDINGS_GEMTAGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Tag(ending.1)));
        }
    }
    for ending in ENDINGS_WEAPON_RESTRICTIONS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Weapon(ending.1)));
        }
    }
    for ending in ENDINGS_CONDITIONS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Condition(ending.1)));
        }
    }

    None
}

/// Attempts to parse a chunk like "melee physical damage", non-multi stat
fn parse_stat_nomulti(input: &str) -> Option<(StatId, BitFlags<GemTag>, BitFlags<ItemClass>, bool)> {
    let mut tags = BitFlags::empty();
    let mut global = false;

    let stat = STATS.iter().find(|s| {
        if input.ends_with(s.0) {
            true
        } else {
            false
        }
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

    Some((stat.1, tags | stat.2, stat.3, global))
}

/// Attempts to parse a chunk like "melee physical damage" or a multistat
fn parse_stat(input: &str) -> Option<Vec<(StatId, BitFlags<GemTag>, BitFlags<ItemClass>, bool)>> {
    if let Some(stats) = MULTISTATS.get(input) {
        return Some(stats.iter().map(|id| (*id, BitFlags::EMPTY, BitFlags::EMPTY, false)).collect());
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
/// 1. ONESHOTS array for static string mods
/// 2. if not oneshot, parse right to left:
///    2.1. any amount of ENDINGS
///    2.2. a BEGINNING
pub fn parse_mod(input: &str, source: Source) -> Option<Vec<Mod>> {
    let mut cache = CACHE.lock().expect("Unable to lock CACHE");

    if let Some(mods_opt) = cache.get(input) {
        return mods_opt.to_owned();
    }

    let lowercase = input.to_lowercase();

    if let Some(oneshot) = ONESHOTS.get(lowercase.as_str()) {
        return Some(oneshot.to_owned());
    }

    let mut m = &lowercase[0..];
    let mut mutations: StackVec<Mutation, 5> = Default::default();
    let mut tags = BitFlags::empty();
    let mut weapons = BitFlags::empty();
    let mut conditions: StackVec<Condition, 5> = Default::default();

    while let Some(ending) = parse_ending(&m) {
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
        if let Some(cap) = begin.0.captures(&m) {
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
