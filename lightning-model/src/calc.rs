use crate::build::stat::{Stat, StatId, Stats};
use crate::build::{self, property, Build, Slot};
use crate::data::base_item::ItemClass;
use crate::data::default_monster_stats::MonsterStats;
use crate::data::gem::GemTag;
use crate::data::{DamageGroup, DamageType, DAMAGE_GROUPS};
use crate::gem::Gem;
use crate::item::Item;
use crate::modifier::{Mod, ModFlag, Source, Type};
use enumflags2::{BitFlags, make_bitflags};
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

fn calc_min_max_dmg(stats: &Stats, active_gem: &Gem, mut base_min: i64, mut base_max: i64, mut added_min: i64, mut added_max: i64, dg: &DamageGroup) -> (i64, i64) {
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

    (stat_min_dt.val(), stat_max_dt.val())
}

fn calc_average_dmg(stats: &Stats, active_gem: &Gem, base_min: i64, base_max: i64, added_min: i64, added_max: i64, dg: &DamageGroup) -> i64 {
    let (min, max) = calc_min_max_dmg(stats, active_gem, base_min, base_max, added_min, added_max, dg);
    (min + max) / 2
}

fn calc_weapon_average_dmg(stats: &Stats, weapon: &Item, active_gem: &Gem, dg: &DamageGroup) -> i64 {
    if let Some((min_item, max_item)) = weapon.calc_dmg(dg.damage_type) {
        let item_class = Some(weapon.data().item_class);
        let added_min_stat = stats.stat(dg.added_min_id).with_weapon(item_class);
        let added_max_stat = stats.stat(dg.added_max_id).with_weapon(item_class);
        let average = calc_average_dmg(stats, active_gem, min_item, max_item, added_min_stat.val(), added_max_stat.val(), dg);
        let dmg_stat_dt = stats.stat(dg.stat_id).with_weapon(item_class);
        let mut dmg_stat = stats.stat(StatId::Damage).with_weapon(item_class);
        dmg_stat.assimilate(&dmg_stat_dt);
        dmg_stat.adjust(Type::Base, average);
        return dmg_stat.val();
    }
    0
}

fn calc_weapon_max_base_dmg(stats: &Stats, weapon: &Item, active_gem: &Gem, dg: &DamageGroup) -> Option<Stat> {
    if let Some((_, max_item)) = weapon.calc_dmg(dg.damage_type) {
        let item_class = Some(weapon.data().item_class);
        let added_max_stat = stats.stat(dg.added_max_id).with_weapon(item_class);
        let (_, max) = calc_min_max_dmg(stats, active_gem, 0, max_item, 0, added_max_stat.val(), dg);
        let dmg_stat_dt = stats.stat(dg.stat_id).with_weapon(item_class);
        let mut dmg_stat = stats.stat(StatId::Damage).with_weapon(item_class);
        dmg_stat.assimilate(&dmg_stat_dt);
        dmg_stat.adjust(Type::Base, max);
        return Some(dmg_stat);
    }
    None
}

