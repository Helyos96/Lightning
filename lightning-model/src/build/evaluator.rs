use std::rc::Rc;

use enumflags2::BitFlags;
use rustc_hash::{FxHashMap, FxHashSet};
use lazy_static::lazy_static;
use strum::{EnumCount, IntoEnumIterator};

use crate::{build::{Build, Defence, GemLink, Slot, buff::{BUFF_MODS, Buff}, property, stat::{self, Stat, StatId}}, data::{base_item::Rarity, gem::{ActiveSkillType, GemTag}}, gem::Gem, modifier::{BuildFlag, Condition, Mod, ModEffect, ModFlag, ModStat, Mutation, Source, Type}, stackvec};

lazy_static! {
    static ref STATS_SOURCES: FxHashMap<StatId, &'static [StatId]> = {
        let mut ret: FxHashMap<StatId, &'static [StatId]> = FxHashMap::default();
        ret.insert(StatId::FireResistance, &[StatId::FireResistance, StatId::ElementalResistances]);
        ret.insert(StatId::ColdResistance, &[StatId::ColdResistance, StatId::ElementalResistances]);
        ret.insert(StatId::LightningResistance, &[StatId::LightningResistance, StatId::ElementalResistances]);
        ret.insert(StatId::MaximumFireResistance, &[StatId::MaximumFireResistance, StatId::MaxElementalResistances]);
        ret.insert(StatId::MaximumColdResistance, &[StatId::MaximumColdResistance, StatId::MaxElementalResistances]);
        ret.insert(StatId::MaximumLightningResistance, &[StatId::MaximumLightningResistance, StatId::MaxElementalResistances]);
        ret
    };
}

pub struct ModDB<'a> {
    mods_by_stat: [Vec<&'a Mod>; StatId::COUNT],
    mods_buff: Vec<&'a Mod>,
    mods_gem_level: Vec<&'a Mod>,
    build_flags: FxHashSet<BuildFlag>,
}

#[derive(Default)]
pub struct StatCache {
    pub resolved_stats: FxHashMap<StatId, Stat>,
    evaluating: FxHashSet<StatId>,
}

pub struct EvaluatorCtx<'a> {
    build: &'a Build,
    weapon: Option<Slot>,
    // Slot containing the Gem
    slot: Option<Slot>,
    tags: BitFlags<GemTag>,
    flags: BitFlags<ModFlag>,
    skill_types: &'a FxHashSet<ActiveSkillType>,
    db: &'a ModDB<'a>,
    pub extra_mods: Vec<Mod>,
}

/// Evaluate Stats from a collection of Mods
pub struct Evaluator<'a> {
    pub ctx: EvaluatorCtx<'a>,
    pub cache: StatCache,
}

impl<'a> ModDB<'a> {
    pub fn new(mods: &'a [Mod]) -> Self {
        let mut mods_by_stat = std::array::from_fn(|_| Vec::new());
        let mut mods_buff = vec![];
        let mut mods_gem_level = vec![];
        let mut build_flags = FxHashSet::default();

        for m in mods {
            match m.effect {
                ModEffect::Stat(mstat) => {
                    mods_by_stat[mstat.stat.as_usize()].push(m);
                },
                ModEffect::LevelOfGems(_) => mods_gem_level.push(m),
                ModEffect::Buff(_) => mods_buff.push(m),
                ModEffect::BuildFlag(flag) => { build_flags.insert(flag); },
                _ => {},
            }
        }

        ModDB { mods_by_stat, mods_buff, mods_gem_level, build_flags }
    }
}

impl<'a> Evaluator<'a> {
    pub fn new(build: &'a Build, db: &'a ModDB, tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>, skill_types: &'a FxHashSet<ActiveSkillType>, weapon: Option<Slot>, slot: Option<Slot>) -> Self {
        let ret = Self {
            ctx: EvaluatorCtx {
                build,
                weapon,
                slot,
                tags,
                skill_types,
                flags,
                db,
                extra_mods: Default::default(),
            },
            cache: Default::default(),
        };
        ret
    }

