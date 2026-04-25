use enumflags2::BitFlags;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{build::{Build, Defence, property, stat::{self, Stat, StatId}}, data::gem::GemTag, modifier::{Condition, Mod, ModFlag, Mutation}};

/// Evaluate Stats from a collection of Mods
pub struct Evaluator<'a> {
    build: &'a Build,
    pub mods_by_stat: FxHashMap<StatId, Vec<&'a Mod>>,
    pub resolved_stats: FxHashMap<StatId, Stat>,
    evaluating: FxHashSet<StatId>,
}

impl<'a> Evaluator<'a> {
    pub fn new(build: &'a Build, mods: &'a [Mod], tags: BitFlags<GemTag>, flags: BitFlags<ModFlag>) -> Self {
        let mut mods_by_stat: FxHashMap<StatId, Vec<&'a Mod>> = FxHashMap::default();

        for m in mods.iter().filter(|m| {
            tags.contains(m.tags) &&
            (m.flags.is_empty() || flags.intersects(m.flags)) &&
            (m.weapons.is_empty() || build.is_holding(&m.weapons))
        }) {
            mods_by_stat.entry(m.stat).or_default().push(m);
        }

        Self {
            build,
            mods_by_stat,
            resolved_stats: FxHashMap::default(),
            evaluating: FxHashSet::default(),
        }
    }

    pub fn get_stat_val(&mut self, stat_id: StatId) -> i64 {
        self.eval_stat(stat_id).val()
    }

    pub fn get_stat_mult(&mut self, stat_id: StatId) -> i64 {
        self.eval_stat(stat_id).mult()
    }

    pub fn eval_stat(&mut self, stat_id: StatId) -> Stat {
        if let Some(stat) = self.resolved_stats.get(&stat_id) {
            return stat.clone();
        }

        if !self.evaluating.insert(stat_id) {
            eprintln!("Warning: Circular dependency detected for stat: {:?}", stat_id);
            return Stat::default();
        }

        let mut current_stat = Stat::default();
        let mods_to_process = self.mods_by_stat.get(&stat_id).cloned().unwrap_or_default();

        for m in mods_to_process {
            let mut m = m.to_owned();

            if !self.check_conditions(&m) {
                continue;
            }

            if !m.mutations.is_empty() {
                self.apply_mutations(&mut m);
            }

            if m.flags.contains(ModFlag::Aura) {
                let mult = self.get_stat_mult(StatId::AuraEffect);
                m.revised_amount = Some((m.final_amount() * mult) / 10000);
            }

            current_stat.adjust_mod_move(m);
        }

        self.evaluating.remove(&stat_id);
        self.resolved_stats.insert(stat_id, current_stat.clone());

        current_stat
    }

    fn property_int_stats(&mut self, p: property::Int) -> i64 {
        let min = match property::int_data(p).min {
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
        self.build.property_int(p).clamp(min, max)
    }

    fn check_conditions(&mut self, m: &Mod) -> bool {
        for c in &m.conditions {
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
            }
        }
        true
    }

    fn apply_mutations(&mut self, m: &mut Mod) {
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
                Mutation::UpTo(mutation) => {
                    up_to = *mutation;
                },
            }
        }
        m.revised_amount = Some(amount.min(up_to));
    }
}
