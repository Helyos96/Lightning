pub mod base_item;
pub mod default_monster_stats;
pub mod gem;
pub mod tree;

use base_item::BaseItem;
use default_monster_stats::MonsterStats;
use gem::GemData;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use tree::TreeData;
use std::error::Error;
use std::fs;
use std::io;
use serde::{Deserialize, Serialize};

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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum DamageType {
    Physical,
    Cold,
    Fire,
    Lightning,
    Chaos,
}

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
