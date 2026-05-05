use crate::build::stat::StatId;
use crate::build::{Defence, Slot, property};
use crate::data::base_item::ItemClass;
use crate::data::gem::GemTag;
use crate::gem::Gem;
use crate::data::TREE;
use crate::item::{self, Item};
use crate::stackvec::{StackVec};
use crate::stackvec;
use crate::tree::NOTHINGNESS_NODE_ID;
use enumflags2::{make_bitflags as flags, BitFlags, bitflags};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use dashmap::DashMap;
use rustc_hash::FxHashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ops::Neg;
use std::str::FromStr;
use std::sync::{Mutex, RwLock};

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
        map.insert("warcry", GemTag::Warcry);
        map
    };
}

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

const BEGINNINGS: &[(&str, BitFlags<GemTag>, BitFlags<ItemClass>, &[Condition])] = &[
    ("axe attacks deal", flags!(GemTag::Attack), ItemClass::AXES, &[]),
    ("sword attacks deal", flags!(GemTag::Attack), ItemClass::SWORDS, &[]),
    ("mace or sceptre attacks deal", flags!(GemTag::Attack), flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre}), &[]),
    ("dagger attacks deal", flags!(GemTag::Attack), ItemClass::DAGGERS, &[]),
    ("claw attacks deal", flags!(GemTag::Attack), flags!(ItemClass::Claw), &[]),
    ("attacks with two handed melee weapons deal", flags!(GemTag::Attack), ItemClass::TWO_HANDED_MELEE, &[]),
    ("attacks with one handed weapons deal", flags!(GemTag::Attack), ItemClass::ONE_HANDED, &[]),
    ("attacks with melee weapons deal", flags!(GemTag::Attack), ItemClass::MELEE, &[]),
    ("attack skills deal", flags!(GemTag::Attack), BitFlags::EMPTY, &[]),
    ("attacks have", flags!(GemTag::Attack), BitFlags::EMPTY, &[]),
    ("melee skills have", flags!(GemTag::Melee), BitFlags::EMPTY, &[]),
    ("minions have", flags!(GemTag::Minion), BitFlags::EMPTY, &[]),
    ("minions deal", flags!(GemTag::Minion), BitFlags::EMPTY, &[]),
];

