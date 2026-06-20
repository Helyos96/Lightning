use crate::build::evaluator::{Evaluator, ModDB};
use crate::build::stat::{Stat, StatId, Stats};
use crate::build::{self, Build, GemLink, Slot, property};
use crate::data::base_item::ItemClass;
use crate::data::gem::GemTag;
use crate::data::{DamageGroup, DamageType, DAMAGE_GROUPS};
use crate::gem::Gem;
use crate::item::Item;
use crate::modifier::{Mod, ModFlag, Mutation, Source, Type};
use enumflags2::{BitFlags, make_bitflags};
use rustc_hash::{FxHashMap, FxHashSet};
use rayon::slice::ParallelSlice;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

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

#[derive(Clone, Copy, Debug)]
struct DamagePortion {
    amount: i64,
    source_types: BitFlags<DamageType>,
}

const CONVERSION_ORDER: [DamageType; 4] = [
    DamageType::Physical,
    DamageType::Lightning,
    DamageType::Cold,
    DamageType::Fire,
];

fn conversion_stat_id(from_dt: DamageType, to_dt: DamageType) -> Option<StatId> {
    match (from_dt, to_dt) {
        (DamageType::Physical, DamageType::Lightning) => Some(StatId::PhysicalToLightningConversion),
        (DamageType::Physical, DamageType::Cold)      => Some(StatId::PhysicalToColdConversion),
        (DamageType::Physical, DamageType::Fire)      => Some(StatId::PhysicalToFireConversion),
        (DamageType::Physical, DamageType::Chaos)     => Some(StatId::PhysicalToChaosConversion),
        (DamageType::Lightning, DamageType::Cold)     => Some(StatId::LightningToColdConversion),
        (DamageType::Lightning, DamageType::Fire)     => Some(StatId::LightningToFireConversion),
        (DamageType::Lightning, DamageType::Chaos)    => Some(StatId::LightningToChaosConversion),
        (DamageType::Cold, DamageType::Fire)          => Some(StatId::ColdToFireConversion),
        (DamageType::Cold, DamageType::Chaos)         => Some(StatId::ColdToChaosConversion),
        (DamageType::Fire, DamageType::Chaos)         => Some(StatId::FireToChaosConversion),
        _ => None,
    }
}

fn extra_damage_stat_id(from_dt: DamageType, to_dt: DamageType) -> Option<StatId> {
    match (from_dt, to_dt) {
        (DamageType::Physical, DamageType::Lightning) => Some(StatId::PhysicalAsLightningExtra),
        (DamageType::Physical, DamageType::Cold)      => Some(StatId::PhysicalAsColdExtra),
        (DamageType::Physical, DamageType::Fire)      => Some(StatId::PhysicalAsFireExtra),
        (DamageType::Physical, DamageType::Chaos)     => Some(StatId::PhysicalAsChaosExtra),
        (DamageType::Lightning, DamageType::Cold)     => Some(StatId::LightningAsColdExtra),
        (DamageType::Lightning, DamageType::Fire)     => Some(StatId::LightningAsFireExtra),
        (DamageType::Lightning, DamageType::Chaos)    => Some(StatId::LightningAsChaosExtra),
        (DamageType::Cold, DamageType::Fire)          => Some(StatId::ColdAsFireExtra),
        (DamageType::Cold, DamageType::Chaos)         => Some(StatId::ColdAsChaosExtra),
        (DamageType::Fire, DamageType::Chaos)         => Some(StatId::FireAsChaosExtra),
        _ => None,
    }
}

fn conversion_targets(from_dt: DamageType) -> &'static [DamageType] {
    match from_dt {
        DamageType::Physical  => &[DamageType::Lightning, DamageType::Cold, DamageType::Fire, DamageType::Chaos],
        DamageType::Lightning => &[DamageType::Cold, DamageType::Fire, DamageType::Chaos],
        DamageType::Cold      => &[DamageType::Fire, DamageType::Chaos],
        DamageType::Fire      => &[DamageType::Chaos],
        _ => &[],
    }
}

