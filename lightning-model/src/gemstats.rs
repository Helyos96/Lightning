use crate::build::{property, Defence, Slot};
use crate::build::stat::StatId;
use crate::data::gem::{ActiveSkillType, GemTag};
use crate::modifier::{Condition, Mod, ModFlag, Mutation, Type};
use crate::stackvec;
use rustc_hash::FxHashMap;
use enumflags2::{make_bitflags as flags, BitFlags};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
// Order is important, end-of-string match is performed in-order
    static ref GEMSTATS_GENERIC: Vec<(&'static str, Vec<Mod>)> = vec![
        ("spell_minimum_base_physical_damage", vec![
            Mod::stat(StatId::BaseMinPhysicalDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_maximum_base_physical_damage", vec![
            Mod::stat(StatId::BaseMaxPhysicalDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_minimum_base_fire_damage", vec![
            Mod::stat(StatId::BaseMinFireDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_maximum_base_fire_damage", vec![
            Mod::stat(StatId::BaseMaxFireDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_minimum_base_lightning_damage", vec![
            Mod::stat(StatId::BaseMinLightningDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_maximum_base_lightning_damage", vec![
            Mod::stat(StatId::BaseMaxLightningDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_minimum_base_cold_damage", vec![
            Mod::stat(StatId::BaseMinColdDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_maximum_base_cold_damage", vec![
            Mod::stat(StatId::BaseMaxColdDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_minimum_base_chaos_damage", vec![
            Mod::stat(StatId::BaseMinChaosDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("spell_maximum_base_chaos_damage", vec![
            Mod::stat(StatId::BaseMaxChaosDamage, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("local_minimum_added_physical_damage", vec![
            Mod::stat(StatId::BaseMinPhysicalDamage, Type::Base, 0),
        ]),
        ("local_maximum_added_physical_damage", vec![
            Mod::stat(StatId::BaseMaxPhysicalDamage, Type::Base, 0),
        ]),
        ("local_minimum_added_fire_damage", vec![
            Mod::stat(StatId::BaseMinFireDamage, Type::Base, 0),
        ]),
        ("local_maximum_added_fire_damage", vec![
            Mod::stat(StatId::BaseMaxFireDamage, Type::Base, 0),
        ]),
        ("local_minimum_added_cold_damage", vec![
            Mod::stat(StatId::BaseMinColdDamage, Type::Base, 0),
        ]),
        ("local_maximum_added_cold_damage", vec![
            Mod::stat(StatId::BaseMaxColdDamage, Type::Base, 0),
        ]),
        ("local_minimum_added_lightning_damage", vec![
            Mod::stat(StatId::BaseMinLightningDamage, Type::Base, 0),
        ]),
        ("local_maximum_added_lightning_damage", vec![
            Mod::stat(StatId::BaseMaxLightningDamage, Type::Base, 0),
        ]),
        ("local_minimum_added_chaos_damage", vec![
            Mod::stat(StatId::BaseMinChaosDamage, Type::Base, 0),
        ]),
        ("local_maximum_added_chaos_damage", vec![
            Mod::stat(StatId::BaseMaxChaosDamage, Type::Base, 0),
        ]),
        ("minimum_added_physical_damage_per_15_shield_armour_and_evasion_rating", vec![
            Mod::stat(StatId::AddedMinPhysicalDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::{Armour | Evasion})))]),
        ]),
        ("maximum_added_physical_damage_per_15_shield_armour_and_evasion_rating", vec![
            Mod::stat(StatId::AddedMaxPhysicalDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::{Armour | Evasion})))]),
        ]),
        ("minimum_added_physical_damage_per_15_shield_armour", vec![
            Mod::stat(StatId::AddedMinPhysicalDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Armour)))]),
        ]),
        ("maximum_added_physical_damage_per_15_shield_armour", vec![
            Mod::stat(StatId::AddedMaxPhysicalDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Armour)))]),
        ]),
        ("minimum_added_fire_damage_per_15_shield_armour", vec![
            Mod::stat(StatId::AddedMinFireDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Armour)))]),
        ]),
        ("maximum_added_fire_damage_per_15_shield_armour", vec![
            Mod::stat(StatId::AddedMaxFireDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Armour)))]),
        ]),
        ("minimum_added_cold_damage_per_15_shield_evasion", vec![
            Mod::stat(StatId::AddedMinColdDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Evasion)))]),
        ]),
        ("maximum_added_cold_damage_per_15_shield_evasion", vec![
            Mod::stat(StatId::AddedMaxColdDamage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((15, Slot::Offhand, flags!(Defence::Evasion)))]),
        ]),
        ("critical_strike_chance_+_per_10_es_on_shield", vec![
            Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierSlotDefences((10, Slot::Offhand, flags!(Defence::EnergyShield)))]),
        ]),
        ("minimum_added_fire_damage", vec![
            Mod::stat(StatId::AddedMinFireDamage, Type::Base, 0),
        ]),
        ("maximum_added_fire_damage", vec![
            Mod::stat(StatId::AddedMaxFireDamage, Type::Base, 0),
        ]),
        ("minimum_added_lightning_damage", vec![
            Mod::stat(StatId::AddedMinLightningDamage, Type::Base, 0),
        ]),
        ("maximum_added_lightning_damage", vec![
            Mod::stat(StatId::AddedMaxLightningDamage, Type::Base, 0),
        ]),
        ("minimum_added_cold_damage", vec![
            Mod::stat(StatId::AddedMinColdDamage, Type::Base, 0),
        ]),
        ("maximum_added_cold_damage", vec![
            Mod::stat(StatId::AddedMaxColdDamage, Type::Base, 0),
        ]),
        ("minimum_added_chaos_damage", vec![
            Mod::stat(StatId::AddedMinChaosDamage, Type::Base, 0),
        ]),
        ("maximum_added_chaos_damage", vec![
            Mod::stat(StatId::AddedMaxChaosDamage, Type::Base, 0),
        ]),
        ("minimum_attack_damage", vec![
            Mod::stat(StatId::MinDamage, Type::Base, 0).with_tags(GemTag::Attack),
        ]),
        ("maximum_attack_damage", vec![
            Mod::stat(StatId::MaxDamage, Type::Base, 0).with_tags(GemTag::Attack),
        ]),
        ("attack_skills_elemental_damage", vec![
            Mod::stat(StatId::FireDamage, Type::Base, 0).with_tags(GemTag::Attack),
            Mod::stat(StatId::ColdDamage, Type::Base, 0).with_tags(GemTag::Attack),
            Mod::stat(StatId::LightningDamage, Type::Base, 0).with_tags(GemTag::Attack),
        ]),
        ("elemental_damage_with_attack_skills", vec![
            Mod::stat(StatId::FireDamage, Type::Base, 0).with_tags(GemTag::Attack),
            Mod::stat(StatId::ColdDamage, Type::Base, 0).with_tags(GemTag::Attack),
            Mod::stat(StatId::LightningDamage, Type::Base, 0).with_tags(GemTag::Attack),
        ]),
        ("elemental_damage", vec![
            Mod::stat(StatId::FireDamage, Type::Base, 0),
            Mod::stat(StatId::ColdDamage, Type::Base, 0),
            Mod::stat(StatId::LightningDamage, Type::Base, 0),
        ]),
        ("reduce_enemy_lightning_resistance", vec![
            Mod::stat(StatId::LightningDamagePen, Type::Base, 0),
        ]),
        ("poison_and_bleeding_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_flags(flags!(ModFlag::{Bleed | Poison})),
        ]),
        ("ailment_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_flags(flags!(ModFlag::{Bleed | Poison})),
        ]),
        ("poison_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_flags(ModFlag::Poison),
        ]),
        ("spell_damage", vec![
            Mod::stat(StatId::SpellDamage, Type::Base, 0),
        ]),
        ("power_charge_on_crit_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_mutations(stackvec![Mutation::MultiplierProperty((1, property::Int::PowerCharges))]),
        ]),
        ("melee_physical_damage", vec![
            Mod::stat(StatId::PhysicalDamage, Type::Base, 0).with_tags(GemTag::Melee).with_flags(ModFlag::Hit),
        ]),
        ("herald_of_purity_physical_damage", vec![
            Mod::stat(StatId::PhysicalDamage, Type::Base, 0).with_flags(ModFlag::Buff),
        ]),
        ("physical_damage", vec![
            Mod::stat(StatId::PhysicalDamage, Type::Base, 0),
        ]),
        ("fire_damage", vec![
            Mod::stat(StatId::FireDamage, Type::Base, 0),
        ]),
        ("lightning_damage", vec![
            Mod::stat(StatId::LightningDamage, Type::Base, 0),
        ]),
        ("cold_damage", vec![
            Mod::stat(StatId::ColdDamage, Type::Base, 0),
        ]),
        ("chaos_damage", vec![
            Mod::stat(StatId::ChaosDamage, Type::Base, 0),
        ]),
        ("melee_area_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(flags!(GemTag::{Melee | Area})).with_flags(ModFlag::Hit),
        ]),
        ("melee_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Melee),
        ]),
        ("area_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Area),
        ]),
        ("trap_and_mine_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Trap),
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Mine),
        ]),
        ("minion_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Minion),
        ]),
        ("totem_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Totem),
        ]),
        ("bleeding_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_flags(ModFlag::Bleed),
        ]),
        ("bleed_on_hit_with_attacks", vec![
            Mod::stat(StatId::ChanceToBleed, Type::Base, 0).with_tags(GemTag::Attack)
        ]),
        ("deal_no_elemental_damage", vec![
            Mod::stat(StatId::FireDamage, Type::More, -100),
            Mod::stat(StatId::ColdDamage, Type::More, -100),
            Mod::stat(StatId::LightningDamage, Type::More, -100),
        ]),
        ("deal_no_chaos_damage", vec![
            Mod::stat(StatId::ChaosDamage, Type::More, -100),
        ]),
        ("dual_wield_attack_speed", vec![
            Mod::stat(StatId::AttackSpeed, Type::Base, 0).with_tags(GemTag::Attack).with_conditions(stackvec![Condition::WhileDualWielding]),
        ]),
        ("dual_wield_damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Hit).with_conditions(stackvec![Condition::WhileDualWielding]),
        ]),
        ("totem_attack_speed", vec![
            Mod::stat(StatId::AttackSpeed, Type::Base, 0).with_tags(flags!(GemTag::{Attack | Totem})),
        ]),
        ("attack_speed", vec![
            Mod::stat(StatId::AttackSpeed, Type::Base, 0).with_tags(GemTag::Attack),
        ]),
        ("base_cast_speed", vec![
            Mod::stat(StatId::CastSpeed, Type::Base, 0).with_tags(GemTag::Spell),
        ]),
        ("skill_area_of_effect", vec![
            Mod::stat(StatId::AreaOfEffect, Type::Base, 0),
        ]),
        ("shock_as_though_damage", vec![
            Mod::stat(StatId::ShockAsThoughDamage, Type::Base, 0),
        ]),
        ("additional_weapon_base_attack_time_ms", vec![
            Mod::stat(StatId::AddedAttackTime, Type::Base, 0),
        ]),
        ("accuracy_rating", vec![
            Mod::stat(StatId::AccuracyRating, Type::Base, 0),
        ]),
        ("skill_buff_grants_critical_strike_chance", vec![
            Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0).with_flags(ModFlag::Aura),
        ]),
        ("critical_strike_chance", vec![
            Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0),
        ]),
        ("critical_strike_multiplier", vec![
            Mod::stat(StatId::CriticalStrikeMultiplier, Type::Base, 0),
        ]),
        ("base_fire_damage_resistance", vec![
            Mod::stat(StatId::FireResistance, Type::Base, 0),
        ]),
        ("damage", vec![
            Mod::stat(StatId::Damage, Type::Base, 0),
        ]),
        ("aura_effect", vec![
            Mod::stat(StatId::AuraEffect, Type::Base, 0),
        ]),
        ("curse_effect", vec![
            Mod::stat(StatId::CurseEffect, Type::Base, 0),
        ]),
        ("damage_while_on_full_life", vec![
            Mod::stat(StatId::Damage, Type::Base, 0).with_conditions(stackvec![Condition::PropertyBool((true, property::Bool::OnFullLife))]),
        ]),
        ("active_skill_gem_level", vec![
            Mod::gem_level(0).with_tags(GemTag::Active_Skill),
        ]),
        ("golem_buff_effect", vec![
            Mod::stat(StatId::BuffEffect, Type::Base, 0).with_tags(GemTag::Golem),
        ]),
    ];

    // HashMap<gemname<HashMap<statname>>>>
    static ref GEMSTATS_PERGEM: FxHashMap<&'static str, FxHashMap<&'static str, Vec<Mod>>> =
    [
        // Gem name = GemData::base_item::display_name
        ("Precision", [
            ("accuracy_rating", vec![
                Mod::stat(StatId::AccuracyRating, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Haste", [
            ("attack_speed", vec![
                Mod::stat(StatId::AttackSpeed, Type::Inc, 0).with_flags(ModFlag::Aura),
            ]),
            ("cast_speed", vec![
                Mod::stat(StatId::CastSpeed, Type::Inc, 0).with_flags(ModFlag::Aura),
            ]),
            ("base_movement_velocity", vec![
                Mod::stat(StatId::MovementSpeed, Type::Inc, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Anger", [
            ("attack_minimum_added_fire_damage", vec![
                Mod::stat(StatId::AddedMinFireDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Aura),
            ]),
            ("attack_maximum_added_fire_damage", vec![
                Mod::stat(StatId::AddedMaxFireDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Aura),
            ]),
            ("spell_minimum_added_fire_damage", vec![
                Mod::stat(StatId::AddedMinFireDamage, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Aura),
            ]),
            ("spell_maximum_added_fire_damage", vec![
                Mod::stat(StatId::AddedMaxFireDamage, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Wrath", [
            ("attack_minimum_added_lightning_damage", vec![
                Mod::stat(StatId::AddedMinLightningDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Aura),
            ]),
            ("attack_maximum_added_lightning_damage", vec![
                Mod::stat(StatId::AddedMaxLightningDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Aura),
            ]),
            ("wrath_aura_spell_lightning_damage", vec![
                Mod::stat(StatId::LightningDamage, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Tempest Shield", [
            ("shield_spell_block", vec![
                Mod::stat(StatId::ChanceToBlockSpellDamage, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Blood Rage", [
            ("attack_speed", vec![
                Mod::stat(StatId::AttackSpeed, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Purity of Fire", [
            ("base_fire_damage_resistance", vec![
                Mod::stat(StatId::FireResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("base_maximum_fire_damage_resistance", vec![
                Mod::stat(StatId::MaximumFireResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Purity of Cold", [ // Fixed copy-paste typo from "Purity of Fire"
            ("base_cold_damage_resistance", vec![
                Mod::stat(StatId::ColdResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("base_maximum_cold_damage_resistance", vec![
                Mod::stat(StatId::MaximumColdResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Purity of Lightning", [
            ("base_lightning_damage_resistance", vec![
                Mod::stat(StatId::LightningResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("base_maximum_lightning_damage_resistance", vec![
                Mod::stat(StatId::MaximumLightningResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Purity of Elements", [
            ("base_resist_all_elements", vec![
                Mod::stat(StatId::FireResistance, Type::Base, 0).with_flags(ModFlag::Aura),
                Mod::stat(StatId::ColdResistance, Type::Base, 0).with_flags(ModFlag::Aura),
                Mod::stat(StatId::LightningResistance, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Discipline", [
            ("base_maximum_energy_shield", vec![
                Mod::stat(StatId::MaximumEnergyShield, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        /*("Clarity", [
            ("base_mana_regeneration_rate_per_minute", vec![
                // Should be per minute
                Mod::stat(StatId::ManaRegeneration, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),*/
        ("Zealotry", [
            ("spell_damage_aura_spell_damage", vec![
                Mod::stat(StatId::SpellDamage, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("spell_critical_strike_chance", vec![
                Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Determination", [
            ("determination_aura_armour", vec![
                Mod::stat(StatId::Armour, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("base_physical_damage_reduction_rating", vec![
                Mod::stat(StatId::Armour, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Grace", [
            ("base_evasion_rating", vec![
                Mod::stat(StatId::EvasionRating, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
            ("grace_aura_evasion_rating", vec![
                Mod::stat(StatId::EvasionRating, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),
        ("Assassin's Mark", [
            // TODO: enemy actor mods
            ("enemy_additional_critical_strike_chance_against_self", vec![
                Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0).with_flags(ModFlag::Curse),
            ]),
            ("enemy_additional_critical_strike_multiplier_against_self", vec![
                Mod::stat(StatId::CriticalStrikeMultiplier, Type::Base, 0).with_flags(ModFlag::Curse),
            ]),
        ].into_iter().collect()),
       /* ("Vitality", [
            // Should be per minute
            ("base_life_regeneration_rate_per_minute", vec![
                Mod::stat(StatId::LifeRegeneration, Type::Base, 0).with_flags(ModFlag::Aura),
            ]),
        ].into_iter().collect()),*/
        ("Herald of Ice", [
            ("spell_minimum_added_cold_damage", vec![
                Mod::stat(StatId::AddedMinColdDamage, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Buff),
            ]),
            ("spell_maximum_added_cold_damage", vec![
                Mod::stat(StatId::AddedMaxColdDamage, Type::Base, 0).with_tags(GemTag::Spell).with_flags(ModFlag::Buff),
            ]),
            ("attack_minimum_added_cold_damage", vec![
                Mod::stat(StatId::AddedMinColdDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Buff),
            ]),
            ("attack_maximum_added_cold_damage", vec![
                Mod::stat(StatId::AddedMaxColdDamage, Type::Base, 0).with_tags(GemTag::Attack).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Herald of Ash", [
            ("physical_damage_%_to_add_as_fire", vec![
                Mod::stat(StatId::PhysicalAsFireExtra, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Herald of the Hive", [
            ("skill_buff_grants_damage_+%_final_per_herald_skill_affecting_you", vec![
                Mod::stat(StatId::Damage, Type::More, 0).with_flags(ModFlag::Buff).with_mutations(stackvec![Mutation::ForEachActiveSkill(&[ActiveSkillType::Herald])]),
            ]),
        ].into_iter().collect()),
        ("Summon Lightning Golem", [
            ("lightning_golem_grants_attack_and_cast_speed", vec![
                Mod::stat(StatId::AttackSpeed, Type::Base, 0).with_flags(ModFlag::Buff),
                Mod::stat(StatId::CastSpeed, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Summon Flame Golem", [
            ("fire_golem_grants_damage", vec![
                Mod::stat(StatId::Damage, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
            ("fire_golem_grants_area_of_effect", vec![
                Mod::stat(StatId::AreaOfEffect, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Summon Ice Golem", [
            ("ice_golem_grants_critical_strike_chance", vec![
                Mod::stat(StatId::CriticalStrikeChance, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
            ("ice_golem_grants_accuracy_rating", vec![
                Mod::stat(StatId::AccuracyRating, Type::Base, 0).with_flags(ModFlag::Buff),
            ]),
        ].into_iter().collect()),
        ("Hypothermia Support", [
            ("support_hypothermia_damage_+%_vs_chilled_enemies_final", vec![
                // TODO: enemy is chilled property
                Mod::stat(StatId::Damage, Type::More, 0).with_flags(flags!(ModFlag::{Hit | Ailment})),
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
    } else if let Some(substat) = stat.strip_suffix("_from_volatility_support") {
        stat = substat;
    } else if let Some(substat) = stat.strip_suffix("_per_power_charge") {
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
    } else if let Some(ret) = stat.strip_suffix("_%") {
        typ_override = Some(Type::Base);
        ret
    } else if let Some(ret) = stat.strip_suffix("_+") {
        typ_override = Some(Type::Base);
        ret
    } else {
        stat
    };

    if let Some(gemstats) = GEMSTATS_PERGEM.get(gem_basename) &&
       let Some(gem_mods) = gemstats.get(search_in)
    {
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
            if let Some(stat) = m.as_stat_mut() {
                stat.typ = typ_override;
            }
        }
    }

    Some(mods)
}
