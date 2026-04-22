use crate::build::stat::StatId;
use crate::data::gem::GemTag;
use crate::modifier::{Mod, Type, ModFlag};
use rustc_hash::FxHashMap;
use enumflags2::{make_bitflags as flags, BitFlags};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    // Order is important, end-of-string match is performed in-order
    static ref GEMSTATS_GENERIC: Vec<(&'static str, Vec<Mod>)> = vec![
        ("spell_minimum_base_fire_damage", vec![
            Mod { stat: StatId::BaseMinFireDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_maximum_base_fire_damage", vec![
            Mod { stat: StatId::BaseMaxFireDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_minimum_base_lightning_damage", vec![
            Mod { stat: StatId::BaseMinLightningDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_maximum_base_lightning_damage", vec![
            Mod { stat: StatId::BaseMaxLightningDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_minimum_base_cold_damage", vec![
            Mod { stat: StatId::BaseMinColdDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_maximum_base_cold_damage", vec![
            Mod { stat: StatId::BaseMaxColdDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_minimum_base_chaos_damage", vec![
            Mod { stat: StatId::BaseMinChaosDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("spell_maximum_base_chaos_damage", vec![
            Mod { stat: StatId::BaseMaxChaosDamage, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("minimum_added_fire_damage", vec![
            Mod { stat: StatId::AddedMinFireDamage, ..Default::default() },
        ]),
        ("maximum_added_fire_damage", vec![
            Mod { stat: StatId::AddedMaxFireDamage, ..Default::default() },
        ]),
        ("minimum_added_lightning_damage", vec![
            Mod { stat: StatId::AddedMinLightningDamage, ..Default::default() },
        ]),
        ("maximum_added_lightning_damage", vec![
            Mod { stat: StatId::AddedMaxLightningDamage, ..Default::default() },
        ]),
        ("minimum_added_cold_damage", vec![
            Mod { stat: StatId::AddedMinColdDamage, ..Default::default() },
        ]),
        ("maximum_added_cold_damage", vec![
            Mod { stat: StatId::AddedMaxColdDamage, ..Default::default() },
        ]),
        ("minimum_added_chaos_damage", vec![
            Mod { stat: StatId::AddedMinChaosDamage, ..Default::default() },
        ]),
        ("maximum_added_chaos_damage", vec![
            Mod { stat: StatId::AddedMaxChaosDamage, ..Default::default() },
        ]),
        ("poison_and_bleeding_damage", vec![
            Mod { stat: StatId::Damage, flags: flags!(ModFlag::{Bleed | Poison}), ..Default::default() },
        ]),
        ("melee_physical_damage", vec![
            Mod { stat: StatId::PhysicalDamage, tags: flags!(GemTag::Melee), flags: flags!(ModFlag::Hit), ..Default::default() },
        ]),
        ("herald_of_purity_physical_damage", vec![
            Mod { stat: StatId::PhysicalDamage, flags: flags!(ModFlag::Buff), ..Default::default() },
        ]),
        ("physical_damage", vec![
            Mod { stat: StatId::PhysicalDamage, ..Default::default() },
        ]),
        ("fire_damage", vec![
            Mod { stat: StatId::FireDamage, ..Default::default() },
        ]),
        ("lightning_damage", vec![
            Mod { stat: StatId::LightningDamage, ..Default::default() },
        ]),
        ("cold_damage", vec![
            Mod { stat: StatId::ColdDamage, ..Default::default() },
        ]),
        ("chaos_damage", vec![
            Mod { stat: StatId::ChaosDamage, ..Default::default() },
        ]),
        ("melee_area_damage", vec![
            Mod { stat: StatId::Damage, tags: flags!(GemTag::{Melee | Area}), flags: flags!(ModFlag::Hit), ..Default::default() },
        ]),
        ("melee_damage", vec![
            Mod { stat: StatId::Damage, tags: flags!(GemTag::Melee), ..Default::default() },
        ]),
        ("area_damage", vec![
            Mod { stat: StatId::Damage, tags: flags!(GemTag::Area), ..Default::default() },
        ]),
        ("deal_no_elemental_damage", vec![
            Mod { stat: StatId::FireDamage, typ: Type::More, amount: -100, ..Default::default() },
            Mod { stat: StatId::ColdDamage, typ: Type::More, amount: -100, ..Default::default() },
            Mod { stat: StatId::LightningDamage, typ: Type::More, amount: -100, ..Default::default() },
        ]),
        ("deal_no_chaos_damage", vec![
            Mod { stat: StatId::ChaosDamage, typ: Type::More, amount: -100, ..Default::default() },
        ]),
        ("attack_speed", vec![
            Mod { stat: StatId::AttackSpeed, tags: flags!(GemTag::Attack), ..Default::default() },
        ]),
        ("base_cast_speed", vec![
            Mod { stat: StatId::CastSpeed, tags: flags!(GemTag::Spell), ..Default::default() },
        ]),
        ("skill_area_of_effect", vec![
            Mod { stat: StatId::AreaOfEffect, ..Default::default() },
        ]),
        ("shock_as_though_damage", vec![
            Mod { stat: StatId::ShockAsThoughDamage, ..Default::default() },
        ]),
        ("additional_weapon_base_attack_time_ms", vec![
            Mod { stat: StatId::AddedAttackTime, ..Default::default() },
        ]),
        ("accuracy_rating", vec![
            Mod { stat: StatId::AccuracyRating, typ: Type::Base, ..Default::default() },
        ]),
        ("skill_buff_grants_critical_strike_chance", vec![
            Mod { stat: StatId::CriticalStrikeChance, flags: flags!(ModFlag::Aura), ..Default::default() },
        ]),
        ("critical_strike_chance", vec![
            Mod { stat: StatId::CriticalStrikeChance, ..Default::default() },
        ]),
        ("base_fire_damage_resistance", vec![
            Mod { stat: StatId::FireResistance, ..Default::default() },
        ]),
        ("damage", vec![
            Mod { stat: StatId::Damage, ..Default::default() },
        ]),
    ];

    // HashMap<gemname<HashMap<statname>>>>
    static ref GEMSTATS_PERGEM: FxHashMap<&'static str, FxHashMap<&'static str, Vec<Mod>>> =
    [
        // Gem name = GemData::base_item::display_name
        ("Precision", [
            ("additional_accuracy", vec![
                Mod { stat: StatId::AccuracyRating, typ: Type::Base, flags: flags!(ModFlag::Aura), ..Default::default() },
            ]),
        ].into_iter().collect()),
        ("Haste", [
            ("attack_speed", vec![
                Mod { stat: StatId::AttackSpeed, typ: Type::Inc, flags: flags!(ModFlag::Aura), ..Default::default() },
            ]),
        ].into_iter().collect()),
    ].into_iter().collect();
}

pub fn match_gemstat(gem_basename: &str, mut stat: &str) -> Option<Vec<Mod>> {
    let mut typ_override = None;
    let mut gem_tags = BitFlags::EMPTY;
    let mut mods = vec![];

    if let Some(substat) = stat.strip_suffix("_granted_from_skill") {
        stat = substat;
    } else if let Some(substat) = stat.strip_suffix("_from_melee_hits") {
        gem_tags.insert(GemTag::Melee);
        stat = substat;
    }

    let search_in = if let Some(ret) = stat.strip_suffix("_+%_final") {
        typ_override = Some(Type::More);
        ret
    } else if let Some(ret) = stat.strip_suffix("_+%") {
        typ_override = Some(Type::Inc);
        ret
    } else {
        stat
    };

    if let Some(gemstats) = GEMSTATS_PERGEM.get(gem_basename) &&
       let Some(gem_mods) = gemstats.get(search_in) {
        mods = gem_mods.to_owned();
    } else {
        for gemstat in GEMSTATS_GENERIC.iter() {
            if search_in.ends_with(gemstat.0) {
                mods = gemstat.1.to_owned();
                break;
            }
        }
    }

    if mods.is_empty() {
        return None;
    }

    if let Some(typ_override) = typ_override {
        for m in &mut mods {
            m.typ = typ_override;
        }
    }

    Some(mods)
}
