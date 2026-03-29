use crate::build::stat::StatId;
use crate::data::gem::GemTag;
use crate::modifier::{Mod, Type};
use enumflags2::{make_bitflags as flags, BitFlags};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    // Order is important, end-of-string match is performed in-order
    static ref GEMSTATS: Vec<(&'static str, Vec<Mod>)> = vec![
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
        ("melee_physical_damage", vec![
            Mod { stat: StatId::PhysicalDamage, tags: flags!(GemTag::Melee), ..Default::default() },
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
            Mod { stat: StatId::Damage, tags: flags!(GemTag::{Melee | Area}), ..Default::default() },
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
        ("damage", vec![
            Mod { stat: StatId::Damage, ..Default::default() },
        ]),
    ];
}

pub fn match_gemstat(stat: &str) -> Option<Vec<Mod>> {
    let mut typ_override = None;
    let search_in = if let Some(ret) = stat.strip_suffix("_+%_final") {
        typ_override = Some(Type::More);
        ret
    } else if let Some(ret) = stat.strip_suffix("_+%") {
        typ_override = Some(Type::Inc);
        ret
    } else {
        stat
    };

    for gemstat in GEMSTATS.iter() {
        if search_in.ends_with(gemstat.0) {
            let mut mods = gemstat.1.to_owned();
            if let Some(typ_override) = typ_override {
                for m in &mut mods {
                    m.typ = typ_override;
                }
            }
            return Some(mods);
        }
    }

    None
}
