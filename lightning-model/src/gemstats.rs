//use crate::gem::Tag;
use crate::gem::GemTag;
use crate::modifier::{Mod, Type};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    // Note: Mod::amount is filled in later automatically
    pub static ref GEMSTATS: HashMap<&'static str, Vec<Mod>> = {
        let mut map = HashMap::default();

        map.insert("damage_+%", vec![
            Mod { stat: "damage".to_string(), typ: Type::Inc, ..Default::default() },
        ]);
        map.insert("base_cast_speed_+%", vec![
            Mod { stat: "cast speed".to_string(), typ: Type::Inc, ..Default::default() },
        ]);
        map.insert("spell_minimum_base_fire_damage", vec![
            Mod { stat: "fire minimum damage".to_string(), typ: Type::Base, tags: hset![GemTag::Spell], ..Default::default() },
        ]);
        map.insert("spell_maximum_base_fire_damage", vec![
            Mod { stat: "fire maximum damage".to_string(), typ: Type::Base, tags: hset![GemTag::Spell], ..Default::default() },
        ]);
        map.insert("support_concentrated_effect_skill_area_of_effect_+%_final", vec![
            Mod { stat: "area of effect".to_string(), typ: Type::More, ..Default::default() },
        ]);
        map.insert("support_area_concentrate_area_damage_+%_final", vec![
            Mod { stat: "damage".to_string(), typ: Type::More, tags: hset![GemTag::Area], ..Default::default() },
        ]);

        map
    };
}