/// Apply the damage conversion chain, tracking which source types contributed
/// to each portion.
fn apply_conversion(eval: &mut Evaluator, base_damages: &[i64; 5]) -> [Vec<DamagePortion>; 5] {
    let mut portions: [Vec<DamagePortion>; 5] = Default::default();

    for i in 0..5 {
        if base_damages[i] > 0 {
            portions[i].push(DamagePortion { amount: base_damages[i], source_types: BitFlags::from(DAMAGE_GROUPS[i].damage_type) });
        }
    }

    for &from_dt in &CONVERSION_ORDER {
        let targets = conversion_targets(from_dt);
        let from_idx = from_dt.as_index();

        for &to_dt in targets {
            if let Some(stat_id) = extra_damage_stat_id(from_dt, to_dt) {
                let extra_pct = eval.eval_stat(stat_id).val();
                if extra_pct > 0 {
                    let to_idx = to_dt.as_index();
                    for portion in &portions[from_idx].clone() {
                        portions[to_idx].push(DamagePortion {
                            amount: (portion.amount * extra_pct) / 100,
                            source_types: portion.source_types | DAMAGE_GROUPS[to_idx].damage_type,
                        });
                    }
                }
            }
        }

        let total_conv: i64 = targets.iter().filter_map(|&to_dt| {
            conversion_stat_id(from_dt, to_dt).map(|sid| eval.eval_stat(sid).val())
        }).sum();

        if total_conv > 0 {
            let remaining_pct = (100 - total_conv.min(100)).max(0);
            let current_portions = std::mem::take(&mut portions[from_idx]);

            for portion in &current_portions {
                if remaining_pct > 0 {
                    portions[from_idx].push(DamagePortion {
                        amount: (portion.amount * remaining_pct) / 100,
                        source_types: portion.source_types,
                    });
                }
                for &to_dt in targets {
                    let to_idx = to_dt.as_index();
                    if let Some(stat_id) = conversion_stat_id(from_dt, to_dt) {
                        let mut conv_pct = eval.eval_stat(stat_id).val();
                        if total_conv > 100 {
                            conv_pct = (conv_pct * 100) / total_conv;
                        }
                        if conv_pct > 0 {
                            portions[to_idx].push(DamagePortion {
                                amount: (portion.amount * conv_pct) / 100,
                                source_types: portion.source_types | DAMAGE_GROUPS[to_idx].damage_type,
                            });
                        }
                    }
                }
            }
        }
    }

    portions
}