const ENDINGS: &[(&str, BitFlags<GemTag>, BitFlags<ItemClass>, BitFlags<ModFlag>, &[Condition])] = &[
    ("of aura skills", flags!(GemTag::Aura), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("of curse skills", flags!(GemTag::Curse), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("of hex skills", flags!(GemTag::Hex), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with attack skills", flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("to attacks", flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("of attacks", flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("on targets you hit with attacks", flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with attacks", flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("of skills", flags!(GemTag::Grants_Active_Skill), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with mines", flags!(GemTag::Mine), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with traps", flags!(GemTag::Trap), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with bow skills", flags!(GemTag::Bow), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with totem skills", flags!(GemTag::Totem), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("for spell damage", flags!(GemTag::Spell), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with cold skills", flags!(GemTag::Cold), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with fire skills", flags!(GemTag::Fire), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with lightning skills", flags!(GemTag::Lightning), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with brand skills", flags!(GemTag::Brand), BitFlags::EMPTY, BitFlags::EMPTY, &[]),
    ("with axes", BitFlags::EMPTY, ItemClass::AXES, BitFlags::EMPTY, &[]),
    ("with swords", BitFlags::EMPTY, ItemClass::SWORDS, BitFlags::EMPTY, &[]),
    ("with maces", BitFlags::EMPTY, ItemClass::MACES, BitFlags::EMPTY, &[]),
    ("with two handed melee weapons", BitFlags::EMPTY, ItemClass::TWO_HANDED_MELEE, BitFlags::EMPTY, &[]),
    ("with one handed melee weapons", BitFlags::EMPTY, ItemClass::ONE_HANDED_MELEE, BitFlags::EMPTY, &[]),
    ("with one handed weapons", BitFlags::EMPTY, ItemClass::ONE_HANDED, BitFlags::EMPTY, &[]),
    ("with staves", BitFlags::EMPTY, ItemClass::STAVES, BitFlags::EMPTY, &[]),
    ("with bows", BitFlags::EMPTY, flags!(ItemClass::Bow), BitFlags::EMPTY, &[]),
    ("with claws", BitFlags::EMPTY, flags!(ItemClass::Claw), BitFlags::EMPTY, &[]),
    ("with wands", BitFlags::EMPTY, flags!(ItemClass::Wand), BitFlags::EMPTY, &[]),
    ("with daggers", BitFlags::EMPTY, ItemClass::DAGGERS, BitFlags::EMPTY, &[]),
    ("with maces or sceptres", BitFlags::EMPTY, flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre}), BitFlags::EMPTY, &[]),
    ("while fortified", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::GreaterEqualProperty((1, property::Int::Fortification))]),
    ("if you've dealt a critical strike recently", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::DealtCritRecently))]),
    ("if you've blocked recently", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::BlockedRecently))]),
    ("while leeching", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::Leeching))]),
    ("when on full life", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::OnFullLife))]),
    ("while on full energy shield", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::OnFullEnergyShield))]),
    ("while on full life", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::OnFullLife))]),
    ("while on low life", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::OnLowLife))]),
    ("when on low life", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::PropertyBool((true, property::Bool::OnLowLife))]),
    ("while holding a shield", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(flags!(ItemClass::Shield))]),
    ("while holding a staff or shield", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(ItemClass::STAVES.union_c(flags!(ItemClass::Shield)))]),
    ("while wielding a wand", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(flags!(ItemClass::Wand))]),
    ("while wielding a staff", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(ItemClass::STAVES)]),
    ("while wielding a sword", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(ItemClass::SWORDS)]),
    ("while wielding a dagger", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(ItemClass::DAGGERS)]),
    ("while wielding a mace or sceptre", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(flags!(ItemClass::{OneHandMace | TwoHandMace | Sceptre}))]),
    ("while wielding a claw or dagger", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileWielding(flags!(ItemClass::{Dagger | RuneDagger | Claw}))]),
    ("while dual wielding", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileDualWielding]),
    ("while dual wielding or holding a shield", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[Condition::WhileDualWielding, Condition::WhileWielding(flags!(ItemClass::Shield))]),
    ("if equipped helmet, body armour, gloves, and boots all have armour", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[
        Condition::SlotsHaveDefence((Defence::Armour, &[Slot::Helm, Slot::BodyArmour, Slot::Gloves, Slot::Boots])),
    ]),
    ("if there are no life modifiers on equipped body armour", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[
        Condition::SlotLesserEqualStats((Slot::BodyArmour, 0, &[StatId::MaximumLife, StatId::LifeRegeneration, StatId::LifeRegenerationPct, StatId::LifeRegenerationRate])),
    ]),
    ("with hits and ailments", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::{Hit | Ailment}), &[]),
    ("with ailments", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::Ailment), &[]),
    ("with bleeding", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::Bleed), &[]),
    ("for bleeding", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::Bleed), &[]),
    ("with poison", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::Poison), &[]),
    ("for poison", BitFlags::EMPTY, BitFlags::EMPTY, flags!(ModFlag::Poison), &[]),
    ("if you have at least 6 life masteries allocated", BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY, &[
        Condition::GreaterEqualMasteryAllocated(("Life Mastery", 6)),
    ]),
];

