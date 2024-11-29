use crate::build::stat::{Stat, StatId, Stats};
use crate::build::{self, property, Build, Slot};
use crate::data::default_monster_stats::MonsterStats;
use crate::data::gem::GemTag;
use crate::data::DamageType;
use crate::gem::Gem;
use crate::item::Item;
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

fn calc_dmg_crit_accuracy(damage: i64, crit_chance: i64, crit_multi: i64, chance_to_hit: i64) -> i64 {
    let effective_crit_chance = (crit_chance * chance_to_hit) / 100;
    let damage_crit = (damage * chance_to_hit * effective_crit_chance * crit_multi) / 100000000;
    let damage_noncrit = (damage * chance_to_hit * (10000 - effective_crit_chance)) / 1000000;
    damage_crit + damage_noncrit
}

struct DamageGroup {
    stat_id: StatId,
    min_id: StatId,
    max_id: StatId,
    damage_type: DamageType,
}

impl DamageGroup {
    fn new(stat_id: StatId, min_id: StatId, max_id: StatId, damage_type: DamageType) -> Self {
        DamageGroup {
            stat_id,
            min_id,
            max_id,
            damage_type,
        }
    }
}

fn calc_weapon_average_dmg(stats: &Stats, weapon: &Item, active_gem: &Gem, dmg_stat: &Stat, slot: Slot, dg: &DamageGroup) -> i64 {
    let dmg_stat_dt = stats.stat(dg.stat_id);
    let added_min_stat = stats.stat(dg.min_id);
    let added_max_stat = stats.stat(dg.max_id);
    //let mut avg = 0;
    // TODO: with physical damage, each weapon should be compared independently regarding
    // enemy armour, not added together.

    if let Some((min_item, max_item)) = weapon.calc_dmg(dg.damage_type) {
        let mut stat_min = dmg_stat_dt.with_weapon(weapon.data().item_class);
        let mut stat_max = dmg_stat_dt.with_weapon(weapon.data().item_class);
        stat_min.adjust(Type::Base, min_item, &Mod { amount: min_item, typ: Type::Base, source: Source::Item(slot), ..Default::default() });
        stat_max.adjust(Type::Base, max_item, &Mod { amount: max_item, typ: Type::Base, source: Source::Item(slot), ..Default::default() });
        stat_min.assimilate(dmg_stat);
        stat_max.assimilate(dmg_stat);

        let mut added_min_stat = added_min_stat.with_weapon(weapon.data().item_class);
        let mut added_max_stat = added_max_stat.with_weapon(weapon.data().item_class);
        added_min_stat.assimilate(dmg_stat);
        added_max_stat.assimilate(dmg_stat);
        let mut added_damage = (added_min_stat.val() + added_max_stat.val()) / 2;
        if let Some(added_effectiveness) = active_gem.added_effectiveness() {
            added_damage = (added_damage * (100 + added_effectiveness)) / 100;
        }

        let mut base_damage = (stat_min.val() + stat_max.val()) / 2;
        if let Some(damage_multiplier) = active_gem.damage_multiplier() {
            base_damage = (base_damage * (10000 + damage_multiplier)) / 10000;
        }

        return base_damage + added_damage;
    }
    0
}

fn calc_crit_chance_weapon(stats: &Stats, weapon: &Item) -> i64 {
    let mut crit_chance_stat = stats.stat(StatId::CriticalStrikeChance).to_owned();
    if let Some(crit_chance) = weapon.crit_chance() {
        crit_chance_stat.adjust_mod(&Mod { typ: Type::Base, amount: crit_chance, ..Default::default() });
    }
    crit_chance_stat.val()
}

fn calc_chance_hit_weapon(stats: &Stats, monster_stats: &Stats, weapon: &Item) -> i64 {
    let mut chance_to_hit_stat = stats.stat(StatId::ChanceToHit).to_owned();
    let mut accuracy_stat = stats.stat(StatId::AccuracyRating).to_owned();
    accuracy_stat.assimilate(&weapon.accuracy());
    let accuracy = accuracy_stat.val() as f32;
    let monster_evasion = monster_stats.val(StatId::EvasionRating) as f32;
    let chance_to_hit_from_accuracy = ((((1.25 * accuracy) / (accuracy + (monster_evasion * 0.2).powf(0.9))) * 100.0) as i64).clamp(0, 100);
    chance_to_hit_stat.adjust_mod(&Mod { amount: chance_to_hit_from_accuracy, typ: Type::Base, ..Default::default()});
    chance_to_hit_stat.val()
}

