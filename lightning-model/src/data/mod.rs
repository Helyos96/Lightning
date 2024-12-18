pub mod base_item;
pub mod default_monster_stats;
pub mod gem;
pub mod tree;
pub mod poe2;

use base_item::BaseItem;
use default_monster_stats::MonsterStats;
use gem::GemData;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use strum_macros::IntoStaticStr;
use tree::TreeData;
use std::error::Error;
use std::fs;
use std::io;
use serde::{Deserialize, Serialize};
use crate::build::stat::StatId;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(non_camel_case_types)]
pub enum ActiveSkillTypes {
    Attack,
    Spell,
    Projectile,
    DualWieldOnly,
    Buff,
    Minion,
    Damage,
    Area,
    Duration,
    RequiresShield,
    ProjectileSpeed,
    HasReservation,
    ReservationBecomesCost,
    Trappable,
    Totemable,
    Mineable,
    ElementalStatus,
    MinionsCanExplode,
    Chains,
    Melee,
    MeleeSingleTarget,
    Multicastable,
    TotemCastsAlone,
    Multistrikeable,
    CausesBurning,
    SummonsTotem,
    TotemCastsWhenNotDetached,
    Physical,
    Fire,
    Cold,
    Lightning,
    Triggerable,
    Trapped,
    Movement,
    DamageOverTime,
    RemoteMined,
    Triggered,
    Vaal,
    Aura,
    CanTargetUnusableCorpse,
    RangedAttack,
    Chaos,
    FixedSpeedProjectile,
    ThresholdJewelArea,
    ThresholdJewelProjectile,
    ThresholdJewelDuration,
    ThresholdJewelRangedAttack,
    Channel,
    DegenOnlySpellDamage,
    InbuiltTrigger,
    Golem,
    Herald,
    AuraAffectsEnemies,
    NoRuthless,
    ThresholdJewelSpellDamage,
    Cascadable,
    ProjectilesFromUser,
    MirageArcherCanUse,
    ProjectileSpiral,
    SingleMainProjectile,
    MinionsPersistWhenSkillRemoved,
    ProjectileNumber,
    Warcry,
    Instant,
    Brand,
    DestroysCorpse,
    NonHitChill,
    ChillingArea,
    AppliesCurse,
    CanRapidFire,
    AuraDuration,
    AreaSpell,
    OR,
    AND,
    NOT,
    AppliesMaim,
    CreatesMinion,
    Guard,
    Travel,
    Blink,
    CanHaveBlessing,
    ProjectilesNotFromUser,
    AttackInPlaceIsDefault,
    Nova,
    InstantNoRepeatWhenHeld,
    InstantShiftAttackForLeftMouse,
    AuraNotOnCaster,
    Banner,
    Rain,
    Cooldown,
    ThresholdJewelChaining,
    Slam,
    Stance,
    NonRepeatable,
    OtherThingUsesSkill,
    Steel,
    Hex,
    Mark,
    Aegis,
    Orb,
    KillNoDamageModifiers,
    RandomElement,
    LateConsumeCooldown,
    Arcane,
    FixedCastTime,
    RequiresOffHandNotWeapon,
    Link,
    Blessing,
    ZeroReservation,
    DynamicCooldown,
    Microtransaction,
    OwnerCannotUse,
    ProjectilesNumberModifiersNotApplied,
    TotemsAreBallistae,
    SkillGrantedBySupport,
    PreventHexTransfer,
    MinionsAreUndamagable,
    InnateTrauma,
    DualWieldRequiresDifferentTypes,
    NoVolley,
    Retaliation,
    NeverExertable,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, IntoStaticStr)]
pub enum DamageType {
    Physical,
    Cold,
    Fire,
    Lightning,
    Chaos,
}

pub struct DamageGroup {
    pub stat_id: StatId,
    pub added_min_id: StatId,
    pub added_max_id: StatId,
    pub base_min_id: StatId,
    pub base_max_id: StatId,
    pub min_id: StatId,
    pub max_id: StatId,
    pub damage_type: DamageType,
}

impl DamageGroup {
    const fn new(stat_id: StatId, added_min_id: StatId, added_max_id: StatId, base_min_id: StatId, base_max_id: StatId, min_id: StatId, max_id: StatId, damage_type: DamageType) -> Self {
        DamageGroup {
            stat_id,
            added_min_id,
            added_max_id,
            base_min_id,
            base_max_id,
            min_id,
            max_id,
            damage_type,
        }
    }
}

pub const DAMAGE_GROUPS: [DamageGroup; 5] = [
    DamageGroup::new(StatId::PhysicalDamage, StatId::AddedMinPhysicalDamage, StatId::AddedMaxPhysicalDamage, StatId::BaseMinPhysicalDamage, StatId::BaseMaxPhysicalDamage, StatId::MinPhysicalDamage, StatId::MaxPhysicalDamage, DamageType::Physical),
    DamageGroup::new(StatId::FireDamage, StatId::AddedMinFireDamage, StatId::AddedMaxFireDamage, StatId::BaseMinFireDamage, StatId::BaseMaxFireDamage, StatId::MinFireDamage, StatId::MaxFireDamage, DamageType::Fire),
    DamageGroup::new(StatId::ColdDamage, StatId::AddedMinColdDamage, StatId::AddedMaxColdDamage, StatId::BaseMinColdDamage, StatId::BaseMaxColdDamage, StatId::MinColdDamage, StatId::MaxColdDamage, DamageType::Cold),
    DamageGroup::new(StatId::LightningDamage, StatId::AddedMinLightningDamage, StatId::AddedMaxLightningDamage, StatId::BaseMinLightningDamage, StatId::BaseMaxLightningDamage, StatId::MinLightningDamage, StatId::MaxLightningDamage, DamageType::Lightning),
    DamageGroup::new(StatId::ChaosDamage, StatId::AddedMinChaosDamage, StatId::AddedMaxChaosDamage, StatId::BaseMinChaosDamage, StatId::BaseMaxChaosDamage, StatId::MinChaosDamage, StatId::MaxChaosDamage, DamageType::Chaos),
];

lazy_static! {
    pub static ref GEMS: FxHashMap<String, GemData> =
        bincode::deserialize(include_bytes!("../../data/gems.bc")).expect("Failed to deserialize GEMS");
    pub static ref ITEMS: FxHashMap<String, BaseItem> =
        bincode::deserialize(include_bytes!("../../data/base_items.bc")).expect("Failed to deserialize base items");
    pub static ref TREE: TreeData =
        bincode::deserialize(include_bytes!("../../data/tree.bc")).expect("Failed to deserialize tree");
    pub static ref MONSTER_STATS: FxHashMap<i64, MonsterStats> =
        bincode::deserialize(include_bytes!("../../data/default_monster_stats.bc")).expect("Failed to deserialize default monster stats");
}
