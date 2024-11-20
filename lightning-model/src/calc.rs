use crate::build::{self, calc_stat, property, Build, Slot, Stat, StatId};
use crate::gem::{Gem, GemTag};
use crate::modifier::{Mod, Source, Type};
use rustc_hash::FxHashMap;

/*enum Val {
    int(i64),
    int100(i64),
}*/

pub fn compare(a: &FxHashMap<&'static str, i64>, b: &FxHashMap<&'static str, i64>) -> FxHashMap<&'static str, i64> {
    let mut result = FxHashMap::default();
    for key in a.keys().chain(b.keys()) {
        let val_a = a.get(key).unwrap_or(&0);
        let val_b = b.get(key).unwrap_or(&0);
        let delta = val_b - val_a;
        if delta != 0 {
            result.insert(*key, delta);
        }
    }
    result
}

pub fn calc_gem<'a>(build: &Build, support_gems: impl Iterator<Item = &'a Gem>, active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();
    //let display_name = &active_gem.data().base_item.as_ref().unwrap().display_name;

    let tags = &active_gem.data().tags;
    //let active_skill = active_gem.data.active_skill.as_ref().unwrap();
    let mut damage = vec![];

    let mut mods = build.calc_mods(true);
    mods.extend(active_gem.calc_mods());
    for support_gem in support_gems {
        mods.extend(support_gem.calc_mods());
    }

    let stats = build.calc_stats(&mods, tags);
    //dbg!(&stats);

    let damage_constants = [
        (StatId::PhysicalDamage, StatId::MinPhysicalDamage, StatId::MaxPhysicalDamage, "physical"),
        (StatId::FireDamage, StatId::MinFireDamage, StatId::MaxFireDamage, "fire"),
    ];

    let dmg_stat = stats.stat(StatId::Damage);
    for dt in &damage_constants {
        let dmg_stat_dt = stats.stat(dt.0);
        let added_min = stats.stat(dt.1);
        let added_max = stats.stat(dt.2);
        let mut min = 0;
        let mut max = 0;

        if tags.contains(&GemTag::Attack) {
            // TODO: with physical damage, each weapon should be compared independently regarding
            // enemy armour, not added together.
            for slot in [Slot::Weapon, Slot::Offhand] {
                if let Some(weapon) = build.equipment.get(&slot) {
                    let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                    if weapon_restrictions.is_empty() || weapon_restrictions.contains(&weapon.data().item_class) {
                        if let Some((min_item, max_item)) = weapon.calc_dmg(dt.3) {
                            let mut stat_min = dmg_stat_dt.with_weapon(weapon.data().item_class);
                            let mut stat_max = dmg_stat_dt.with_weapon(weapon.data().item_class);
                            stat_min.adjust(Type::Base, min_item, &Mod { amount: min_item, typ: Type::Base, source: Source::Item(slot), ..Default::default() });
                            stat_max.adjust(Type::Base, max_item, &Mod { amount: max_item, typ: Type::Base, source: Source::Item(slot), ..Default::default() });
                            stat_min.assimilate(&dmg_stat);
                            stat_max.assimilate(&dmg_stat);
                            // TODO: added damage can have different effectiveness
                            stat_min.assimilate(&added_min);
                            stat_max.assimilate(&added_max);
                            min += stat_min.val();
                            max += stat_max.val();
                        }
                    }
                }
            }
        }

        if max <= 0 {
            continue;
        }

        if let Some(damage_effectiveness) = active_gem.damage_effectiveness() {
            damage.push((((min + max) / 2) * (100 + damage_effectiveness)) / 100);
        } else {
            damage.push((min + max) / 2);
        }
    }

    let time = {
        if tags.contains(&GemTag::Spell) {
            if let Some(time) = active_gem.data().cast_time {
                stats.stat(StatId::CastSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else if tags.contains(&GemTag::Attack) {
            let mut div = 0;
            let mut time = 0;
            for slot in [Slot::Weapon, Slot::Offhand] {
                if let Some(weapon) = build.equipment.get(&slot) {
                    let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                    if weapon_restrictions.is_empty() || weapon_restrictions.contains(&weapon.data().item_class) {
                        if let Some(item_speed) = weapon.attack_speed() {
                            time += item_speed;
                            div += 1;
                        }
                    }
                }
            }
            if div > 0 {
                time /= div;
                stats.stat(StatId::AttackSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else {
            0
        }
    };

    let monster_build = Build::new_player();
    let monster_mods = monster_build.calc_mods_monster(build.property_int(property::Int::Level).min(83));
    let monster_stats = monster_build.calc_stats(&monster_mods, &hset![]);
    let mut total_damage: i64 = damage.iter().sum();

    if tags.contains(&GemTag::Attack) {
        let accuracy = stats.stat(StatId::AccuracyRating).val() as f32;
        let monster_evasion = monster_stats.stat(StatId::EvasionRating).val() as f32;
        let chance_to_hit_f = ((1.25 * accuracy) / (accuracy + (monster_evasion * 0.2).powf(0.9))).min(1.0);
        total_damage = (total_damage as f32 * chance_to_hit_f) as i64;
        ret.insert("Chance to Hit", (chance_to_hit_f * 100.0) as i64);
    }

    if time != 0 {
        let dps = (total_damage * 1000) / time;
        ret.insert("DPS", dps);
        ret.insert("Speed", time);
    }
    ret
}

pub fn calc_defence(build: &Build) -> FxHashMap<&'static str, i64> {
    let mut ret = FxHashMap::default();
    let mods = build.calc_mods(true);
    let stats = build.calc_stats(&mods, &hset![]);

    ret.insert("Maximum Life", stats.stat(StatId::MaximumLife).val_rounded_up());
    ret.insert("Fire Resistance", stats.stat(StatId::FireResistance).val());
    ret.insert("Maximum Fire Resistance", stats.stat(StatId::MaximumFireResistance).val());
    ret.insert("Cold Resistance", stats.stat(StatId::ColdResistance).val());
    ret.insert("Maximum Cold Resistance", stats.stat(StatId::MaximumColdResistance).val());
    ret.insert("Lightning Resistance", stats.stat(StatId::LightningResistance).val());
    ret.insert("Maximum Lightning Resistance", stats.stat(StatId::MaximumLightningResistance).val());
    ret.insert("Chaos Resistance", stats.stat(StatId::ChaosResistance).val());
    ret.insert("Maximum Chaos Resistance", stats.stat(StatId::MaximumChaosResistance).val());
    ret.insert("Strength", stats.stat(StatId::Strength).val());
    ret.insert("Dexterity", stats.stat(StatId::Dexterity).val());
    ret.insert("Intelligence", stats.stat(StatId::Intelligence).val());
    ret.insert("Armour", stats.stat(StatId::Armour).val());
    ret.insert("Evasion", stats.stat(StatId::EvasionRating).val());
    ret.insert("Energy Shield", stats.stat(StatId::MaximumEnergyShield).val());

    let mut life_regen = stats.stat(StatId::LifeRegeneration);
    let life_regen_pct = stats.stat(StatId::LifeRegenerationPct);
    let adjust_life_regen = Mod {
        stat: StatId::LifeRegeneration,
        typ: Type::Base,
        amount: (life_regen_pct.val() * stats.stat(StatId::MaximumLife).val_rounded_up()) / 10000,
        ..Default::default()
    };
    life_regen.adjust(Type::Base, adjust_life_regen.amount(), &adjust_life_regen);
    life_regen.assimilate(&stats.stat(StatId::LifeRegenerationRate));
    ret.insert("Life Regeneration", life_regen.val(),);

    ret
}
