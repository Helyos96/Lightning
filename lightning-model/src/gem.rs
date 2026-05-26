use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicI32, Ordering};

use crate::build::stat::StatId;
use crate::data::gem::{GemData, GemTag};
use crate::data::{DamageType, GEMS};
use crate::gemstats;
use crate::modifier::{Mod, ModFlag, Source, Type};
use crate::{item, util};
use crate::data;
use arc_swap::ArcSwap;
use derivative::Derivative;
use enumflags2::make_bitflags;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// Shadow struct for serde so that we can keep having `data` as a &'static in Gem
#[derive(Deserialize)]
struct RawGem {
    id: String,
    enabled: bool,
    level: u32,
    qual: i32,
    alt_qual: i32,
}

/// Gem used in Build
#[derive(Debug, Derivative, Serialize, Deserialize)]
#[derivative(Clone)]
#[serde(from = "RawGem")]
pub struct Gem {
    pub id: String,
    data: &'static GemData,
    pub enabled: bool,
    pub level: u32,
    pub qual: i32,
    pub alt_qual: i32,

    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_arc_swap"))]
    mod_cache: ArcSwap<Vec<Mod>>,

    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_arc_swap"))]
    mod_cache_auras: ArcSwap<Vec<Mod>>,

    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_atomic_u32"))]
    mod_cache_level: AtomicU32,

    #[serde(skip)]
    #[derivative(Clone(clone_with = "clone_atomic_i32"))]
    mod_cache_qual: AtomicI32,
}

impl From<RawGem> for Gem {
    fn from(raw: RawGem) -> Self {
        Gem {
            data: &GEMS[&raw.id],
            id: raw.id,
            enabled: raw.enabled,
            level: raw.level,
            qual: raw.qual,
            alt_qual: raw.alt_qual,

            mod_cache: ArcSwap::from_pointee(Vec::new()),
            mod_cache_auras: ArcSwap::from_pointee(Vec::new()),
            mod_cache_level: AtomicU32::new(0),
            mod_cache_qual: AtomicI32::new(0),
        }
    }
}

fn clone_arc_swap<T>(cache: &ArcSwap<T>) -> ArcSwap<T> {
    ArcSwap::new(cache.load_full())
}

fn clone_atomic_u32(u32_ref: &AtomicU32) -> AtomicU32 {
    AtomicU32::new(u32_ref.load(Ordering::Relaxed))
}

fn clone_atomic_i32(i32_ref: &AtomicI32) -> AtomicI32 {
    AtomicI32::new(i32_ref.load(Ordering::Relaxed))
}

fn extract_bracket_content(input: &str) -> Option<&str> {
    let (_, after_open) = input.split_once('{')?;
    let (inside, _) = after_open.split_once('}')?;
    Some(inside)
}

impl Gem {
    pub fn new(id: String, enabled: bool, level: u32, qual: i32, alt_qual: i32) -> Gem {
        Gem {
            data: &GEMS[&id],
            id,
            enabled,
            level,
            qual,
            alt_qual,
            mod_cache: Default::default(),
            mod_cache_auras: Default::default(),
            mod_cache_level: Default::default(),
            mod_cache_qual: Default::default(),
        }
    }

    pub fn data(&self) -> &'static GemData {
        self.data
    }

    pub fn can_support(&self, active_gem: &Gem) -> bool {
        if let Some(active_skill) = active_gem.data().active_skill.as_ref() &&
           let Some(support_gem) = self.data().support_gem.as_ref()
        {
            if let Some(excluded_types) = support_gem.excluded_types.as_ref() {
                if !excluded_types.is_disjoint(&active_skill.types) {
                    return false;
                }
            }
            if let Some(allowed_types) = support_gem.allowed_types.as_ref() {
                if !allowed_types.is_empty() &&
                    allowed_types.is_disjoint(&active_skill.types) {
                    return false;
                }
            }
        }
        true
    }

    pub fn format_quality_stats(&self) -> Vec<String> {
        let mut ret = vec![];
        for quality_stat in &self.data().r#static.quality_stats {
            if let Some(inside_brackets) = extract_bracket_content(&quality_stat.stat) &&
               let Some(val) = quality_stat.stats.get(inside_brackets)
            {
                let val = (val * self.qual) / 1000;
                if val == 0 {
                    continue;
                }
                let stat = quality_stat.stat.replace(&format!("{{{}}}", inside_brackets), &(val).to_string());
                ret.push(stat);
            }
        }
        ret
    }

    fn regen_modcache(&self, level: u32, qual: i32) {
        self.mod_cache.store(Arc::new(self.data().calc_mods(false, level, qual)));
        self.mod_cache_auras.store(Arc::new(self.data().calc_mods(true, level, qual)));
        self.mod_cache_level.store(level, Ordering::Relaxed);
        self.mod_cache_qual.store(qual, Ordering::Relaxed);
    }

    pub fn invalidate_modcache(&self) {
        self.mod_cache_level.store(0, Ordering::Relaxed);
    }

    pub fn set_level(&mut self, level: u32) {
        self.level = level;
        self.invalidate_modcache();
    }

    pub fn set_qual(&mut self, qual: i32) {
        self.qual = qual;
        self.invalidate_modcache();
    }

    pub fn calc_mods(&self, as_aura_buff_curse: bool, extra_level: u32, extra_qual: i32) -> Arc<Vec<Mod>> {
        let level = self.level + extra_level;
        let qual = self.qual + extra_qual;

        if self.mod_cache_level.load(Ordering::Relaxed) != level ||
           self.mod_cache_qual.load(Ordering::Relaxed) != qual
        {
            self.regen_modcache(level, qual);
        }

        match as_aura_buff_curse {
            true => arc_swap::Guard::into_inner(self.mod_cache_auras.load()),
            false => arc_swap::Guard::into_inner(self.mod_cache.load()),
        }
    }

    pub fn mana_cost_level(&self, extra_level: u32) -> Option<i64> {
        self.data().mana_cost(self.level + extra_level)
    }

    pub fn cost_multiplier_level(&self, extra_level: u32) -> Option<i64> {
        self.data().cost_multiplier(self.level + extra_level)
    }

    fn stat_value_level(&self, id: &str) -> Option<i64> {
        self.data().stat_value_level(id, self.level)
    }

    pub fn stat_value(&self, id: &str) -> Option<i64> {
        self.data().stat_value(id, self.level)
    }

    pub fn crit_chance(&self) -> Option<i64> {
        self.data().r#static.crit_chance
    }

    pub fn added_effectiveness(&self, extra_level: u32) -> Option<i64> {
        if let Some(level_data) = self.data().per_level.get(&(self.level + extra_level)) {
            if level_data.damage_effectiveness.is_some() {
                return level_data.damage_effectiveness;
            }
        }
        self.data().r#static.damage_effectiveness
    }

    pub fn damage_multiplier(&self, extra_level: u32) -> Option<i64> {
        if let Some(level_data) = self.data().per_level.get(&(self.level + extra_level)) {
            if level_data.damage_multiplier.is_some() {
                return level_data.damage_multiplier;
            }
        }
        self.data().r#static.damage_multiplier
    }
}