// Order is important for overlapping stats
// like "area of effect" and "effect"
const STATS: &[(&'static str, StatId, BitFlags<GemTag>, BitFlags<ItemClass>, BitFlags<ModFlag>)] = &[
    ("strength", StatId::Strength, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("dexterity", StatId::Dexterity, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("intelligence", StatId::Intelligence, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("attributes", StatId::Attributes, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("action speed", StatId::ActionSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("attack speed", StatId::AttackSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cast speed", StatId::CastSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("warcry speed", StatId::WarcrySpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cooldown recovery speed", StatId::CooldownRecoverySpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cooldown recovery rate", StatId::CooldownRecoverySpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("projectile speed", StatId::ProjectileSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("trap throwing speed", StatId::TrapThrowingSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block attack damage", StatId::ChanceToBlockAttackDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block spell damage", StatId::ChanceToBlockSpellDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to block", StatId::ChanceToBlockAttackDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY), // local on shields
    ("chance to suppress spell damage", StatId::ChanceToSuppressSpellDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to deal double damage", StatId::ChanceToDealDoubleDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage over time multiplier", StatId::FireDotMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage over time multiplier", StatId::ColdDotMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage over time multiplier", StatId::ChaosDotMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage over time multiplier", StatId::PhysicalDotMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("damage over time multiplier", StatId::DotMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage penetration", StatId::FireDamagePen, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning damage penetration", StatId::LightningDamagePen, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage penetration", StatId::ColdDamagePen, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage penetration", StatId::ChaosDamagePen, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage over time", StatId::FireDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage over time", StatId::ColdDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage over time", StatId::ChaosDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage over time", StatId::PhysicalDamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("damage over time", StatId::DamageOverTime, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical damage reduction", StatId::PhysicalDamageReduction, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire damage", StatId::FireDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold damage", StatId::ColdDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning damage", StatId::LightningDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos damage", StatId::ChaosDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum physical attack damage", StatId::MinPhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum physical attack damage", StatId::MaxPhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY, BitFlags::EMPTY),
    ("added minimum physical damage", StatId::AddedMinPhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("added maximum physical damage", StatId::AddedMaxPhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("physical attack damage", StatId::PhysicalDamage, flags!(GemTag::Attack), BitFlags::EMPTY, flags!(ModFlag::Hit)),
    ("physical damage", StatId::PhysicalDamage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("wand damage", StatId::Damage, BitFlags::EMPTY, flags!(ItemClass::Wand), BitFlags::EMPTY),
    ("damage", StatId::Damage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("area of effect", StatId::AreaOfEffect, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("accuracy rating", StatId::AccuracyRating, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("movement speed", StatId::MovementSpeed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("skill effect duration", StatId::SkillEffectDuration, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("duration", StatId::Duration, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("impale effect", StatId::ImpaleEffect, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum frenzy charges", StatId::MinimumFrenzyCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum power charges", StatId::MinimumPowerCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum endurance charges", StatId::MinimumEnduranceCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum frenzy charges", StatId::MaximumFrenzyCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum power charges", StatId::MaximumPowerCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum endurance charges", StatId::MaximumEnduranceCharges, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum fortification", StatId::MaximumFortification, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum life", StatId::MaximumLife, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum mana", StatId::MaximumMana, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("minimum rage", StatId::MinimumRage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum rage", StatId::MaximumRage, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum energy shield", StatId::MaximumEnergyShield, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("energy shield recharge rate", StatId::EnergyShieldRechargeRate, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("energy shield", StatId::MaximumEnergyShield, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("life regeneration rate", StatId::LifeRegenerationRate, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana regeneration rate", StatId::ManaRegenerationRate, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana reservation efficiency", StatId::ManaReservationEfficiency, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("critical strike chance", StatId::CriticalStrikeChance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("critical strike multiplier", StatId::CriticalStrikeMultiplier, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("armour", StatId::Armour, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("evasion rating", StatId::EvasionRating, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("stun threshold", StatId::StunThreshold, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to avoid being stunned", StatId::ChanceToAvoidBeingStunned, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum fire resistance", StatId::MaximumFireResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum cold resistance", StatId::MaximumColdResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum lightning resistance", StatId::MaximumLightningResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("maximum chaos resistance", StatId::MaximumChaosResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("fire resistance", StatId::FireResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cold resistance", StatId::ColdResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("lightning resistance", StatId::LightningResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chaos resistance", StatId::ChaosResistance, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask charges gained", StatId::FlaskChargesGained, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask effect duration", StatId::FlaskEffectDuration, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask recovery rate", StatId::FlaskRecoveryRate, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("flask charges used", StatId::FlaskChargesUsed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana cost", StatId::ManaCost, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("life cost", StatId::LifeCost, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("cost", StatId::Cost, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("passive skill points", StatId::PassiveSkillPoints, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("passive skill point", StatId::PassiveSkillPoints, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to cause bleeding", StatId::ChanceToBleed, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to ignite", StatId::ChanceToIgnite, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to shock", StatId::ChanceToShock, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("chance to poison on hit", StatId::ChanceToPoison, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("poison duration", StatId::PoisonDuration, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("effect of non-curse auras from your skills", StatId::AuraEffect, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("life", StatId::MaximumLife, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
    ("mana", StatId::MaximumMana, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY),
];

lazy_static! {
    static ref CORES: Vec<(Regex, Box<dyn Fn(&Captures) -> Option<Vec<Mod>> + Send + Sync>)> = vec![
        (
            regex!(r"^([0-9]+)% (increased|reduced) ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                let mut amount = i64::from_str(&c[1]).unwrap();
                amount = match &c[2] {
                    "reduced" => amount.neg(),
                    "increased" => amount,
                    _ => panic!(),
                };
                Some(stat_tags.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::Inc, amount, tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }).collect())
            })
        ), (
            regex!(r"^([+-]?[0-9]+)%? (?:additional )?(?:to )?(?:all )?([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                let amount = i64::from_str(&c[1]).unwrap();
                Some(stat_tags.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::Base, amount, tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% (increased|reduced) ([a-z -]+) and ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat(&c[3])?;
                let stat_tags_2 = parse_stat(&c[4])?;
                let mut amount = i64::from_str(&c[1]).unwrap();
                amount = match &c[2] {
                    "reduced" => amount.neg(),
                    "increased" => amount,
                    _ => panic!(),
                };
                let mut ret: Vec<Mod> = stat_tags_1.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::Inc, amount, tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }).collect();
                ret.extend(stat_tags_2.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::Inc, amount, tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }));
                Some(ret)
            })
        ), (
            regex!(r"^([0-9]+)% more ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::More, amount: i64::from_str(&c[1]).unwrap(), tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% less ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod { stat: s.0, typ: Type::More, amount: i64::from_str(&c[1]).unwrap().neg(), tags: s.1, weapons: s.2, flags: s.3, ..Default::default() }
                }).collect())
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z -]+) and ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[2])?;
                let stat_tags_2 = parse_stat_nomulti(&c[3])?;
                Some(vec![
                    Mod { stat: stat_tags_1.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), tags: stat_tags_1.1, weapons: stat_tags_1.2, ..Default::default() },
                    Mod { stat: stat_tags_2.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), tags: stat_tags_2.1, weapons: stat_tags_2.2, ..Default::default() },
                ])
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z]+) and ([a-z]+) resistances$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("{} resistance", &c[2]).as_str()).cloned()?;
                let stat_tags_2 = STATS_MAP.get(format!("{} resistance", &c[3]).as_str()).cloned()?;
                Some(vec![
                    Mod { stat: stat_tags_1.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), tags: stat_tags_1.1, ..Default::default() },
                    Mod { stat: stat_tags_2.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), tags: stat_tags_2.1, ..Default::default() },
                ])
            })
        ), (
            regex!(r"^adds ([0-9]+) to ([0-9]+) ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("added minimum {}", &c[3]).as_str()).cloned()?;
                let stat_tags_2 = STATS_MAP.get(format!("added maximum {}", &c[3]).as_str()).cloned()?;
                Some(vec![
                    Mod { stat: stat_tags_1.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), tags: stat_tags_1.1, weapons: stat_tags_1.2, ..Default::default() },
                    Mod { stat: stat_tags_2.0, typ: Type::Base, amount: i64::from_str(&c[2]).unwrap(), tags: stat_tags_2.1, weapons: stat_tags_2.2, ..Default::default() },
                ])
            })
        ), (
            regex!(r"^regenerate ([0-9.]+)(% of)? (life|mana) per second$"),
            Box::new(|c| {
                let stat = match(&c[3], c.get(2).is_some()) {
                    ("life", false) => StatId::LifeRegeneration,
                    ("life", true) => StatId::LifeRegenerationPct,
                    ("mana", false) => StatId::ManaRegeneration,
                    ("mana", true) => StatId::ManaRegenerationPct,
                    _ => panic!(),
                };

                Some(vec![Mod { stat, typ: Type::Base, amount: parse_val100(&c[1])?, ..Default::default() }])
            })
        ), (
            regex!(r"^damage penetrates ([0-9]+)% ([a-z]+) resistance$"),
            Box::new(|c| {
                let stat_tags_1 = STATS_MAP.get(format!("{} damage penetration", &c[2]).as_str()).cloned()?;
                Some(vec![Mod { stat: stat_tags_1.0, typ: Type::Base, amount: parse_val100(&c[1])?, tags: stat_tags_1.1, ..Default::default() }])
            })
        ), (
            regex!(r"^your ([a-z -]+) is equal to ([0-9]+)% of your ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[1])?;
                let stat_tags_2 = parse_stat_nomulti(&c[3])?;
                let pct = i64::from_str(&c[2]).unwrap();
                Some(vec![Mod { stat: stat_tags_1.0, typ: Type::Override, tags: stat_tags_1.1, weapons: stat_tags_2.2, flags: stat_tags_1.3, mutations: stackvec!(Mutation::StatPct((pct, stat_tags_2.0))), ..Default::default() }])
            })
        ), (
            regex!(r"^grants ([0-9]+) ([a-z -]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[2])?;
                Some(vec![Mod { stat: stat_tags_1.0, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^allocates ([a-z '-]+)$"),
            Box::new(|c| {
                let (node, _) = TREE.nodes.iter().find(|(_, v)| {
                    v.name.to_lowercase() == &c[1]
                })?;
                Some(vec![Mod { allocates: Some(*node), ..Default::default() }])
            })
        ), (
            regex!(r"^adds ([0-9]+) passive skills$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::AllocatesPassiveSkills, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^([0-9]+) added passive skills? (are|is a) jewel sockets?$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::AddedPassivesAreJewelSockets, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^adds ([0-9]+) jewel socket passive skills$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::AddedPassivesAreJewelSockets, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^adds ([0-9]+) small passive skills which grant nothing$"),
            Box::new(|c| {
                Some(vec![
                    Mod { stat: StatId::AllocatesPassiveSkills, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() },
                    Mod { stat: StatId::AddedPassiveSkillsGrantNode, typ: Type::Base, amount: NOTHINGNESS_NODE_ID as i64, ..Default::default() },
                ])
            })
        ), (
            regex!(r"^has ([0-9]+) abyssal sockets?$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::AbyssalSockets, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^attacks have ([0-9]+) abyssal sockets?$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::AbyssalSockets, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^added small passive skills have ([0-9]+)% increased effect$"),
            Box::new(|c| {
                Some(vec![Mod { stat: StatId::SmallPassiveIncreasedEffect, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ), (
            regex!(r"^([0-9]+)% of (physical|cold|fire|lightning|chaos) damage converted to (fire|cold|lightning|chaos) damage$"),
            Box::new(|c| {
                let stat = CONVERSIONS.get(&(c[2].to_string(), c[3].to_string()))?;
                Some(vec![Mod { stat: *stat, typ: Type::Base, amount: i64::from_str(&c[1]).unwrap(), ..Default::default() }])
            })
        ),
    ];

    // amounts can be modified by parsing code
    static ref ENDINGS_MUTATIONS: Vec<(Regex, Mutation)> = vec![
        (regex!(",? ?up to(?: a maximum of)? ([0-9]+)%?$"), Mutation::UpTo(1)),
        (regex!("per ([0-9]+) of your lowest attribute$"), Mutation::MultiplierStatLowest((1, &[StatId::Strength, StatId::Dexterity, StatId::Intelligence]))),
        (regex!("per ([0-9]+) maximum energy shield on shield$"), Mutation::MultiplierSlotDefence((1, Slot::Offhand, Defence::EnergyShield))),
        (regex!("per level$"), Mutation::MultiplierProperty((1, property::Int::Level))),
        (regex!("per frenzy charge$"), Mutation::MultiplierProperty((1, property::Int::FrenzyCharges))),
        (regex!("per power charge$"), Mutation::MultiplierProperty((1, property::Int::PowerCharges))),
        (regex!("per endurance charge$"), Mutation::MultiplierProperty((1, property::Int::EnduranceCharges))),
        (regex!("per fortification$"), Mutation::MultiplierProperty((1, property::Int::Fortification))),
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
        map.insert("strength's damage bonus applies to all spell damage as well", vec![
            Mod { stat: StatId::Damage, typ: Type::Inc, amount: 1, tags: GemTag::Spell.into(), mutations: stackvec!(Mutation::MultiplierStat((5, StatId::Strength))), ..Default::default()},
        ]);
        map.insert("removes all energy shield", vec![
            Mod { stat: StatId::MaximumEnergyShield, typ: Type::Override, amount: 0, ..Default::default()},
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
    static ref STATS_MAP: FxHashMap<&'static str, (StatId, BitFlags<GemTag>, BitFlags<ItemClass>, BitFlags<ModFlag>)> = {
        let mut map = FxHashMap::default();
        for entry in STATS.iter() {
            map.insert(entry.0, (entry.1, entry.2, entry.3, entry.4));
        }
        map
    };

    static ref CONVERSIONS: FxHashMap<(String, String), StatId> = {
        let mut map = FxHashMap::default();
        map.insert(("physical".into(), "lightning".into()), StatId::PhysicalToLightningConversion);
        map.insert(("physical".into(), "cold".into()), StatId::PhysicalToColdConversion);
        map.insert(("physical".into(), "fire".into()), StatId::PhysicalToFireConversion);
        map.insert(("physical".into(), "chaos".into()), StatId::PhysicalToChaosConversion);
        map.insert(("lightning".into(), "cold".into()), StatId::LightningToColdConversion);
        map.insert(("lightning".into(), "fire".into()), StatId::LightningToFireConversion);
        map.insert(("lightning".into(), "chaos".into()), StatId::LightningToChaosConversion);
        map.insert(("cold".into(), "fire".into()), StatId::ColdToFireConversion);
        map.insert(("cold".into(), "chaos".into()), StatId::ColdToChaosConversion);
        map.insert(("fire".into(), "chaos".into()), StatId::FireToChaosConversion);
        map
    };
}

pub fn lol() {

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Type {
    #[default]
    Base,
    Inc,
    More,
    Override,
}

#[derive(Debug, Clone, Copy)]
pub enum Mutation {
    MultiplierStat((i64, StatId)),
    MultiplierStatLowest((i64, &'static [StatId])),
    MultiplierProperty((i64, property::Int)),
    StatPct((i64, StatId)),
    MultiplierSlotDefence((i64, Slot, Defence)),
    UpTo(i64),
    IncreasedEffect(i64),
}

impl Mutation {
    pub fn set_amount(&mut self, amount: i64) {
        match self {
            Mutation::MultiplierStat(mutation) => mutation.0 = amount,
            Mutation::MultiplierProperty(mutation) => mutation.0 = amount,
            Mutation::MultiplierStatLowest(mutation) => mutation.0 = amount,
            Mutation::StatPct(mutation) => mutation.0 = amount,
            Mutation::MultiplierSlotDefence(mutation) => mutation.0 = amount,
            Mutation::UpTo(mutation) => *mutation = amount,
            Mutation::IncreasedEffect(mutation) => *mutation = amount,
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
    WhileDualWielding,
    SlotsHaveDefence((Defence, &'static [Slot])),
    SlotLesserEqualStats((Slot, i64, &'static [StatId])),
    GreaterEqualMasteryAllocated((&'static str, u32)),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    #[default]
    Innate,
    Node(u32),
    Mastery((u32, u32)),
    Item(Slot),
    Gem(&'static str),
    Custom(&'static str),
}

#[bitflags]
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum ModFlag {
    Hit,
    Ailment,
    Bleed,
    Poison,
    Aura,
    Buff,
}

const MUTATIONS_COUNT: usize = 2;
const CONDITIONS_COUNT: usize = 2;

#[derive(Default, Debug, Clone, Copy)]
pub struct Mod {
    pub stat: StatId,
    pub typ: Type,
    pub amount: i64,
    pub revised_amount: Option<i64>, // After mutations or other amount-modifying functions
    pub mutations: StackVec<Mutation, MUTATIONS_COUNT>,
    pub conditions: StackVec<Condition, CONDITIONS_COUNT>,
    pub tags: BitFlags<GemTag>,
    pub source: Source,
    pub weapons: BitFlags<ItemClass>,
    pub flags: BitFlags<ModFlag>,
    pub allocates: Option<u32>,
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

fn parse_beginning(m: &str) -> Option<(usize, Mod)> {
    for beginning in BEGINNINGS {
        if m.len() > beginning.0.len() + 1 && m.starts_with(beginning.0) {
            let mut ret = Mod::default();
            ret.tags.insert(beginning.1);
            ret.weapons.insert(beginning.2);
            ret.conditions.extend_from_slice(beginning.3);
            return Some((beginning.0.len(), ret));
        }
    }

    None
}

fn parse_ending(m: &str) -> Option<(usize, Mod)> {
    let mut ret = Mod::default();

    for ending in ENDINGS_MUTATIONS.iter() {
        if let Some(cap) = ending.0.captures(&m) {
            let mut mutation = ending.1;
            if let Some(amount) = cap.get(1) {
                mutation.set_amount(i64::from_str(amount.as_str()).unwrap());
            }
            ret.mutations.push(mutation);
            return Some((cap.get_match().len(), ret));
        }
    }

    if let Some(cap) = ENDING_PER_GENERIC.captures(&m) {
        if let Some(stat) = parse_stat_nomulti(cap.get(2).unwrap().as_str()) {
            let amount = match cap.get(1) {
                Some(amount_str) => i64::from_str(amount_str.as_str()).unwrap(),
                None => 1,
            };
            ret.mutations.push(Mutation::MultiplierStat((amount, stat.0)));
            return Some((cap.get_match().len(), ret));
        }
    }

    for ending in ENDINGS {
        if m.ends_with(ending.0) {
            ret.tags.insert(ending.1);
            ret.weapons.insert(ending.2);
            if !ending.2.is_empty() {
                // Hypothesis: Endings mentioning a weapon restriction are always about Hits
                ret.flags.insert(ModFlag::Hit);
            }
            ret.flags.insert(ending.3);
            ret.conditions.extend_from_slice(ending.4);
            return Some((ending.0.len(), ret));
        }
    }

    None
}

/// Attempts to parse a chunk like "melee physical damage", non-multi stat
fn parse_stat_nomulti(input: &str) -> Option<(StatId, BitFlags<GemTag>, BitFlags<ItemClass>, BitFlags<ModFlag>)> {
    let mut tags = BitFlags::empty();

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
        } else {
            return None;
        }
    }

    Some((stat.1, tags | stat.2, stat.3, stat.4))
}

/// Attempts to parse a chunk like "melee physical damage" or a multistat
fn parse_stat(input: &str) -> Option<Vec<(StatId, BitFlags<GemTag>, BitFlags<ItemClass>, BitFlags<ModFlag>)>> {
    if let Some(stats) = MULTISTATS.get(input) {
        return Some(stats.iter().map(|id| (*id, BitFlags::EMPTY, BitFlags::EMPTY, BitFlags::EMPTY)).collect());
    }

    if let Some(stat) = parse_stat_nomulti(input) {
        return Some(vec![stat]);
    }

    None
}

lazy_static! {
    pub static ref CACHE: DashMap<String, Option<Vec<Mod>>> = DashMap::new();
}

/// Attempts to parse a modifier like "30℅ increased poison damage while focussed"
/// 1. ONESHOTS array for static string mods
/// 2. if not oneshot, parse right to left:
///    2.1. any amount of ENDINGS
///    2.2. any amount of BEGINNINGS
///    2.3. a CORES
pub fn parse_mod(input: &str, source: Source) -> Option<Vec<Mod>> {
    if let Some(cached_mods) = CACHE.get(input) {
        let mut mods_opt = cached_mods.to_owned();
        if let Some(mods) = &mut mods_opt {
            for m in mods {
                m.source = source;
            }
        }
        return mods_opt;
    }

    let lowercase = input.to_lowercase();

    if let Some(oneshot) = ONESHOTS.get(lowercase.as_str()) {
        let mut mods = oneshot.to_owned();
        for m in &mut mods {
            m.source = source;
        }
        return Some(mods);
    }

    let mut m = &lowercase[0..];
    let mut mutations: StackVec<Mutation, MUTATIONS_COUNT> = Default::default();
    let mut tags = BitFlags::empty();
    let mut weapons = BitFlags::empty();
    let mut conditions: StackVec<Condition, CONDITIONS_COUNT> = Default::default();
    let mut flags: BitFlags<ModFlag> = BitFlags::EMPTY;

    while let Some((size, modifier)) = parse_ending(&m) {
        m = &m[0..m.len() - size];
        if let Some(c) = m.chars().last() && c == ' ' {
            m = &m[0..m.len() - 1];
        }

        mutations.extend_from_slice(&modifier.mutations);
        tags.insert(modifier.tags);
        weapons.insert(modifier.weapons);
        flags.insert(modifier.flags);
        conditions.extend_from_slice(&modifier.conditions);
    }

    while let Some((size, modifier)) = parse_beginning(&m) {
        m = &m[size + 1..m.len()];
        mutations.extend_from_slice(&modifier.mutations);
        tags.insert(modifier.tags);
        weapons.insert(modifier.weapons);
        flags.insert(modifier.flags);
        conditions.extend_from_slice(&modifier.conditions);
    }

    for begin in CORES.iter() {
        if let Some(cap) = begin.0.captures(&m) {
            if let Some(mut mods) = begin.1(&cap) {
                for modifier in &mut mods {
                    modifier.tags.insert(tags);
                    modifier.mutations.extend_from_slice(&mutations);
                    modifier.weapons.insert(weapons);
                    modifier.conditions.extend_from_slice(&conditions);
                    modifier.source = source;
                    modifier.flags.insert(flags);
                }
                CACHE.insert(input.to_string(), Some(mods.clone()));
                return Some(mods);
            }
        }
    }

    CACHE.insert(input.to_string(), None);
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
    assert!(parse_mod("40% of physical damage converted to fire damage", Source::Innate).is_some());
    assert!(parse_mod("50% of lightning damage converted to cold damage", Source::Innate).is_some());
    assert!(parse_mod("100% of fire damage converted to chaos damage", Source::Innate).is_some());
    // Invalid conversion direction (chaos can't convert to physical)
    assert!(parse_mod("40% of chaos damage converted to physical damage", Source::Innate).is_none());
}

#[test]
fn count_tree_parses() {
    use crate::data::tree::NodeType;

    let mut nb_mods = 0;
    let mut nb_mods_success = 0;
    let mut failed_mods: FxHashMap<String, usize> = Default::default();

    let mut func = |stat| {
        nb_mods += 1;
        if parse_mod(stat, Source::Innate).is_some() {
            nb_mods_success += 1;
        } else {
            *failed_mods.entry(stat.to_owned()).or_default() += 1;
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
    let mut sorted_failed_mods: Vec<(&String, &usize)> = failed_mods.iter().collect();
    sorted_failed_mods.sort_by(|a, b| b.1.cmp(a.1));
    for (stat, count) in sorted_failed_mods {
        println!("{}: {}", stat, count);
    }
}
