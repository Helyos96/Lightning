use crate::data::GEMS;
use crate::gemstats::GEMSTATS;
use crate::modifier::{Mod, Source};
use crate::{item, util};
use crate::data;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveSkill {
    description: String,
    display_name: String,
    id: String,
    stat_conversions: Option<FxHashMap<String, String>>,
    types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseItem {
    pub display_name: String,
    id: String,
    max_level: Option<i32>,
    release_state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Costs {
    #[serde(rename = "Mana")]
    mana: Option<i32>,
    #[serde(rename = "Life")]
    life: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StatRequirements {
    #[serde(default)]
    str: Option<i32>,
    #[serde(default)]
    dex: Option<i32>,
    #[serde(default)]
    int: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelStat {
    value: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level {
    costs: Option<Costs>,
    required_level: Option<f32>,
    #[serde(default)]
    stat_requirements: Option<StatRequirements>,
    stats: Option<Vec<Option<LevelStat>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum GemStatType {
    Additional,
    Constant,
    Flag,
    Float,
    Implicit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GemStat {
    value: Option<i64>,
    id: Option<String>,
    r#type: Option<GemStatType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Static {
    crit_chance: Option<i32>,
    cooldown: Option<i32>,
    damage_effectiveness: Option<i32>,
    attack_speed_multiplier: Option<i32>,
    pub stats: Option<Vec<Option<GemStat>>>,
}

impl Static {
    pub fn stat_idx(&self, id: &str) -> Option<usize> {
        let stats = self.stats.as_ref()?;
        stats.iter().position(|x| {
            if let Some(stat) = x {
                if let Some(stat_id) = &stat.id {
                    if stat_id == id {
                        return true;
                    }
                }
            }
            false
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(non_camel_case_types)]
pub enum GemTag {
    Attack,
    Melee,
    Spell,
    Projectile,
    Fire,
    Cold,
    Lightning,
    Random_Element,
    Chaos,
    Physical,
    Minion,
    Golem,
    Duration,
    Area,
    Warcry,
    Trigger,
    Aura,
    Critical,
    Curse,
    Hex,
    Mark,
    Movement,
    Travel,
    Totem,
    Slam,
    Intelligence,
    Strength,
    Dexterity,
    Chaining,
    Guard,
    Arcane,
    Bow,
    Strike,
    Trap,
    Mine,
    Orb,
    Channelling,
    Stance,
    Low_Max_Level,
    Banner,
    Brand,
    Vaal,
    Nova,
    Link,
    Blink,
    Herald,
    Exceptional,
    Blessing,
    Active_Skill,
    Grants_Active_Skill,
    Awakened,
    Support,
    Retaliation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GemData {
    pub active_skill: Option<ActiveSkill>,
    pub base_item: Option<BaseItem>,
    pub cast_time: Option<i64>,
    pub is_support: bool,
    per_level: FxHashMap<u32, Level>,
    pub r#static: Static,
    #[serde(default)]
    pub tags: FxHashSet<GemTag>,
    #[serde(default)]
    pub weapon_restrictions: FxHashSet<item::ItemClass>,
    #[serde(default)]
    pub types: FxHashSet<data::ActiveSkillTypes>,
}

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
                        println!("failed: {id}");
                    }
                }
            }
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
}