/// Apply inc/more modifiers to converted damage portions.
/// Each portion gets modifiers from all damage types in its conversion path.
fn apply_damage_mods_portions(portions: &[Vec<DamagePortion>; 5], eval: &mut Evaluator, weapon: Option<ItemClass>, is_spell: bool) -> [i64; 5] {
    let mut result = [0i64; 5];
    let mut generic = eval.eval_stat(StatId::Damage).with_weapon(weapon);
    if is_spell {
        generic.assimilate(eval.eval_stat(StatId::SpellDamage));
    }

    for (dg_idx, dg_portions) in portions.iter().enumerate() {
        for portion in dg_portions {
            let mut inc = generic.inc;
            let mut more = generic.more;

            for type_idx in 0..5 {
                if portion.source_types.contains(DAMAGE_GROUPS[type_idx].damage_type) {
                    let type_stat = eval.eval_stat(DAMAGE_GROUPS[type_idx].stat_id).with_weapon(weapon);
                    inc += type_stat.inc;
                    more = (more * type_stat.more) / 100;
                }
            }

            result[dg_idx] += (portion.amount * (100 + inc) * more) / 10000;
        }
    }

    result
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

fn calc_min_max_dmg(eval: &mut Evaluator, active_gem: &Gem, mut base_min: i64, mut base_max: i64, mut added_min: i64, mut added_max: i64, dg: &DamageGroup, extra_level: i32) -> (i64, i64) {
    if let Some(damage_multiplier) = active_gem.damage_multiplier(extra_level) {
        base_min = (base_min * (10000 + damage_multiplier)) / 10000;
        base_max = (base_max * (10000 + damage_multiplier)) / 10000;
    }

    if let Some(added_effectiveness) = active_gem.added_effectiveness(extra_level) {
        added_min = (added_min * (100 + added_effectiveness)) / 100;
        added_max = (added_max * (100 + added_effectiveness)) / 100;
    }

    // These stats are like "10% more maximum physical attack damage"
    let mut stat_min_dt = eval.eval_stat(dg.min_id).clone();
    let mut stat_max_dt = eval.eval_stat(dg.max_id).clone();
    stat_min_dt.assimilate(eval.eval_stat(StatId::MinDamage));
    stat_max_dt.assimilate(eval.eval_stat(StatId::MaxDamage));
    stat_min_dt.adjust(Type::Base, base_min + added_min);
    stat_max_dt.adjust(Type::Base, base_max + added_max);

    (stat_min_dt.val(), stat_max_dt.val())
}

fn calc_average_dmg(eval: &mut Evaluator, active_gem: &Gem, base_min: i64, base_max: i64, added_min: i64, added_max: i64, dg: &DamageGroup, extra_level: i32) -> i64 {
    let (min, max) = calc_min_max_dmg(eval, active_gem, base_min, base_max, added_min, added_max, dg, extra_level);
    (min + max) / 2
}


fn calc_max_base_dmg(eval: &mut Evaluator, active_gem: &Gem, base_max: i64, item_class: Option<ItemClass>, dg: &DamageGroup, extra_level: i32) -> Option<Stat> {
    if base_max <= 0 {
        return None;
    }
    let added_max_stat = eval.eval_stat(dg.added_max_id).with_weapon(item_class);
    let (_, max) = calc_min_max_dmg(eval, active_gem, 0, base_max, 0, added_max_stat.val(), dg, extra_level);
    let dmg_stat_dt = eval.eval_stat(dg.stat_id).with_weapon(item_class);
    let mut dmg_stat = eval.eval_stat(StatId::Damage).with_weapon(item_class);
    dmg_stat.assimilate(&dmg_stat_dt);
    dmg_stat.adjust(Type::Base, max);
    Some(dmg_stat)
}

/// Damaging ailments inflicted by critical strikes gain a bonus +50% DoT
/// multiplier, additive with other DoT multipliers. Returns the ailment
/// damage averaged over crit and non-crit hits (crit_chance is per 10000).
fn avg_ailment_dmg_crit(base: i64, dot_multi: i64, crit_chance: i64) -> i64 {
    let noncrit = (base * (100 + dot_multi)) / 100;
    let crit = (base * (100 + dot_multi + 50)) / 100;
    (noncrit * (10000 - crit_chance) + crit * crit_chance) / 10000
}

fn calc_bleed_dmg(eval: &mut Evaluator, active_gem: &Gem, base_max: i64, item_class: Option<ItemClass>, dg: &DamageGroup, extra_level: i32, crit_chance: i64) -> i64 {
    if let Some(mut max_dmg) = calc_max_base_dmg(eval, active_gem, base_max, item_class, dg, extra_level) {
        let mut dot_multi = eval.eval_stat(StatId::DotMultiplier).to_owned();
        dot_multi.assimilate(eval.eval_stat(StatId::PhysicalDotMultiplier));
        max_dmg.adjust_mod(&Mod::stat(StatId::Damage, Type::More, -30).with_source(Source::Custom("Bleeds deal 70%")));
        return avg_ailment_dmg_crit(max_dmg.val(), dot_multi.val(), crit_chance);
    }
    0
}


/// Poison deals chaos damage over time. Its base DPS is 30% of the hit's
/// combined flat physical and chaos damage (after conversion, before hit modifiers).
fn calc_poison_dps(eval: &mut Evaluator, portions: &[Vec<DamagePortion>; 5], weapon: Option<ItemClass>, crit_chance: i64) -> i64 {
    let mut generic = eval.eval_stat(StatId::Damage).with_weapon(weapon);
    generic.assimilate(eval.eval_stat(StatId::DamageOverTime));
    generic.assimilate(eval.eval_stat(StatId::ChaosDamageOverTime));

    let mut dot_multi = eval.eval_stat(StatId::DotMultiplier).to_owned();
    dot_multi.assimilate(eval.eval_stat(StatId::ChaosDotMultiplier));

    let mut dps = 0i64;
    for dt in [DamageType::Physical, DamageType::Chaos] {
        for portion in &portions[dt.as_index()] {
            let mut inc = generic.inc;
            let mut more = generic.more;

            let source_types = portion.source_types | DamageType::Chaos;
            for type_idx in 0..5 {
                if source_types.contains(DAMAGE_GROUPS[type_idx].damage_type) {
                    let type_stat = eval.eval_stat(DAMAGE_GROUPS[type_idx].stat_id).with_weapon(weapon);
                    inc += type_stat.inc;
                    more = (more * type_stat.more) / 100;
                }
            }

            dps += ((portion.amount * 30) / 100) * (100 + inc) * more / 10000;
        }
    }

    avg_ailment_dmg_crit(dps, dot_multi.val(), crit_chance)
}

fn double_damage_effect(eval: &mut Evaluator) -> i64 {
    let triple = eval.eval_stat(StatId::ChanceToDealTripleDamage).val().clamp(0, 100);
    let double = eval.eval_stat(StatId::ChanceToDealDoubleDamage).val().clamp(0, 100);
    let effective_double = double.min(100 - triple);
    10000 + (2 * triple + effective_double) * 100
}

fn calc_crit_chance(eval: &mut Evaluator, crit_chance: Option<i64>) -> i64 {
    let mut crit_chance_stat = eval.eval_stat(StatId::CriticalStrikeChance).to_owned();
    if let Some(crit_chance) = crit_chance {
        crit_chance_stat.adjust(Type::Base, crit_chance);
    }
    crit_chance_stat.val().min(10000)
}

fn calc_chance_to_hit(eval: &mut Evaluator, monster_stats: &Stats, weapon: Option<&Item>) -> i64 {
    let mut chance_to_hit_stat = eval.eval_stat(StatId::ChanceToHit).to_owned();
    let mut accuracy_stat = eval.eval_stat(StatId::AccuracyRating).to_owned();
    if let Some(weapon) = weapon {
        accuracy_stat.assimilate(&weapon.accuracy());
    }
    let accuracy = accuracy_stat.val() as f32;
    let monster_evasion = monster_stats.val(StatId::EvasionRating) as f32;
    let chance_to_hit_from_accuracy = ((((1.25 * accuracy) / (accuracy + (monster_evasion * 0.2).powf(0.9))) * 100.0) as i64).clamp(0, 100);
    chance_to_hit_stat.adjust(Type::Base, chance_to_hit_from_accuracy);
    chance_to_hit_stat.val()
}

fn physical_damage_reduction_armour(amount: i64, armour: i64, pdr: i64) -> i64 {
    let pdr_from_armour = (armour * 100) / (armour + 5 * amount);
    pdr + pdr_from_armour
}

/// Impale records 10% of the hit's pre-mitigation physical damage, scaled by
/// impale effect, and reflects it on each subsequent hit while stacks last.
/// Returns the average damage stored per stack, scaled by impale chance.
fn calc_impale_stored_dmg(eval: &mut Evaluator, phys_damage: i64, impale_chance: i64) -> i64 {
    eval.eval_stat(StatId::ImpaleEffect).val_custom((phys_damage * 10 * impale_chance) / 10000)
}

/// Hypothesis: weaponless attacks have a 5% base crit chance
const WEAPONLESS_CRIT_CHANCE: i64 = 500;

/// Returns the item in `slot` if the attack skill can use it.
fn attack_item<'a>(build: &'a Build, active_gem: &Gem, slot: Slot) -> Option<&'a Item> {
    let item = build.get_equipped(slot)?;
    let weapon_restrictions = &active_gem.data().active_skill.as_ref()?.weapon_restrictions;
    if weapon_restrictions.is_empty() {
        if !item.data().tags.contains("weapon") {
            return None;
        }
    } else if !weapon_restrictions.contains(&item.data().item_class) {
        return None;
    }
    Some(item)
}

