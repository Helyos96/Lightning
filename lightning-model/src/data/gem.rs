use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use enumflags2::{BitFlags, bitflags, make_bitflags};
use crate::{gemstats, modifier::{Mod, Source, Type}};
use crate::modifier::ModFlag;
use crate::build::stat::StatId;

use super::base_item::ItemClass;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveSkill {
    description: String,
    display_name: String,
    id: String,
    stat_conversions: Option<FxHashMap<String, String>>,
    pub weapon_restrictions: FxHashSet<ItemClass>,
    #[serde(default)]
    pub types: FxHashSet<ActiveSkillType>,
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
    pub mana: Option<i32>,
    #[serde(rename = "Life")]
    pub life: Option<i32>,
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
    pub costs: Option<Costs>,
    required_level: Option<f32>,
    #[serde(default)]
    stat_requirements: Option<StatRequirements>,
    pub stats: Option<Vec<Option<LevelStat>>>,
    pub damage_effectiveness: Option<i64>,
    pub damage_multiplier: Option<i64>,
    pub cost_multiplier: Option<i64>,
    #[serde(default)]
    pub stat_text: Option<FxHashMap<String, String>>,
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
    pub stat: String,
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
    #[serde(default)]
    pub stat_text: Option<FxHashMap<String, String>>,
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
pub enum ActiveSkillType {
    AND,
    AppliesCurse,
    AppliesMaim,
    Arcane,
    Area,
    AreaSpell,
    Attack,
    AttackInPlaceIsDefault,
    Aura,
    AuraAffectsEnemies,
    AuraDuration,
    AuraNotOnCaster,
    Banner,
    Blessing,
    Blink,
    Brand,
    Buff,
    CanHaveBlessing,
    CanRapidFire,
    Cascadable,
    CausesBurning,
    Chains,
    Channel,
    Chaos,
    ChillingArea,
    Cold,
    Cooldown,
    CreatesMinion,
    CreatesSentinelMinion,
    Damage,
    DamageOverTime,
    DegenOnlySpellDamage,
    DestroysCorpse,
    DisallowTriggerSupports,
    DualWieldOnly,
    DualWieldRequiresDifferentTypes,
    Duration,
    DynamicCooldown,
    ElementalStatus,
    Fire,
    FixedCastTime,
    FixedSpeedProjectile,
    GainsIntensity,
    Golem,
    Guard,
    HasReservation,
    Herald,
    Hex,
    InbuiltTrigger,
    InnateTrauma,
    Instant,
    InstantNoRepeatWhenHeld,
    InstantShiftAttackForLeftMouse,
    KillNoDamageModifiers,
    LateConsumeCooldown,
    Lightning,
    Link,
    Mark,
    Melee,
    MeleeSingleTarget,
    Mineable,
    Minion,
    MinionsAreUndamagable,
    MinionsCanExplode,
    MinionsPersistWhenSkillRemoved,
    MirageArcherCanUse,
    Movement,
    Multicastable,
    Multistrikeable,
    NOT,
    NeverExertable,
    NoRuthless,
    NoVolley,
    NonHitChill,
    NonRepeatable,
    Nova,
    OR,
    Offering,
    Orb,
    OtherThingUsesSkill,
    Physical,
    PreventHexTransfer,
    Projectile,
    ProjectileCannotReturn,
    ProjectileNumber,
    ProjectileSpeed,
    ProjectileSpiral,
    ProjectilesFromUser,
    ProjectilesNotFromUser,
    ProjectilesNumberModifiersNotApplied,
    Rain,
    RandomElement,
    RangedAttack,
    RemoteMined,
    RequiresOffHandNotWeapon,
    RequiresShield,
    ReservationBecomesCost,
    Retaliation,
    SingleMainProjectile,
    SkillGrantedBySupport,
    Slam,
    Spell,
    Stance,
    Steel,
    SummonsTotem,
    SupportedByBane,
    ThresholdJewelArea,
    ThresholdJewelChaining,
    ThresholdJewelDuration,
    ThresholdJewelProjectile,
    ThresholdJewelRangedAttack,
    ThresholdJewelSpellDamage,
    TotemCastsAlone,
    TotemCastsWhenNotDetached,
    Totemable,
    TotemsAreBallistae,
    Trappable,
    Trapped,
    Travel,
    Triggerable,
    Triggered,
    Vaal,
    WandAttack,
    Warcry,
    ZeroReservation,
}

#[bitflags]
#[repr(u64)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, Eq, PartialEq, strum_macros::IntoStaticStr)]
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
pub struct SupportGemData {
    #[serde(default)]
    pub allowed_types: Option<FxHashSet<ActiveSkillType>>,
    #[serde(default)]
    pub excluded_types: Option<FxHashSet<ActiveSkillType>>,
    pub support_text: String,
}

// Ser/Des an array into BitFlags
pub mod enumflags2_array {
    use enumflags2::{BitFlag, BitFlags};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<BitFlags<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: BitFlag + Deserialize<'de>,
    {
        let vec: Vec<T> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }

    pub fn serialize<S, T>(flags: &BitFlags<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: BitFlag + Serialize,
    {
        let vec: Vec<T> = flags.into_iter().collect();
        vec.serialize(serializer)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GemData {
    pub active_skill: Option<ActiveSkill>,
    pub base_item: BaseItem,
    pub cast_time: Option<i64>,
    pub is_support: bool,
    pub per_level: FxHashMap<u32, Level>,
    pub r#static: Static,
    #[serde(default, with = "enumflags2_array")]
    pub tags: BitFlags<GemTag>,
    #[serde(default)]
    pub weapon_restrictions: FxHashSet<ItemClass>,
    #[serde(default)]
    pub tooltip_order: Vec<String>,
    #[serde(default)]
    pub support_gem: Option<SupportGemData>,
    pub color: String,
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
        if let Some(max_level) = self.per_level.keys().max().copied() {
            return max_level.min(20) as i32;
        }
        return 20;
    }

    pub fn stat_text(&'static self, stat: &str, level: u32) -> Option<&'static str> {
        if let Some(level) = self.per_level.get(&level) &&
           let Some(stat_text) = &level.stat_text &&
           let Some(text) = stat_text.get(stat)
        {
            return Some(text);
        }
        if let Some(stat_text) = &self.r#static.stat_text &&
           let Some(text) = stat_text.get(stat)
        {
            return Some(text);
        }
        None
    }

    pub fn description(&'static self) -> Option<&'static str> {
        if let Some(active_skill) = &self.active_skill {
            return Some(&active_skill.description);
        } else if let Some(support_data) = &self.support_gem {
            return Some(&support_data.support_text);
        }
        None
    }

    pub fn mana_cost(&self, level: u32) -> Option<i64> {
        let level_data = self.per_level.get(&level)?;
        if let Some(mana) = level_data.costs.as_ref()?.mana {
            Some(mana as i64)
        } else {
            None
        }
    }

    pub fn cost_multiplier(&self, level: u32) -> Option<i64> {
        let level_data = self.per_level.get(&level)?;
        level_data.cost_multiplier
    }

    pub fn stat_value_level(&self, id: &str, level: u32) -> Option<i64> {
        let idx = self.r#static.stat_idx(id)?;
        let level_data = self.per_level.get(&level)?;
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
    pub fn stat_value(&self, id: &str, level: u32) -> Option<i64> {
        if let Some(value_level) = self.stat_value_level(id, level) {
            return Some(value_level);
        }

        if let Some(stats) = &self.r#static.stats {
            if let Some(gem_stat) = stats.iter().flatten().find(|x| x.id.as_ref().is_some_and(|stat_id| stat_id == id)) {
                if gem_stat.value.is_some() {
                    return gem_stat.value;
                }
            }
        }

        None
    }

    pub fn calc_mods(&'static self, as_aura_buff_curse: bool, level: u32, qual: i32) -> Vec<Mod> {
        let mut mods = vec![];
        let source = Source::Gem(self.display_name());

        if let Some(stats) = &self.r#static.stats {
            for gem_stat in stats.iter().flatten() {
                if let Some(id) = &gem_stat.id {
                    if let Some(modifiers) = gemstats::match_gemstat(&self.base_item.display_name, id) {
                        for mut modifier in modifiers {
                            if as_aura_buff_curse != modifier.flags.intersects(make_bitflags!(ModFlag::{Aura | Buff | Curse})) {
                                continue;
                            }
                            let amount = self.stat_value(id, level).unwrap_or(0);
                            if let Some(stat) = modifier.as_stat_mut() {
                                if stat.amount == 0 {
                                    stat.amount = amount;
                                }
                            } else if modifier.as_gem_level().is_some() {
                                modifier.effect = crate::modifier::ModEffect::LevelOfGems(amount as u32)
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

        for quality_stat in &self.r#static.quality_stats {
            for (stat_name, val) in &quality_stat.stats {
                if let Some(modifiers) = gemstats::match_gemstat(&self.base_item.display_name, stat_name) {
                    for mut modifier in modifiers {
                        if as_aura_buff_curse != modifier.flags.intersects(make_bitflags!(ModFlag::{Aura | Buff | Curse})) {
                            continue;
                        }
                        if let Some(stat) = modifier.as_stat_mut() {
                            if stat.amount == 0 {
                                stat.amount = (*val as i64 * qual as i64) / 1000;
                            }
                        }
                        modifier.source = source;
                        mods.push(modifier);
                    }
                } else {
                    //println!("failed: {stat_name}");
                }
            }
        }

        if !as_aura_buff_curse {
            if let Some(speed_multiplier) = &self.r#static.attack_speed_multiplier {
                mods.push(Mod::stat(StatId::AttackSpeed, Type::More, *speed_multiplier as i64).with_source(source));
            }

            if let Some(base_mana_cost) = self.mana_cost(level) {
                mods.push(Mod::stat(StatId::ManaCost, Type::Base, base_mana_cost).with_source(source));
            }

            if let Some(cost_multiplier) = self.cost_multiplier(level) {
                mods.push(Mod::stat(StatId::Cost, Type::More, cost_multiplier).with_source(source));
            }
        }

        mods
    }
}
