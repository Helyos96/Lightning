use crate::build::stat::{Stat, StatId, Stats};
use crate::build::{self, property, Build, Slot};
use crate::data::default_monster_stats::MonsterStats;
use crate::data::gem::GemTag;
use crate::data::{DamageGroup, DamageType, DAMAGE_GROUPS};
use crate::gem::Gem;
use crate::item::Item;
use crate::modifier::{Mod, Source, Type};
use rustc_hash::FxHashMap;

enum DamageSource {
    Slot(Slot),
    Gem,
}

struct DamageInstanceType {
    typ: DamageType,
    amount: i64,
    chance_to_hit: i64,
    crit_chance: i64,
}

struct DamageInstance {
    source: DamageSource,
    instance_type: Vec<DamageInstanceType>,
}

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

fn calc_average_dmg(stats: &Stats, active_gem: &Gem, mut base_min: i64, mut base_max: i64, mut added_min: i64, mut added_max: i64, dg: &DamageGroup) -> i64 {
    if let Some(damage_multiplier) = active_gem.damage_multiplier() {
        base_min = (base_min * (10000 + damage_multiplier)) / 10000;
        base_max = (base_max * (10000 + damage_multiplier)) / 10000;
    }

    if let Some(added_effectiveness) = active_gem.added_effectiveness() {
        added_min = (added_min * (100 + added_effectiveness)) / 100;
        added_max = (added_max * (100 + added_effectiveness)) / 100;
    }

    // These stats are like "10% more maximum physical attack damage"
    let mut stat_min_dt = stats.stat(dg.min_id).clone();
    let mut stat_max_dt = stats.stat(dg.max_id).clone();
    stat_min_dt.adjust_mod(&Mod { typ: Type::Base, amount: base_min + added_min, ..Default::default() });
    stat_max_dt.adjust_mod(&Mod { typ: Type::Base, amount: base_max + added_max, ..Default::default() });

    (stat_min_dt.val() + stat_max_dt.val()) / 2
}

fn calc_weapon_average_dmg(stats: &Stats, weapon: &Item, active_gem: &Gem, slot: Slot, dg: &DamageGroup) -> i64 {
    if let Some((min_item, max_item)) = weapon.calc_dmg(dg.damage_type) {
        let item_class = Some(weapon.data().item_class);
        let added_min_stat = stats.stat(dg.added_min_id).with_weapon(item_class);
        let added_max_stat = stats.stat(dg.added_max_id).with_weapon(item_class);
        let average = calc_average_dmg(stats, active_gem, min_item, max_item, added_min_stat.val(), added_max_stat.val(), dg);
        let dmg_stat_dt = stats.stat(dg.stat_id).with_weapon(item_class);
        let mut dmg_stat = stats.stat(StatId::Damage).with_weapon(item_class);
        dmg_stat.assimilate(&dmg_stat_dt);
        dmg_stat.adjust(Type::Base, average, &Mod { amount: average, typ: Type::Base, source: Source::Item(slot), ..Default::default() });
        return dmg_stat.val();
    }
    0
}

fn calc_spell_average_dmg(stats: &Stats, active_gem: &Gem, dg: &DamageGroup) -> i64 {
    // e.g Added Fire Damage
    let added_min_stat = stats.stat(dg.added_min_id).with_weapon(None);
    let added_max_stat = stats.stat(dg.added_max_id).with_weapon(None);
    // Base damage usually from spells
    let base_min_stat = stats.stat(dg.base_min_id).with_weapon(None);
    let base_max_stat = stats.stat(dg.base_max_id).with_weapon(None);
    let average = calc_average_dmg(stats, active_gem, base_min_stat.val(), base_max_stat.val(), added_min_stat.val(), added_max_stat.val(), dg);
    let dmg_stat_dt = stats.stat(dg.stat_id).with_weapon(None);
    let mut dmg_stat = stats.stat(StatId::Damage).with_weapon(None);
    dmg_stat.assimilate(&dmg_stat_dt);
    dmg_stat.adjust(Type::Base, average, &Mod { amount: average, typ: Type::Base, ..Default::default() });
    return dmg_stat.val();
}

fn calc_crit_chance(stats: &Stats, crit_chance: Option<i64>) -> i64 {
    let mut crit_chance_stat = stats.stat(StatId::CriticalStrikeChance).to_owned();
    if let Some(crit_chance) = crit_chance {
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

    let crit_multi = stats.val(StatId::CriticalStrikeMultiplier);

    let mut damage_instances = vec![];

    if tags.contains(&GemTag::Attack) {
        for slot in [Slot::Weapon, Slot::Offhand] {
            if let Some(weapon) = build.get_equipped(slot) {
                let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                if weapon_restrictions.is_empty() || weapon_restrictions.contains(&weapon.data().item_class) {
                    let chance_to_hit = calc_chance_hit_weapon(&stats, &monster_stats, weapon);
                    let crit_chance = calc_crit_chance(&stats, weapon.crit_chance());

                    if crit_chance > 0 {
                        if slot == Slot::Weapon {
                            ret.insert("Chance to Hit (MH)", chance_to_hit);
                            ret.insert("Crit Chance (MH)", crit_chance);
                        } else {
                            ret.insert("Chance to Hit (OH)", chance_to_hit);
                            ret.insert("Crit Chance (OH)", crit_chance);
                        }
                    }

                    let mut dmg_inst = DamageInstance {
                        source: DamageSource::Slot(slot),
                        instance_type: vec![],
                    };
                    for dg in &DAMAGE_GROUPS {
                        let avg_damage = calc_weapon_average_dmg(&stats, weapon, active_gem, slot, dg);
                        if avg_damage > 0 {
                            dmg_inst.instance_type.push(DamageInstanceType {
                                typ: dg.damage_type,
                                amount: avg_damage,
                                chance_to_hit,
                                crit_chance,
                            });
                            damage.push(calc_dmg_crit_accuracy(avg_damage, crit_chance, crit_multi, chance_to_hit));
                        }
                    }
                    damage_instances.push(dmg_inst);
                }
            }
        }
    } else if tags.contains(&GemTag::Spell) {
        let crit_chance = calc_crit_chance(&stats, active_gem.crit_chance());
        if crit_chance > 0 {
            ret.insert("Crit Chance", crit_chance);
        }
        let mut dmg_inst = DamageInstance {
            source: DamageSource::Gem,
            instance_type: vec![],
        };
        for dg in &DAMAGE_GROUPS {
            let avg_damage = calc_spell_average_dmg(&stats, active_gem, dg);
            if avg_damage > 0 {
                dmg_inst.instance_type.push(DamageInstanceType {
                    typ: dg.damage_type,
                    amount: avg_damage,
                    chance_to_hit: 100,
                    crit_chance,
                });
                damage.push(calc_dmg_crit_accuracy(avg_damage, crit_chance, crit_multi, 100));
            }
        }
    }

    if ret.contains_key("Crit Chance") || ret.contains_key("Crit Chance (MH)") || ret.contains_key("Crit Chance (OH)") {
        ret.insert("Crit Multi", crit_multi);
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
                if let Some(weapon) = build.get_equipped(slot) {
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
    life_regen.assimilate(stats.stat(StatId::LifeRegenerationRate));
    ret.insert("Life Regeneration", life_regen.val(),);

    ret
}