    fn resolve_buffs(&mut self) {
        let mut new_mods = vec![];
        for m in &self.ctx.db.mods_buff {
            let passes_conditions_bor = m.conditions.is_empty() || m.conditions.iter().any(|c| self.ctx.check_condition(&mut self.cache, c, m.source));
            if !passes_conditions_bor {
                continue;
            }
            if let Some(mods) = BUFF_MODS.get(&m.as_buff().unwrap()) {
                for m in mods {
                    new_mods.push(m.to_owned());
                }
            }
        }
        self.ctx.extra_mods.append(&mut new_mods);
    }

    pub fn resolve(&mut self) {
        self.resolve_buffs();
        self.resolve_armour();
        self.resolve_flags_post();
    }

    fn resolve_flags_post(&mut self) {
        for f in &self.ctx.db.build_flags {
            match f {
                BuildFlag::EleMaxResHighest => {
                    let fire = self.ctx.eval_stat(&mut self.cache, StatId::MaximumFireResistance).val();
                    let cold = self.ctx.eval_stat(&mut self.cache, StatId::MaximumColdResistance).val();
                    let lightning = self.ctx.eval_stat(&mut self.cache, StatId::MaximumLightningResistance).val();
                    let max = fire.max(cold).max(lightning);
                    self.cache.resolved_stats.get_mut(&StatId::MaximumFireResistance).unwrap().adjust(Type::Override, max);
                    self.cache.resolved_stats.get_mut(&StatId::MaximumColdResistance).unwrap().adjust(Type::Override, max);
                    self.cache.resolved_stats.get_mut(&StatId::MaximumLightningResistance).unwrap().adjust(Type::Override, max);
                }
                _ => {}
            }
        }
    }

    pub fn resolve_stats(&mut self) {
        for stat_id in StatId::iter() {
            self.ctx.eval_stat(&mut self.cache, stat_id);
        }
    }

