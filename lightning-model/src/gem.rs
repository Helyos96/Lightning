use crate::data::GEMS;
use crate::gemstats::GEMSTATS;
use crate::modifier::{Mod, Source};
use crate::util;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveSkill {
    description: String,
    display_name: String,
    id: String,
    stat_conversions: Option<FxHashMap<String, String>>,
    types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stat {
    id: String,
    value: Option<i64>,
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
    #[serde(default = "zero")]
    str: i32,
    #[serde(default = "zero")]
    dex: i32,
    #[serde(default = "zero")]
    int: i32,
}

// change this once serde allows literal defaults
fn zero() -> i32 {
    0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level {
    costs: Option<Costs>,
    required_level: Option<i32>,
    #[serde(default)]
    stat_requirements: StatRequirements,
    stats: Option<Vec<Option<i64>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Static {
    crit_chance: Option<i32>,
    cooldown: Option<i32>,
    damage_effectiveness: Option<i32>,
    attack_speed_multiplier: Option<i32>,
    pub stats: Option<Vec<Stat>>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(non_camel_case_types)]
pub enum Tag {
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
    Support,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GemData {
    pub active_skill: Option<ActiveSkill>,
    pub base_item: Option<BaseItem>,
    pub cast_time: Option<i64>,
    pub is_support: bool,
    per_level: Vec<Level>,
    pub r#static: Static,
    #[serde(default)]
    pub tags: FxHashSet<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    pub id: String,
    pub level: usize,
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
            for (pos, stat) in stats.iter().enumerate() {
                let value = self.stat_value(pos, stat);
                if let Some(stat_mods) = GEMSTATS.get(&stat.id[0..]) {
                    for m in stat_mods {
                        let mut modifier = m.to_owned();
                        modifier.amount = value;
                        modifier.source = Source::Gem;
                        mods.push(modifier);
                    }
                } else {
                    //println!("failed: {}", &stat.id);
                }
            }
        }

        mods
    }
    /// Get the value of a gem stat.
    /// Will use per_level value if available,
    /// otherwise static value, otherwise None
    /// todo: add quality value if present.
    fn stat_value(&self, pos: usize, stat: &Stat) -> i64 {
        if let Some(stats_lvl) = &self.data().per_level[self.level - 1].stats {
            if let Some(Some(value)) = stats_lvl.get(pos) {
                return *value;
            }
        }
        if let Some(value) = stat.value {
            return value;
        }

        0
    }
}
