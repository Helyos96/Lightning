use enumflags2::BitFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{build::{Build, Defence, GemLink, Slot, property, stat::{self, Stat, StatId}}, data::gem::{ActiveSkillType, GemTag}, gem::Gem, modifier::{BuildFlag, Condition, Mod, ModEffect, ModFlag, ModStat, Mutation, Source, Type}};

/// Evaluate Stats from a collection of Mods
pub struct Evaluator<'a> {
    build: &'a Build,
    slot: Option<Slot>,
    tags: BitFlags<GemTag>,
    flags: BitFlags<ModFlag>,
    build_flags: FxHashSet<BuildFlag>,
    pub mods_by_stat: FxHashMap<StatId, Vec<Mod>>,
    other_mods: Vec<Mod>,
    pub resolved_stats: FxHashMap<StatId, Stat>,
    evaluating: FxHashSet<StatId>,
}

impl<'a> Evaluator<'a> {
    pub fn new(build: &'a Build, mods: &'a [Mod], tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>, slot: Option<Slot>) -> Self {
        let mut mods_by_stat: FxHashMap<StatId, Vec<Mod>> = FxHashMap::default();
        let mut other_mods = vec![];

        for m in mods.iter().filter(|m| {
            tags.contains(m.tags) &&
            (m.flags.is_empty() || flags.intersects(m.flags)) &&
            (m.weapons.is_empty() || build.is_holding(&m.weapons))
        }) {
            if let Some(mstat) = m.as_stat() {
                mods_by_stat.entry(mstat.stat).or_default().push(m.to_owned());
            } else {
                other_mods.push(m.to_owned());
            }
        }

        Self {
            build,
            slot,
            tags,
            flags,
            build_flags: FxHashSet::from(other_mods.iter().filter_map(|m| m.as_build_flag()).copied().collect()),
            mods_by_stat,
            other_mods,
            resolved_stats: FxHashMap::default(),
            evaluating: FxHashSet::default(),
        }
    }

    pub fn calc_buffs_auras_mods(&mut self) -> Vec<Mod> {
        // Find best unique active auras
        let mut best_gems: FxHashMap<&str, (&Gem, &GemLink)> = FxHashMap::default();
        for link in &self.build.gem_links {
            for active_gem in link.active_gems().filter(|gem| {
                let types = &gem.data().active_skill.as_ref().unwrap().types;
                gem.enabled && (types.contains(&ActiveSkillType::Aura) || types.contains(&ActiveSkillType::Buff))
            })
            {
                if let Some((existing_gem, _)) = best_gems.get(active_gem.id.as_str()) {
                    if existing_gem.level >= active_gem.level {
                        continue;
                    }
                }
                best_gems.insert(active_gem.id.as_str(), (active_gem, link));
            }
        }

        let mut ret = vec![];
        for (gem, link) in best_gems.values() {
            // extra gem level from support gems
            let mut extra_level = self.gem_level_extra(gem.data().tags);
            // extra aura effect from support gems
            let mut extra_aura_effect = 0;
            let mut best_supports: FxHashMap<&str, &Gem> = FxHashMap::default();

            // Find best unique support gems in link
            for support_gem in link.support_gems() {
                if support_gem.can_support(gem) {
                    if let Some(existing_gem) = best_supports.get(support_gem.id.as_str()) {
                        if existing_gem.level >= support_gem.level {
                            continue;
                        }
                    }
                    best_supports.insert(support_gem.id.as_str(), support_gem);
                }
            }

            for support in best_supports.values().filter(|gem| gem.enabled) {
                for m in support.calc_mods(false, self.gem_level_extra(support.data().tags), 0).iter() {
                    if let Some(level) = m.as_gem_level() {
                        extra_level += level;
                    } else if let Some(mstat) = m.as_stat() && mstat.stat == StatId::AuraEffect {
                        extra_aura_effect += mstat.amount;
                    }
                }
            }

            let mods = gem.calc_mods(true, extra_level, 0);
            if gem.data().active_skill.as_ref().unwrap().types.contains(&ActiveSkillType::Aura) {
                for mut m in mods.iter().cloned() {
                    if let Some(mstat) = m.as_stat_mut() {
                        mstat.mutations.push(Mutation::StatMultExtra(StatId::AuraEffect, extra_aura_effect));
                    }
                    ret.push(m);
                }
            } else {
                ret.extend_from_slice(&mods);
            }
        }
        ret
    }