fn calc_weapon_bleed_dmg(stats: &Stats, weapon: &Item, active_gem: &Gem, dg: &DamageGroup) -> i64 {
    if let Some(mut max_dmg) = calc_weapon_max_base_dmg(stats, weapon, active_gem, dg) {
        max_dmg.assimilate(stats.stat(StatId::DamageWithAilments));
        max_dmg.assimilate(stats.stat(StatId::BleedDamage));
        max_dmg.adjust_mod(&Mod { typ: Type::More, amount: -30, ..Default::default() });
        dbg!(&max_dmg);
        return max_dmg.val();
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
    dmg_stat.adjust(Type::Base, average);
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

pub fn calc_gem<'a>(build: &Build, support_gems: &[&Gem], active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();

    // convert HashSet<GemTag> into BitFlags
    let tags = active_gem.data().tags.iter().copied().map(BitFlags::from).fold(BitFlags::empty(), |acc, flag| acc | flag);
    let mut damage = vec![];

    let mut mods = build.calc_mods(true);
    mods.extend(active_gem.calc_mods());
    for support_gem in support_gems {
        if support_gem.can_support(active_gem) {
            mods.extend(support_gem.calc_mods());
        }
    }

    let stats = build.calc_stats(&mods, tags, make_bitflags!(ModFlag::Hit));
    let stats_no_modflags = build.calc_stats(&mods, tags, BitFlags::EMPTY);

    let monster_build = Build::new_player();
    let monster_mods = monster_build.calc_mods_monster(build.property_int(property::Int::Level).min(83));
    let monster_stats = monster_build.calc_stats(&monster_mods, BitFlags::empty(), BitFlags::EMPTY);

    let crit_multi = stats.val(StatId::CriticalStrikeMultiplier);

    let mut damage_instances = vec![];
    let mut bleed_dps = 0;

    if tags.contains(GemTag::Attack) {
        let bleed_chance = stats.val(StatId::ChanceToBleed);

        for slot in [Slot::Weapon, Slot::Offhand] {
            if let Some(weapon) = build.get_equipped(slot) {
                let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                if !weapon_restrictions.is_empty() && !weapon_restrictions.contains(&weapon.data().item_class) {
                    continue;
                }
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
                    let avg_damage = calc_weapon_average_dmg(&stats, weapon, active_gem, dg);
                    if avg_damage > 0 {
                        dmg_inst.instance_type.push(DamageInstanceType {
                            typ: dg.damage_type,
                            amount: avg_damage,
                            chance_to_hit,
                            crit_chance,
                        });
                        damage.push(calc_dmg_crit_accuracy(avg_damage, crit_chance, crit_multi, chance_to_hit));
                    }
                    if dg.damage_type == DamageType::Physical && bleed_chance > 0 {
                        let local_bleed_dps = calc_weapon_bleed_dmg(&stats_no_modflags, weapon, active_gem, dg);
                        if local_bleed_dps > bleed_dps {
                            bleed_dps = local_bleed_dps;
                        }
                    }
                }
                damage_instances.push(dmg_inst);
            }
        }
    } else if tags.contains(GemTag::Spell) {
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

    ret.insert("Bleed DPS", bleed_dps);

    if ret.contains_key("Crit Chance") || ret.contains_key("Crit Chance (MH)") || ret.contains_key("Crit Chance (OH)") {
        ret.insert("Crit Multi", crit_multi);
    }

    let time = {
        if tags.contains(GemTag::Spell) {
            if let Some(time) = active_gem.data().cast_time {
                stats.stat(StatId::CastSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else if tags.contains(GemTag::Attack) {
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
                time += stats.stat(StatId::AddedAttackTime).val();
                stats.stat(StatId::AttackSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else {
            0
        }
    };

    let mut mana_cost_stat = stats.stat(StatId::ManaCost).to_owned();
    mana_cost_stat.assimilate(stats.stat(StatId::Cost));
    ret.insert("Mana Cost", mana_cost_stat.val());

    let average_damage: i64 = damage.iter().sum();
    ret.insert("Average Damage", average_damage);

    if time != 0 {
        let dps = (average_damage * 1000) / time;
        ret.insert("DPS", dps);
        ret.insert("Speed", time);
    }
    ret
}

pub fn calc_defence(build: &Build) -> (FxHashMap<&'static str, i64>, Stats) {
    let mut ret = FxHashMap::default();
    let mods = build.calc_mods(true);
    let stats = build.calc_stats(&mods, BitFlags::EMPTY, BitFlags::EMPTY);

    let max_life = stats.stat(StatId::MaximumLife).val_ceil();
    let max_mana = stats.stat(StatId::MaximumMana).val_ceil();
    ret.insert("Maximum Life", max_life);
    ret.insert("Maximum Mana", max_mana);
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
    ret.insert("Spell Suppression", stats.val(StatId::ChanceToSuppressSpellDamage));

    if let Some(offhand) = build.get_equipped(Slot::Offhand) {
        if offhand.data().item_class == ItemClass::Shield {
            ret.insert("Block", stats.val(StatId::ChanceToBlockAttackDamage));
            ret.insert("Spell Block", stats.val(StatId::ChanceToBlockSpellDamage));
        }
    }

    let mut life_regen = stats.stat(StatId::LifeRegeneration).to_owned();
    life_regen.adjust(Type::Base, (stats.stat(StatId::LifeRegenerationPct).val() * max_life) / 100);
    life_regen.adjust(Type::More, stats.stat(StatId::LifeRegenerationRate).val());
    ret.insert("Life Regeneration", life_regen.val() / 100);

    let mut mana_regen = stats.stat(StatId::ManaRegeneration).to_owned();
    mana_regen.adjust(Type::Base, (stats.stat(StatId::ManaRegenerationPct).val() * max_mana) / 10000);
    mana_regen.adjust(Type::More, (stats.stat(StatId::ManaRegenerationRate).val() * max_mana) / 10000);
    ret.insert("Mana Regeneration", mana_regen.val());

    (ret, stats)
}