pub fn calc_gem<'a>(build: &Build, link: &GemLink, active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();

    let tags = active_gem.data().tags;
    let mut damage = vec![];
    let mut mods = build.calc_mods(true);

    let mut best_supports: FxHashMap<&str, &Gem> = FxHashMap::default();
    for support_gem in link.support_gems().filter(|gem| gem.enabled) {
        if support_gem.can_support(active_gem) {
            if let Some(existing_gem) = best_supports.get(support_gem.id.as_str()) {
                if existing_gem.level >= support_gem.level {
                    continue;
                }
            }
            best_supports.insert(support_gem.id.as_str(), support_gem);
        }
    }

    // TODO: the way extra_level is computed will ignore Condition::Socketed
    for support_gem in best_supports.values() {
        let extra_level = mods.iter().filter(|m| support_gem.data().tags.contains(m.tags)).flat_map(|m| m.as_gem_level()).sum();
        mods.extend_from_slice(&support_gem.calc_mods(false, extra_level, 0));
    }

    let extra_level = mods.iter().filter(|m| tags.contains(m.tags) && !tags.intersects(m.tags_not)).flat_map(|m| m.as_gem_level()).sum();
    mods.extend_from_slice(&active_gem.calc_mods(false, extra_level, 0));
    let mod_db = ModDB::new(&mods);

    let skill_types = if let Some(active_skill) = &active_gem.data().active_skill {
        &active_skill.types
    } else {
        &FxHashSet::default()
    };
    let mut eval = Evaluator::new(build, &mod_db, tags, make_bitflags!(ModFlag::{Hit | Aura | Buff | Curse}), skill_types, None, link.slot);
    eval.resolve();
    let monster_mods = Build::calc_mods_monster(build.property_int(property::Int::Level).min(83));
    let monster_stats = build::stat::calc_stats(&monster_mods);

    let crit_multi = eval.eval_stat(StatId::CriticalStrikeMultiplier).val();

    let mut damage_instances = vec![];
    let mut bleed_dps = 0;
    let mut poison_dps = 0;
    let mut poison_hit_chance = 100;
    // (stored damage per stack, crit chance, chance to hit) per weapon slot
    let mut impale_sources: Vec<(i64, i64, i64)> = vec![];
    let poison_chance = eval.eval_stat(StatId::ChanceToPoison).val().min(100);

    if tags.contains(GemTag::Attack) {
        let bleed_chance = eval.eval_stat(StatId::ChanceToBleed).val();

        for slot in [Slot::Weapon, Slot::Offhand] {
            if let Some(item) = attack_item(build, active_gem, slot) {
                let mut eval = Evaluator::new(build, &mod_db, tags, make_bitflags!(ModFlag::{Hit | Aura | Buff | Curse}), skill_types, Some(slot), link.slot);
                eval.resolve();
                // Weaponless attacks (e.g. Shield Charge) get their base damage
                // from the gem instead of an equipped weapon
                let is_weapon = item.data().tags.contains("weapon");
                let item_class = if is_weapon { Some(item.data().item_class) } else { None };
                let (chance_to_hit, crit_chance) = if is_weapon {
                    (calc_chance_to_hit(&mut eval, &monster_stats, Some(item)),
                     calc_crit_chance(&mut eval, item.crit_chance()))
                } else {
                    (calc_chance_to_hit(&mut eval, &monster_stats, None),
                     calc_crit_chance(&mut eval, active_gem.crit_chance().or(Some(WEAPONLESS_CRIT_CHANCE))))
                };

                if crit_chance > 0 {
                    if slot == Slot::Weapon {
                        ret.insert("Chance to Hit (MH)", chance_to_hit);
                        ret.insert("Crit Chance (MH)", crit_chance);
                    } else {
                        ret.insert("Chance to Hit (OH)", chance_to_hit);
                        ret.insert("Crit Chance (OH)", crit_chance);
                    }
                }

                let mut base_damages = [0i64; 5];
                for (i, dg) in DAMAGE_GROUPS.iter().enumerate() {
                    let (base_min, base_max) = if is_weapon {
                        item.calc_dmg(dg.damage_type).unwrap_or((0, 0))
                    } else {
                        (eval.eval_stat(dg.base_min_id).with_weapon(None).val(),
                         eval.eval_stat(dg.base_max_id).with_weapon(None).val())
                    };
                    let added_min = eval.eval_stat(dg.added_min_id).with_weapon(item_class).val();
                    let added_max = eval.eval_stat(dg.added_max_id).with_weapon(item_class).val();
                    base_damages[i] = calc_average_dmg(&mut eval, active_gem, base_min, base_max, added_min, added_max, dg, extra_level);
                }

                let portions = apply_conversion(&mut eval, &base_damages);
                let final_damages = apply_damage_mods_portions(&portions, &mut eval, item_class, false);
                let double_damage = double_damage_effect(&mut eval);

                let mut dmg_inst = DamageInstance {
                    source: DamageSource::Slot(slot),
                    instance_type: vec![],
                };
                for (i, dg) in DAMAGE_GROUPS.iter().enumerate() {
                    let mut avg_damage = (final_damages[i] * double_damage) / 10000;
                    if avg_damage <= 0 { continue; }

                    if dg.damage_type == DamageType::Physical {
                        let impale_chance = eval.eval_stat(StatId::ChanceToImpale).val().min(100);
                        if impale_chance > 0 {
                            // Impale records the hit's physical damage before mitigation
                            let stored = calc_impale_stored_dmg(&mut eval, avg_damage, impale_chance);
                            impale_sources.push((stored, crit_chance, chance_to_hit));
                        }
                        let pdr = physical_damage_reduction_armour(avg_damage, monster_stats.val(StatId::Armour), 0);
                        avg_damage = (avg_damage * (100 - pdr)) / 100;
                    }

                    dmg_inst.instance_type.push(DamageInstanceType {
                        typ: dg.damage_type,
                        amount: avg_damage,
                        chance_to_hit,
                        crit_chance,
                    });

                    if let Some(pen_id) = dg.pen_id {
                        avg_damage = (avg_damage * (100 + eval.eval_stat(pen_id).val())) / 100;
                    }

                    damage.push(calc_dmg_crit_accuracy(avg_damage, crit_chance, crit_multi, chance_to_hit));
                }
                damage_instances.push(dmg_inst);

                if bleed_chance > 0 {
                    let mut eval = Evaluator::new(build, &mod_db, tags, make_bitflags!(ModFlag::{Ailment | Bleed | Aura | Buff | Curse}), skill_types, Some(slot), link.slot);
                    eval.resolve();
                    let physical_dg = &DAMAGE_GROUPS[0];
                    let base_max = if is_weapon {
                        item.calc_dmg(physical_dg.damage_type).map_or(0, |(_, max)| max)
                    } else {
                        eval.eval_stat(physical_dg.base_max_id).with_weapon(None).val()
                    };
                    let local_bleed_dps = calc_bleed_dmg(&mut eval, active_gem, base_max, item_class, physical_dg, extra_level, crit_chance);
                    if local_bleed_dps > bleed_dps {
                        bleed_dps = local_bleed_dps;
                    }
                }

                if poison_chance > 0 {
                    let mut eval = Evaluator::new(build, &mod_db, tags, make_bitflags!(ModFlag::{Ailment | Poison | Aura | Buff | Curse}), skill_types, Some(slot), link.slot);
                    eval.resolve();
                    let local_poison_dps = calc_poison_dps(&mut eval, &portions, item_class, crit_chance);
                    if local_poison_dps > poison_dps {
                        poison_dps = local_poison_dps;
                        poison_hit_chance = chance_to_hit;
                    }
                }
            }
        }
    } else if tags.contains(GemTag::Spell) {
        let crit_chance = calc_crit_chance(&mut eval, active_gem.crit_chance());
        if crit_chance > 0 {
            ret.insert("Crit Chance", crit_chance);
        }

        let mut base_damages = [0i64; 5];
        for (i, dg) in DAMAGE_GROUPS.iter().enumerate() {
            let added_min = eval.eval_stat(dg.added_min_id).with_weapon(None).val();
            let added_max = eval.eval_stat(dg.added_max_id).with_weapon(None).val();
            let base_min = eval.eval_stat(dg.base_min_id).with_weapon(None).val();
            let base_max = eval.eval_stat(dg.base_max_id).with_weapon(None).val();
            base_damages[i] = calc_average_dmg(&mut eval, active_gem, base_min, base_max, added_min, added_max, dg, extra_level);
        }

        let portions = apply_conversion(&mut eval, &base_damages);
        let final_damages = apply_damage_mods_portions(&portions, &mut eval, None, true);
        let double_damage = double_damage_effect(&mut eval);

        let mut dmg_inst = DamageInstance {
            source: DamageSource::Gem,
            instance_type: vec![],
        };
        for (i, dg) in DAMAGE_GROUPS.iter().enumerate() {
            let avg_damage = (final_damages[i] * double_damage) / 10000;
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

        if poison_chance > 0 {
            let mut eval = Evaluator::new(build, &mod_db, tags, make_bitflags!(ModFlag::{Ailment | Poison | Aura | Buff | Curse}), skill_types, None, link.slot);
            eval.resolve();
            poison_dps = calc_poison_dps(&mut eval, &portions, None, crit_chance);
        }
    }

    ret.insert("Bleed DPS", bleed_dps);
    let poison_duration = eval.eval_stat(StatId::PoisonDuration).val100();
    ret.insert("Poison Duration", poison_duration);

    if ret.contains_key("Crit Chance") || ret.contains_key("Crit Chance (MH)") || ret.contains_key("Crit Chance (OH)") {
        ret.insert("Crit Multi", crit_multi);
    }

    let time = {
        if tags.contains(GemTag::Spell) {
            if let Some(time) = active_gem.data().cast_time {
                eval.eval_stat(StatId::CastSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else if tags.contains(GemTag::Attack) {
            let mut div = 0;
            let mut time = 0;
            for slot in [Slot::Weapon, Slot::Offhand] {
                if let Some(item) = attack_item(build, active_gem, slot) {
                    if let Some(item_speed) = item.attack_speed() {
                        time += item_speed;
                        div += 1;
                    } else if !item.data().tags.contains("weapon") &&
                        let Some(cast_time) = active_gem.data().cast_time
                    {
                        // TODO: weaponless skills should have their own base attack time but it's missing from gems.json
                        time += cast_time;
                        div += 1;
                    }
                }
            }
            if div > 0 {
                time /= div;
                time += eval.eval_stat(StatId::AddedAttackTime).val();
                eval.eval_stat(StatId::AttackSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else {
            0
        }
    };

    let mut mana_cost_stat = eval.eval_stat(StatId::ManaCost).to_owned();
    mana_cost_stat.assimilate(eval.eval_stat(StatId::Cost));
    ret.insert("Mana Cost", mana_cost_stat.val());

    // TODO: currently we always add up both weapons even for skills where dual weapons alternate
    let average_damage: i64 = damage.iter().sum();
    ret.insert("Average Damage", average_damage);

    if time != 0 {
        let mut dps = (average_damage * 1000) / time;

        if !impale_sources.is_empty() {
            let max_stacks = eval.eval_stat(StatId::MaxImpaleStacks).val();
            let stacks100 = (max_stacks * 100).min(800000 / time);
            let mut impale_damage = 0;
            for &(stored, crit_chance, chance_to_hit) in &impale_sources {
                // Armour mitigates the combined stack damage when it's dealt
                let combined = (stored * stacks100) / 100;
                let pdr = physical_damage_reduction_armour(combined, monster_stats.val(StatId::Armour), 0);
                let mitigated = (combined * (100 - pdr)) / 100;
                impale_damage += calc_dmg_crit_accuracy(mitigated, crit_chance, crit_multi, chance_to_hit);
            }
            let impale_dps = (impale_damage * 1000) / time;
            if impale_dps > 0 {
                ret.insert("Impale DPS", impale_dps);
                dps += impale_dps;
            }
        }

        ret.insert("DPS", dps);
        ret.insert("Speed", time);

        if poison_dps > 0 {
            poison_dps = (poison_dps * poison_duration * poison_hit_chance * poison_chance) / (time * 100 * 10);
            ret.insert("Poison DPS", poison_dps);
        }
    }
    ret
}

pub fn calc_defence(build: &Build) -> (FxHashMap<&'static str, i64>, Stats) {
    let mut ret = FxHashMap::default();
    let mods = build.calc_mods(true);
    let stats = build.calc_stats(&mods, BitFlags::EMPTY, make_bitflags!(ModFlag::{Aura | Buff}));

    let max_life = stats.stat(StatId::MaximumLife).val_rounded();
    let max_mana = stats.stat(StatId::MaximumMana).val_rounded();
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
    ret.insert("Block", stats.val(StatId::ChanceToBlockAttackDamage));
    ret.insert("Spell Block", stats.val(StatId::ChanceToBlockSpellDamage));
    ret.insert("Maximum Block", stats.val(StatId::MaximumChanceToBlockAttackDamage));
    ret.insert("Maximum Spell Block", stats.val(StatId::MaximumChanceToBlockSpellDamage));

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

#[derive(Debug)]
pub struct PowerReport {
    pub nodes_delta: FxHashMap<u32, f32>,
    pub max: f32,
}

impl PowerReport {
    pub fn new_defence(build: &Build, delta_str: &str) -> PowerReport {
        let defence = calc_defence(build).0;

        let nodes_compare: Vec<u32> = build.tree.nodes_data.keys()
            .filter(|node_id| !build.tree.nodes.contains(node_id))
            .copied()
            .collect();

        let results: Vec<(u32, f32)> = nodes_compare.par_iter().map_init(
            || build.clone(),
            |local_build, node_id| {

                local_build.tree.nodes.insert(*node_id);
                local_build.tree.invalidate_modcache();

                let calc = calc_defence(&*local_build).0;
                let delta = *calc.get(delta_str).unwrap_or(&0) as f32 / *defence.get(delta_str).unwrap_or(&0) as f32;

                local_build.tree.nodes.remove(node_id);

                (*node_id, delta)
            }
        ).collect();

        PowerReport {
            nodes_delta: FxHashMap::from_iter(results.into_iter()),
            max: 0.0,
        }
    }

    pub fn new_gem(build: &Build, delta_str: &str, link: &GemLink, active_gem: &Gem) -> PowerReport {
        let offence = calc_gem(build, link, active_gem);

        let nodes_compare: Vec<u32> = build.tree.nodes_data.keys()
            .filter(|node_id| !build.tree.nodes.contains(node_id))
            .copied()
            .collect();

        let results: Vec<(u32, f32)> = nodes_compare.par_iter().map_init(
            || build.clone(),
            |local_build, node_id| {

                local_build.tree.nodes.insert(*node_id);
                local_build.tree.invalidate_modcache();

                let calc = calc_gem(&*local_build, link, active_gem);
                let delta = *calc.get(delta_str).unwrap_or(&0) as f32 / *offence.get(delta_str).unwrap_or(&0) as f32;

                local_build.tree.nodes.remove(node_id);

                (*node_id, delta)
            }
        ).collect();

        PowerReport {
            nodes_delta: FxHashMap::from_iter(results.into_iter()),
            max: 0.0,
        }
    }
}
