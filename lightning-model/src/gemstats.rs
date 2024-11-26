use crate::build::StatId;
use crate::data::gem::GemTag;
use crate::modifier::{Mod, Type};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    // Note: Mod::amount is filled in later automatically
    pub static ref GEMSTATS: HashMap<&'static str, Vec<Mod>> = {
        let mut map = HashMap::default();

        map.insert("damage_+%", vec![
            Mod { stat: StatId::Damage, typ: Type::Inc, ..Default::default() },
        ]);
        map.insert("physical_damage_+%", vec![
            Mod { stat: StatId::PhysicalDamage, typ: Type::Inc, ..Default::default() },
        ]);
        map.insert("melee_physical_damage_+%", vec![
            Mod { stat: StatId::PhysicalDamage, typ: Type::Inc, tags: hset![GemTag::Melee], ..Default::default() },
        ]);
        map.insert("base_cast_speed_+%", vec![
            Mod { stat: StatId::CastSpeed, typ: Type::Inc, ..Default::default() },
        ]);
        map.insert("spell_minimum_base_fire_damage", vec![
            Mod { stat: StatId::MinFireDamage, typ: Type::Base, tags: hset![GemTag::Spell], ..Default::default() },
        ]);
        map.insert("spell_maximum_base_fire_damage", vec![
            Mod { stat: StatId::MaxFireDamage, typ: Type::Base, tags: hset![GemTag::Spell], ..Default::default() },
        ]);
        map.insert("base_skill_area_of_effect_+%", vec![
            Mod { stat: StatId::AreaOfEffect, typ: Type::Inc, tags: hset![], ..Default::default() },
        ]);

        // (Awakened) Melee physical damage support
        map.insert("support_melee_physical_damage_attack_speed_+%_final", vec![
            Mod { stat: StatId::AttackSpeed, typ: Type::More, tags: hset![GemTag::Attack], ..Default::default() },
        ]);
        map.insert("support_melee_physical_damage_+%_final", vec![
            Mod { stat: StatId::PhysicalDamage, typ: Type::More, tags: hset![GemTag::Melee], ..Default::default() },
        ]);
        // (Awakened) Brutality Support
        map.insert("support_brutality_physical_damage_+%_final", vec![
            Mod { stat: StatId::PhysicalDamage, typ: Type::More, tags: hset![], ..Default::default() },
        ]);
        // Pulverise Support
        map.insert("support_pulverise_melee_area_damage_+%_final", vec![
            Mod { stat: StatId::Damage, typ: Type::More, tags: hset![GemTag::Melee, GemTag::Area], ..Default::default() },
        ]);
        // Concentrated Effect Support
        map.insert("support_concentrated_effect_skill_area_of_effect_+%_final", vec![
            Mod { stat: StatId::AreaOfEffect, typ: Type::More, ..Default::default() },
        ]);
        map.insert("support_area_concentrate_area_damage_+%_final", vec![
            Mod { stat: StatId::Damage, typ: Type::More, tags: hset![GemTag::Area], ..Default::default() },
        ]);

        map
    };
}