pub fn calc_gem<'a>(build: &Build, support_gems: impl Iterator<Item = &'a Gem>, active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();

    let tags = &active_gem.data().tags;
    let mut damage = vec![];

    let mut mods = build.calc_mods(true);
    mods.extend(active_gem.calc_mods());
    for support_gem in support_gems {
        mods.extend(support_gem.calc_mods());
    }

    let stats = build.calc_stats(&mods, tags);

    let monster_build = Build::new_player();
    let monster_mods = monster_build.calc_mods_monster(build.property_int(property::Int::Level).min(83));
    let monster_stats = monster_build.calc_stats(&monster_mods, &hset![]);

    let damage_constants = [
        DamageGroup::new(StatId::PhysicalDamage, StatId::MinPhysicalDamage, StatId::MaxPhysicalDamage, DamageType::Physical),
        DamageGroup::new(StatId::FireDamage, StatId::MinFireDamage, StatId::MaxFireDamage, DamageType::Fire),
    ];

    let crit_multi = stats.val(StatId::CriticalStrikeMultiplier);
    ret.insert("Crit Multi", crit_multi);

    let dmg_stat = stats.stat(StatId::Damage);
    if tags.contains(&GemTag::Attack) {
        for slot in [Slot::Weapon, Slot::Offhand] {
            if let Some(weapon) = build.equipment.get(&slot) {
                let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                if weapon_restrictions.is_empty() || weapon_restrictions.contains(&weapon.data().item_class) {
                    let chance_to_hit = calc_chance_hit_weapon(&stats, &monster_stats, weapon);
                    let crit_chance = calc_crit_chance_weapon(&stats, weapon);

                    if slot == Slot::Weapon {
                        ret.insert("Chance to Hit (MH)", chance_to_hit);
                        ret.insert("Crit Chance (MH)", crit_chance);
                    } else {
                        ret.insert("Chance to Hit (OH)", chance_to_hit);
                        ret.insert("Crit Chance (OH)", crit_chance);
                    }

                    for dt in &damage_constants {
                        let avg_damage = calc_weapon_average_dmg(&stats, weapon, active_gem, dmg_stat, slot, dt);
                        if avg_damage > 0 {
                            damage.push(calc_dmg_crit_accuracy(avg_damage, crit_chance, crit_multi, chance_to_hit));
                        }
                    }
                }
            }
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

    let total_damage: i64 = damage.iter().sum();

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
    ret.insert("Fire Resistance", stats.val(StatId::FireResistance));
    ret.insert("Maximum Fire Resistance", stats.val(StatId::MaximumFireResistance));
    ret.insert("Cold Resistance", stats.val(StatId::ColdResistance));
    ret.insert("Maximum Cold Resistance", stats.val(StatId::MaximumColdResistance));
    ret.insert("Lightning Resistance", stats.val(StatId::LightningResistance));
    ret.insert("Maximum Lightning Resistance", stats.val(StatId::MaximumLightningResistance));
    ret.insert("Chaos Resistance", stats.val(StatId::ChaosResistance));
    ret.insert("Maximum Chaos Resistance", stats.val(StatId::MaximumChaosResistance));
    ret.insert("Strength", stats.val(StatId::Strength));
    ret.insert("Dexterity", stats.val(StatId::Dexterity));
    ret.insert("Intelligence", stats.val(StatId::Intelligence));
    ret.insert("Armour", stats.val(StatId::Armour));
    ret.insert("Evasion", stats.val(StatId::EvasionRating));
    ret.insert("Energy Shield", stats.val(StatId::MaximumEnergyShield));

    let mut life_regen = stats.stat(StatId::LifeRegeneration).to_owned();
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