    fn resolve_armour(&mut self) {
        for (slot, idx) in &self.ctx.build.equipment {
            let item = &self.ctx.build.inventory[*idx];
            let defence = item.calc_defence();

            if defence.armour.val() != 0 {
                let val = self.ctx.eval_stat(&mut self.cache, StatId::Armour).val_custom(defence.armour.val());
                self.cache.resolved_stats.entry(StatId::Armour).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
            if defence.energy_shield.val() != 0 {
                if self.ctx.db.build_flags.contains(&BuildFlag::ItemsGrantLifeInsteadES) {
                    self.ctx.extra_mods.push(Mod::stat(StatId::MaximumLife, Type::Base, defence.energy_shield.val()).with_source(Source::Item(*slot)));
                } else {
                    let val = self.ctx.eval_stat(&mut self.cache, StatId::MaximumEnergyShield).val_custom(defence.energy_shield.val());
                    self.cache.resolved_stats.entry(StatId::MaximumEnergyShield).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
                }
            }
            if defence.evasion.val() != 0 {
                let val = self.ctx.eval_stat(&mut self.cache, StatId::EvasionRating).val_custom(defence.evasion.val());
                self.cache.resolved_stats.entry(StatId::EvasionRating).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
            if defence.block_chance.val() != 0 {
                let val = self.ctx.eval_stat(&mut self.cache, StatId::ChanceToBlockAttackDamage).val_custom(defence.block_chance.val());
                self.cache.resolved_stats.entry(StatId::ChanceToBlockAttackDamage).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
        }
    }

    pub fn gem_level_extra(&self, tags: BitFlags<GemTag>) -> i32 {
        self.ctx.db.mods_gem_level.iter().filter(|m| tags.contains(m.tags) && !tags.intersects(m.tags_not)).flat_map(|m| m.as_gem_level()).sum()
    }

    pub fn eval_stat(&mut self, stat_id: StatId) -> &Stat {
        self.ctx.eval_stat(&mut self.cache, stat_id)
    }

    /*pub fn gem_quality_extra(&self) -> i32 {
        self.mods.iter().filter(|m| tags.contains(m.tags)).flat_map(|m| m.as_gem_quality()).sum()
    }*/
}

impl<'a> EvaluatorCtx<'a> {
    pub fn get_stat_val(&self, cache: &mut StatCache, stat_id: StatId) -> i64 {
        self.eval_stat(cache, stat_id).val()
    }

    pub fn get_stat_mult(&self, cache: &mut StatCache, stat_id: StatId) -> i64 {
        self.eval_stat(cache, stat_id).mult()
    }

    pub fn eval_stat<'b>(&self, cache: &'b mut StatCache, stat_id: StatId) -> &'b Stat {
        if !cache.resolved_stats.contains_key(&stat_id) {
            if !cache.evaluating.insert(stat_id) {
                eprintln!("Warning: Circular dependency detected for stat: {:?}", stat_id);
                cache.resolved_stats.insert(stat_id, Stat::default());
                return cache.resolved_stats.get(&stat_id).unwrap();
            }

            let mut current_stat = Stat::default();
            let stat_sources = STATS_SOURCES.get(&stat_id).copied().unwrap_or(std::slice::from_ref(&stat_id));
            let mods = stat_sources.iter().flat_map(|&source_id| self.db.mods_by_stat[source_id as usize].iter().copied());

            for m in mods.chain(&self.extra_mods).filter(|m| {
                if let Some(mstat) = m.as_stat() && stat_sources.contains(&mstat.stat) &&
                   (m.flags.is_empty() || self.flags.intersects(m.flags)) &&
                   (m.weapons.is_empty() || self.build.is_holding(&m.weapons)) &&
                   self.tags.contains(m.tags) && !self.tags.intersects(m.tags_not) &&
                   m.skill_types.iter().all(|st| self.skill_types.contains(st))
                {
                    true
                } else {
                    false
                }
            })
            {
                let mut m = m.to_owned();
                let passes_conditions_bor = m.conditions.is_empty() || m.conditions.iter().any(|c| self.check_condition(cache, c, m.source));
                if !passes_conditions_bor {
                    continue;
                }

                let source = m.source;
                if let Source::Item(Slot::TreeJewel(idx)) = source {
                    let item = self.build.get_equipped(Slot::TreeJewel(idx)).unwrap();
                    if item.corrupted && item.rarity == Rarity::Magic {
                        let corrupted_magic_effect = self.eval_stat(cache, StatId::CorruptedMagicJewelEffect).mult();
                        if corrupted_magic_effect != 10000 {
                            m = m.with_mutations(stackvec![Mutation::CustomMult(corrupted_magic_effect)])
                        }
                    }
                }
                if let Some(stat) = m.as_stat_mut() && !stat.mutations.is_empty() {
                    self.apply_mutations(cache, stat, source);
                }

                if let Source::Item(Slot::Flask(idx)) = m.source && let Some(stat) = m.as_stat_mut() {
                    let effect_local = self.build.get_equipped(Slot::Flask(idx)).unwrap().effect();
                    let mut flask_effect = self.eval_stat(cache, StatId::FlaskEffect).clone();
                    flask_effect.assimilate(&effect_local);
                    let new_amount = (stat.final_amount() * flask_effect.mult()) / 10000;
                    stat.revised_amount = Some(new_amount);
                }

                current_stat.adjust_mod_move(m);
            }

            cache.evaluating.remove(&stat_id);
            cache.resolved_stats.insert(stat_id, current_stat);
        }

        cache.resolved_stats.get(&stat_id).unwrap()
    }

    fn property_int_stats(&self, cache: &mut StatCache, p: property::Int) -> i64 {
        let mut min = match property::int_data(p).min {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => self.get_stat_val(cache, s),
        };
        let max = match property::int_data(p).max {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => self.get_stat_val(cache, s),
        };

        if self.build.is_property_int_maxed(p) {
            return max;
        }
        min = min.min(max);
        self.build.property_int(p).clamp(min, max)
    }

    fn apply_mutations(&self, cache: &mut StatCache, m: &mut ModStat, source: Source) {
        let mut amount = m.amount;
        let mut up_to = i64::MAX;
        for f in &m.mutations {
            match f {
                Mutation::MultiplierProperty(mutation) => {
                    amount = (amount * self.property_int_stats(cache, mutation.1)) / mutation.0;
                },
                Mutation::MultiplierStat(mutation) => {
                    amount = (amount * self.get_stat_val(cache, mutation.1)) / mutation.0;
                },
                Mutation::MultiplierStatLowest(mutation) => {
                    let mut lowest = None;
                    for stat_id in mutation.1 {
                        let val = self.get_stat_val(cache, *stat_id);
                        if lowest.is_none() || val < lowest.unwrap() {
                            lowest = Some(val);
                        }
                    }
                    amount = lowest.map_or(0, |l| (amount * l) / mutation.0);
                },
                Mutation::MultiplierSlotDefences((per, slot, defences_flags)) => {
                    let mut def_amount = 0;
                    if let Some(item) = self.build.get_equipped(*slot) {
                        let defences = item.calc_defence();
                        for defence in defences_flags.iter() {
                            match defence {
                                Defence::Armour => def_amount += defences.armour.val(),
                                Defence::Evasion => def_amount += defences.evasion.val(),
                                Defence::EnergyShield => def_amount += defences.energy_shield.val(),
                                Defence::Block => def_amount += defences.block_chance.val(),
                            }
                        }
                    }
                    amount = (amount * def_amount) / per;
                },
                Mutation::StatPct((pct, stat_id)) => {
                    amount = (self.get_stat_val(cache, *stat_id) * pct) / 100;
                },
                Mutation::UpTo(mutation) => {
                    up_to = *mutation;
                },
                Mutation::IncreasedEffect(effect) => {
                    amount = (amount * (100 + effect)) / 100;
                },
                Mutation::MultiplierQuality(per) => {
                    if let Source::Item(slot) = source {
                        let qual = self.build.get_equipped(slot).unwrap().quality;
                        amount = (amount * qual) / per;
                    } else {
                        eprintln!("Warning: applying MultiplierQuality for non-item source");
                        dbg!(&m);
                    }
                },
                Mutation::StatIncPct(pct, stat_id) => {
                    amount = (self.eval_stat(cache, *stat_id).inc * pct) / 100;
                },
                Mutation::MultiplierOvercap(per, stat_a, stat_b) => {
                    let delta = (self.eval_stat(cache, *stat_a).val() - self.eval_stat(cache, *stat_b).val()).max(0);
                    amount = (amount * delta) / per;
                },
                Mutation::StatMultExtra(stat_id, extra) => {
                    // Multiplies by a Stat's multiplier with an added extra Inc
                    // Typical use is AuraEffect where the extra is the sum of support gems' increased aura effect
                    let mut stat = self.eval_stat(cache, *stat_id).clone();
                    stat.adjust(Type::Inc, *extra);
                    amount = stat.val_custom(amount);
                },
                Mutation::ForEachActiveSkill(types) => {
                    let count = self.build.gem_links.iter().flat_map(|link| link.active_gems()).filter(|gem| {
                        if gem.enabled && let Some(active_skill) = &gem.data().active_skill {
                            types.iter().any(|t| active_skill.types.contains(t))
                        } else {
                            false
                        }
                    }).count() as i64;
                    amount = amount * count;
                },
                Mutation::CustomMult(mult) => {
                    amount = (amount * mult) / 10000;
                },
            }
        }

        m.revised_amount = Some(amount.min(up_to));
    }

    fn check_condition(&self, cache: &mut StatCache, c: &Condition, source: Source) -> bool {
        match c {
            Condition::GreaterEqualProperty(mutation) => {
                if self.property_int_stats(cache, mutation.1) < mutation.0 { return false; }
            },
            Condition::LesserEqualProperty(mutation) => {
                if self.property_int_stats(cache, mutation.1) > mutation.0 { return false; }
            },
            Condition::GreaterEqualStat(mutation) => {
                if self.get_stat_val(cache, mutation.1) < mutation.0 { return false; }
            },
            Condition::LesserEqualStat(mutation) => {
                if self.get_stat_val(cache, mutation.1) > mutation.0 { return false; }
            },
            Condition::PropertyBool(mutation) => {
                if self.build.property_bool(mutation.1) != mutation.0 { return false; }
            },
            Condition::WhileWielding(weapons) => {
                if !self.build.is_holding(weapons) { return false; }
            },
            Condition::SlotsHaveDefence((defence, slots)) => {
                for slot in *slots {
                    if let Some(item) = self.build.get_equipped(*slot) {
                        let calc_defence = item.calc_defence();
                        let val = match defence {
                            Defence::Armour => calc_defence.armour.val(),
                            Defence::Evasion => calc_defence.evasion.val(),
                            Defence::EnergyShield => calc_defence.energy_shield.val(),
                            Defence::Block => calc_defence.block_chance.val(),
                        };
                        if val == 0 { return false; }
                    } else {
                        return false;
                    }
                }
            },
            Condition::SlotLesserEqualStats((slot, amount, stat_ids)) => {
                if let Some(item) = self.build.get_equipped(*slot) {
                    for stat_id in *stat_ids {
                        let item_mods = item.calc_nonlocal_mods();
                        let stat = stat::calc_stat(*stat_id, &item_mods);
                        if stat.val() > *amount { return false; }
                    }
                }
            },
            Condition::GreaterEqualMasteryAllocated((mastery_str, count)) => {
                let count_tree = self.build.tree.masteries.keys().filter(|node_id| {
                    &self.build.tree.nodes_data[*node_id].name == *mastery_str
                }).count() as u32;
                if count_tree < *count {
                    return false;
                }
            }
            Condition::WhileDualWielding => {
                if let Some(mainhand) = self.build.get_equipped(Slot::Weapon) &&
                   let Some(offhand) = self.build.get_equipped(Slot::Offhand) {
                    if !mainhand.data().tags.contains("weapon") || !offhand.data().tags.contains("weapon") {
                        return false;
                    }
                } else {
                    return false;
                }
            },
            Condition::WhileDualWieldingItems(items) => {
                if let Some(mainhand) = self.build.get_equipped(Slot::Weapon) &&
                   let Some(offhand) = self.build.get_equipped(Slot::Offhand) {
                    if !items.contains(mainhand.data().item_class) || !items.contains(offhand.data().item_class) {
                        return false;
                    }
                } else {
                    return false;
                }
            },
            Condition::WithThisWeapon => {
                if self.weapon.is_none() {
                    return false;
                }
                if !matches!(source, Source::Item(slot) if Some(slot) == self.weapon) {
                    return false;
                }
            },
            Condition::NoFlaskActive => {
                if self.build.flask_enabled.iter().filter(|idx| self.build.get_equipped(Slot::Flask(**idx)).is_some()).count() > 0 {
                    return false;
                }
            },
            Condition::AffectedByGemTag(tag) => {
                if !self.build.gem_links.iter().flat_map(|link| link.active_gems()).filter(|gem| gem.enabled).find(|gem| gem.data().tags.contains(*tag)).is_some() {
                    return false;
                }
            },
            Condition::Socketed => {
                if !matches!(source, Source::Item(slot) if Some(slot) == self.slot) {
                    return false;
                }
            },
            Condition::WhileUsing(name) => {
                if self.build.gem_links.iter().flat_map(|link| link.active_gems()).find(|gem| gem.enabled && &gem.data().display_name() == name).is_none() {
                    return false;
                }
            }
        }
        true
    }
}