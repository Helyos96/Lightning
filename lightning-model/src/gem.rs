use crate::build::stat::StatId;
use crate::data::gem::GemData;
use crate::data::{DamageType, GEMS};
use crate::gemstats::GEMSTATS;
use crate::modifier::{Mod, Source, Type};
use crate::{item, util};
use crate::data;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    pub id: String,
    pub enabled: bool,
    pub level: u32,
    pub qual: i32,
    pub alt_qual: i32,
}

impl Gem {
    pub fn data(&self) -> &'static GemData {
        &GEMS[&self.id]
    }

    pub fn calc_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        if let Some(stats) = &self.data().r#static.stats {
            for gem_stat in stats.iter().flatten() {
                if let Some(id) = &gem_stat.id {
                    if let Some(stat_mods) = GEMSTATS.get(id.as_str()) {
                        for m in stat_mods {
                            let mut modifier = m.to_owned();
                            modifier.amount = self.stat_value(id).unwrap_or(0);
                            modifier.source = Source::Gem;
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
                if let Some(stat_mods) = GEMSTATS.get(stat_name.as_str()) {
                    for m in stat_mods {
                        let mut modifier = m.to_owned();
                        modifier.amount = (*val as i64 * self.qual as i64) / 1000;
                        modifier.source = Source::Gem;
                        mods.push(modifier);
                    }
                } else {
                    //println!("failed: {stat_name}");
                }
            }
        }

        if let Some(speed_multiplier) = &self.data().r#static.attack_speed_multiplier {
            mods.push(Mod {stat: StatId::AttackSpeed, typ: Type::More, amount: *speed_multiplier as i64, ..Default::default()});
        }

        mods
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
        let value_level = self.stat_value_level(id);
        if value_level.is_some() {
            return value_level;
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
