use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use super::{base_item::ItemClass, ActiveSkillTypes};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveSkill {
    description: String,
    display_name: String,
    id: String,
    stat_conversions: Option<FxHashMap<String, String>>,
    pub weapon_restrictions: FxHashSet<ItemClass>,
    types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseItem {
    pub display_name: String,
    pub id: String,
    pub max_level: Option<i32>,
    pub release_state: String,
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
    pub value: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level {
    costs: Option<Costs>,
    required_level: Option<f32>,
    #[serde(default)]
    stat_requirements: Option<StatRequirements>,
    pub stats: Option<Vec<Option<LevelStat>>>,
    pub damage_effectiveness: Option<i64>,
    pub damage_multiplier: Option<i64>,
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
    pub value: Option<i64>,
    pub id: Option<String>,
    r#type: Option<GemStatType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityStat {
    stat: String,
    pub stats: FxHashMap<String, i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Static {
    pub crit_chance: Option<i64>,
    pub cooldown: Option<i32>,
    pub damage_effectiveness: Option<i64>,
    pub damage_multiplier: Option<i64>,
    pub attack_speed_multiplier: Option<i32>,
    pub stats: Option<Vec<Option<GemStat>>>,
    pub quality_stats: Vec<QualityStat>,
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
    pub base_item: BaseItem,
    pub cast_time: Option<i64>,
    pub is_support: bool,
    pub per_level: FxHashMap<u32, Level>,
    pub r#static: Static,
    #[serde(default)]
    pub tags: FxHashSet<GemTag>,
    #[serde(default)]
    pub weapon_restrictions: FxHashSet<ItemClass>,
    #[serde(default)]
    pub types: FxHashSet<ActiveSkillTypes>,
}

impl GemData {
    pub fn display_name(&'static self) -> &'static str {
        if let Some(active_skill) = self.active_skill.as_ref() {
            &active_skill.display_name
        } else {
            &self.base_item.display_name
        }
    }

    pub fn max_level(&self) -> i32 {
        if let Some(max_level) = self.base_item.max_level {
            return max_level;
        }
        return 20;
    }
}
