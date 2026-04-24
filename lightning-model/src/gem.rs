use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::build::stat::StatId;
use crate::data::gem::{GemData, GemTag};
use crate::data::{DamageType, GEMS};
use crate::gemstats;
use crate::modifier::{Mod, ModFlag, Source, Type};
use crate::{item, util};
use crate::data;
use enumflags2::make_bitflags;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    pub id: String,
    pub enabled: bool,
    pub level: u32,
    pub qual: i32,
    pub alt_qual: i32,
    #[serde(skip)]
    mod_cache: RefCell<Rc<Vec<Mod>>>,
    #[serde(skip)]
    mod_cache_auras: RefCell<Rc<Vec<Mod>>>,
    #[serde(skip, default = "default_cell_true")]
    should_regen_modcache: Cell<bool>,
}

fn default_cell_true() -> Cell<bool> {
    Cell::new(true)
}

impl Gem {
    pub fn new(id: String, enabled: bool, level: u32, qual: i32, alt_qual: i32) -> Gem {
        Gem {
            id,
            enabled,
            level,
            qual,
            alt_qual,
            mod_cache: Default::default(),
            mod_cache_auras: Default::default(),
            should_regen_modcache: Cell::new(true),
        }
    }

    pub fn data(&self) -> &'static GemData {
        &GEMS[&self.id]
    }

    pub fn can_support(&self, active_gem: &Gem) -> bool {
        if let Some(active_skill) = active_gem.data().active_skill.as_ref() &&
           let Some(support_gem) = self.data().support_gem.as_ref() &&
           let Some(excluded_types) = support_gem.excluded_types.as_ref() &&
           let Some(allowed_types) = support_gem.allowed_types.as_ref() {
            if !excluded_types.is_disjoint(&active_skill.types) {
                return false;
            }
            if !allowed_types.is_empty() &&
                allowed_types.is_disjoint(&active_skill.types) {
                return false;
            }
        }
        true
    }

    fn regen_modcache(&self) {
        *self.mod_cache.borrow_mut() = Rc::new(self._calc_mods(false));
        *self.mod_cache_auras.borrow_mut() = Rc::new(self._calc_mods(true));
        self.should_regen_modcache.set(false);
    }

    pub fn _calc_mods(&self, as_aura_buff: bool) -> Vec<Mod> {
        let mut mods = vec![];
        let source = Source::Gem(self.data().display_name());

        if let Some(stats) = &self.data().r#static.stats {
            for gem_stat in stats.iter().flatten() {
                if let Some(id) = &gem_stat.id {
                    if let Some(modifiers) = gemstats::match_gemstat(&self.data().base_item.display_name, id) {
                        for mut modifier in modifiers {
                            if as_aura_buff != modifier.flags.intersects(make_bitflags!(ModFlag::{Aura | Buff})) {
                                continue;
                            }
                            if modifier.amount == 0 {
                                modifier.amount = self.stat_value(id).unwrap_or(0);
                            }
                            modifier.source = source;
                            mods.push(modifier);
                        }
                    } else {
                        //println!("failed: {id}");
                    }
                }
            }
        }

        for quality_stat in &self.data().r#static.quality_stats {
            for (stat_name, val) in &quality_stat.stats {
                if let Some(modifiers) = gemstats::match_gemstat(&self.data().base_item.display_name, stat_name) {
                    for mut modifier in modifiers {
                        if as_aura_buff != modifier.flags.intersects(make_bitflags!(ModFlag::{Aura | Buff})) {
                            continue;
                        }
                        if modifier.amount == 0 {
                            modifier.amount = (*val as i64 * self.qual as i64) / 1000;
                        }
                        modifier.source = source;
                        mods.push(modifier);
                    }
                } else {
                    //println!("failed: {stat_name}");
                }
            }
        }

        if !as_aura_buff {
            if let Some(speed_multiplier) = &self.data().r#static.attack_speed_multiplier {
                mods.push(Mod {stat: StatId::AttackSpeed, typ: Type::More, amount: *speed_multiplier as i64, source, ..Default::default()});
            }

            if let Some(base_mana_cost) = self.mana_cost_level() {
                mods.push(Mod {stat: StatId::ManaCost, typ: Type::Base, amount: base_mana_cost, source, ..Default::default()});
            }

            if let Some(cost_multiplier) = self.cost_multiplier_level() {
                mods.push(Mod {stat: StatId::Cost, typ: Type::More, amount: cost_multiplier, source, ..Default::default()});
            }
        }

        mods
    }

    pub fn set_level(&mut self, level: u32) {
        self.level = level;
        self.should_regen_modcache.set(true);
    }

    pub fn set_qual(&mut self, qual: i32) {
        self.qual = qual;
        self.should_regen_modcache.set(true);
    }

    pub fn calc_mods(&self, as_aura_buff: bool) -> Rc<Vec<Mod>> {
        if self.should_regen_modcache.get() {
            self.regen_modcache();
        }

        match as_aura_buff {
            true => self.mod_cache_auras.borrow().clone(),
            false => self.mod_cache.borrow().clone(),
        }
    }

    pub fn mana_cost_level(&self) -> Option<i64> {
        let level_data = self.data().per_level.get(&self.level)?;
        if let Some(mana) = level_data.costs.as_ref()?.mana {
            Some(mana as i64)
        } else {
            None
        }
    }

    pub fn cost_multiplier_level(&self) -> Option<i64> {
        let level_data = self.data().per_level.get(&self.level)?;
        level_data.cost_multiplier
    }

    fn stat_value_level(&self, id: &str) -> Option<i64> {
        let idx = self.data().r#static.stat_idx(id)?;
        let level_data = self.data().per_level.get(&self.level)?;
        let level_stats = level_data.stats.as_ref()?;
        if let Some(Some(stat)) = level_stats.get(idx) {
            return stat.value;
        }
        None
    }

    /// Get the value of a gem stat.
    /// Will use per_level value if available,
    /// otherwise static value, otherwise None
    /// todo: add quality value if present.
    pub fn stat_value(&self, id: &str) -> Option<i64> {
        if let Some(value_level) = self.stat_value_level(id) {
            return Some(value_level);
        }

        if let Some(stats) = &self.data().r#static.stats {
            if let Some(gem_stat) = stats.iter().flatten().find(|x| x.id.as_ref().is_some_and(|stat_id| stat_id == id)) {
                if gem_stat.value.is_some() {
                    return gem_stat.value;
                }
            }
        }

        None
    }

    pub fn crit_chance(&self) -> Option<i64> {
        self.data().r#static.crit_chance
    }

    pub fn added_effectiveness(&self) -> Option<i64> {
        if let Some(level_data) = self.data().per_level.get(&self.level) {
            if level_data.damage_effectiveness.is_some() {
                return level_data.damage_effectiveness;
            }
        }
        self.data().r#static.damage_effectiveness
    }

    pub fn damage_multiplier(&self) -> Option<i64> {
        if let Some(level_data) = self.data().per_level.get(&self.level) {
            if level_data.damage_multiplier.is_some() {
                return level_data.damage_multiplier;
            }
        }
        self.data().r#static.damage_multiplier
    }
}