    pub fn resolve(&mut self) {
        let mods = self.calc_buffs_auras_mods();
        for m in mods.into_iter().filter(|m| {
            self.tags.contains(m.tags) &&
            (m.flags.is_empty() || self.flags.intersects(m.flags)) &&
            (m.weapons.is_empty() || self.build.is_holding(&m.weapons))
        }) {
            if let Some(mstat) = m.as_stat() {
                self.mods_by_stat.entry(mstat.stat).or_default().push(m.to_owned());
            } else {
                self.other_mods.push(m);
            }
        }
        self.resolve_armour();
        self.resolve_stats();
    }

    fn resolve_stats(&mut self) {
        let stat_ids: Vec<StatId> = self.mods_by_stat.keys().copied().collect();

        for stat_id in stat_ids {
            self.eval_stat(stat_id);
        }
    }

    fn resolve_armour(&mut self) {
        for (slot, idx) in &self.build.equipment {
            let item = &self.build.inventory[*idx];
            let defence = item.calc_defence();

            if defence.armour.val() != 0 {
                let val = self.eval_stat(StatId::Armour).val_custom(defence.armour.val());
                self.resolved_stats.entry(StatId::Armour).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
            if defence.energy_shield.val() != 0 {
                if self.build_flags.contains(&BuildFlag::ItemsGrantLifeInsteadES) {
                    self.mods_by_stat.entry(StatId::MaximumLife).or_default().push(Mod::stat(StatId::MaximumLife, Type::Base, defence.energy_shield.val()).with_source(Source::Item(*slot)));
                } else {
                    let val = self.eval_stat(StatId::MaximumEnergyShield).val_custom(defence.energy_shield.val());
                    self.resolved_stats.entry(StatId::MaximumEnergyShield).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
                }
            }
            if defence.evasion.val() != 0 {
                let val = self.eval_stat(StatId::EvasionRating).val_custom(defence.evasion.val());
                self.resolved_stats.entry(StatId::EvasionRating).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
            if defence.block_chance.val() != 0 {
                let val = self.eval_stat(StatId::ChanceToBlockAttackDamage).val_custom(defence.block_chance.val());
                self.resolved_stats.entry(StatId::ChanceToBlockAttackDamage).or_default().adjust_mod_move(Mod::stat(StatId::Armour, Type::Flat, val).with_source(Source::Item(*slot)));
            }
        }
    }

    pub fn gem_level_extra(&self, tags: BitFlags<GemTag>) -> u32 {
        self.other_mods.iter().filter(|m| tags.contains(m.tags)).flat_map(|m| m.as_gem_level()).sum()
    }

    pub fn gem_quality_extra(&self) -> i32 {
        self.other_mods.iter().flat_map(|m| m.as_gem_quality()).sum()
    }

    pub fn get_stat_val(&mut self, stat_id: StatId) -> i64 {
        self.eval_stat(stat_id).val()
    }

    pub fn get_stat_mult(&mut self, stat_id: StatId) -> i64 {
        self.eval_stat(stat_id).mult()
    }

    pub fn eval_stat(&mut self, stat_id: StatId) -> &Stat {
        if !self.resolved_stats.contains_key(&stat_id) {
            if !self.evaluating.insert(stat_id) {
                eprintln!("Warning: Circular dependency detected for stat: {:?}", stat_id);
                self.resolved_stats.insert(stat_id, Stat::default());
                return self.resolved_stats.get(&stat_id).unwrap();
            }

            let mut current_stat = Stat::default();
            let mods_to_process = self.mods_by_stat.remove(&stat_id).unwrap_or_default();

            for m in mods_to_process {
                let mut m = m.to_owned();
                let passes_conditions_bor = m.conditions.is_empty() || m.conditions.iter().any(|c| self.check_condition(c, m.source));
                if !passes_conditions_bor {
                    continue;
                }

                let source = m.source;
                if let Some(stat) = m.as_stat_mut() && !stat.mutations.is_empty() {
                    self.apply_mutations(stat, source);
                }

                if m.flags.contains(ModFlag::Aura) && let Some(stat) = m.as_stat_mut() {
                    let mult = self.get_stat_mult(StatId::AuraEffect);
                    let new_amount = (stat.final_amount() * mult) / 10000;
                    stat.revised_amount = Some(new_amount);
                }

                if let Source::Item(Slot::Flask(idx)) = m.source && let Some(stat) = m.as_stat_mut() {
                    let effect_local = self.build.get_equipped(Slot::Flask(idx)).unwrap().effect();
                    let mut flask_effect = self.eval_stat(StatId::FlaskEffect).clone();
                    flask_effect.assimilate(&effect_local);
                    let new_amount = (stat.final_amount() * flask_effect.mult()) / 10000;
                    stat.revised_amount = Some(new_amount);
                }

                current_stat.adjust_mod_move(m);
            }

            self.evaluating.remove(&stat_id);
            self.resolved_stats.insert(stat_id, current_stat);
        }

        self.resolved_stats.get(&stat_id).unwrap()
    }

    fn property_int_stats(&mut self, p: property::Int) -> i64 {
        let mut min = match property::int_data(p).min {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => self.get_stat_val(s),
        };
        let max = match property::int_data(p).max {
            property::Val::Val(i) => i,
            property::Val::Stat(s) => self.get_stat_val(s),
        };

        if self.build.is_property_int_maxed(p) {
            return max;
        }
        min = min.min(max);
        self.build.property_int(p).clamp(min, max)
    }

    fn check_condition(&mut self, c: &Condition, source: Source) -> bool {
        match c {
            Condition::GreaterEqualProperty(mutation) => {
                if self.property_int_stats(mutation.1) < mutation.0 { return false; }
            },
            Condition::LesserEqualProperty(mutation) => {
                if self.property_int_stats(mutation.1) > mutation.0 { return false; }
            },
            Condition::GreaterEqualStat(mutation) => {
                if self.get_stat_val(mutation.1) < mutation.0 { return false; }
            },
            Condition::LesserEqualStat(mutation) => {
                if self.get_stat_val(mutation.1) > mutation.0 { return false; }
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
                    &self.build.tree.nodes_data[node_id].name == *mastery_str
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
                if self.slot.is_none() {
                    return false;
                }
                if !matches!(source, Source::Item(slot) if Some(slot) == self.slot) {
                    return false;
                }
            }
        }
        true
    }

    fn apply_mutations(&mut self, m: &mut ModStat, source: Source) {
        let mut amount = m.amount;
        let mut up_to = i64::MAX;
        for f in &m.mutations {
            match f {
                Mutation::MultiplierProperty(mutation) => {
                    amount = (amount * self.property_int_stats(mutation.1)) / mutation.0;
                },
                Mutation::MultiplierStat(mutation) => {
                    amount = (amount * self.get_stat_val(mutation.1)) / mutation.0;
                },
                Mutation::MultiplierStatLowest(mutation) => {
                    let mut lowest = None;
                    for stat_id in mutation.1 {
                        let val = self.get_stat_val(*stat_id);
                        if lowest.is_none() || val < lowest.unwrap() {
                            lowest = Some(val);
                        }
                    }
                    amount = lowest.map_or(0, |l| (amount * l) / mutation.0);
                },
                Mutation::MultiplierSlotDefence((per, slot, defence)) => {
                    let def_amount = if let Some(item) = self.build.get_equipped(*slot) {
                        let defences = item.calc_defence();
                        match defence {
                            Defence::Armour => defences.armour.val(),
                            Defence::Evasion => defences.evasion.val(),
                            Defence::EnergyShield => defences.energy_shield.val(),
                            Defence::Block => defences.block_chance.val(),
                        }
                    } else {
                        0
                    };
                    amount = (amount * def_amount) / per;
                },
                Mutation::StatPct((pct, stat_id)) => {
                    amount = (self.get_stat_val(*stat_id) * pct) / 100;
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
                    amount = (self.eval_stat(*stat_id).inc * pct) / 100;
                },
                Mutation::MultiplierOvercap(per, stat_a, stat_b) => {
                    let delta = (self.eval_stat(*stat_a).val() - self.eval_stat(*stat_b).val()).max(0);
                    amount = (amount * delta) / per;
                },
                Mutation::StatMultExtra(stat_id, extra) => {
                    // Multiplies by a Stat's multiplier with an added extra Inc
                    // Typical use is AuraEffect where the extra is the sum of support gems' increased aura effect
                    let mut stat = self.eval_stat(*stat_id).clone();
                    stat.adjust(Type::Inc, *extra);
                    amount = stat.val_custom(amount);
                },
            }
        }

        m.revised_amount = Some(amount.min(up_to));
    }
}
