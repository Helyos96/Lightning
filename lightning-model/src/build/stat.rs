use enumflags2::BitFlags;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Serialize, Deserialize};
use crate::{data::{base_item::ItemClass, gem::GemTag}, modifier::{Mod, Type}};
use lazy_static::lazy_static;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum_macros::Display, strum_macros::EnumCount)]
pub enum StatId {
    #[default]
    Strength,
    Dexterity,
    Intelligence,
    Attributes,
    ActionSpeed,
    AttackSpeed,
    CastSpeed,
    WarcrySpeed,
    CooldownRecoverySpeed,
    ProjectileSpeed,
    TrapThrowingSpeed,
    MaximumChanceToBlockAttackDamage,
    MaximumChanceToBlockSpellDamage,
    ChanceToBlockAttackDamage,
    ChanceToBlockSpellDamage,
    ChanceToSuppressSpellDamage,
    FireDotMultiplier,
    ColdDotMultiplier,
    ChaosDotMultiplier,
    PhysicalDotMultiplier,
    DotMultiplier,
    FireDamageOverTime,
    ColdDamageOverTime,
    ChaosDamageOverTime,
    PhysicalDamageOverTime,
    DamageOverTime,
    BaseMinFireDamage,
    BaseMaxFireDamage,
    AddedMinFireDamage,
    AddedMaxFireDamage,
    MinFireDamage,
    MaxFireDamage,
    FireDamage,
    BaseMinColdDamage,
    BaseMaxColdDamage,
    AddedMinColdDamage,
    AddedMaxColdDamage,
    MinColdDamage,
    MaxColdDamage,
    ColdDamage,
    BaseMinLightningDamage,
    BaseMaxLightningDamage,
    AddedMinLightningDamage,
    AddedMaxLightningDamage,
    MinLightningDamage,
    MaxLightningDamage,
    LightningDamage,
    BaseMinChaosDamage,
    BaseMaxChaosDamage,
    AddedMinChaosDamage,
    AddedMaxChaosDamage,
    MinChaosDamage,
    MaxChaosDamage,
    ChaosDamage,
    BaseMinPhysicalDamage,
    BaseMaxPhysicalDamage,
    AddedMinPhysicalDamage,
    AddedMaxPhysicalDamage,
    MinPhysicalDamage,
    MaxPhysicalDamage,
    MinDamage,
    MaxDamage,
    PhysicalDamage,
    Damage,
    SpellDamage,
    AreaOfEffect,
    AccuracyRating,
    MovementSpeed,
    SkillEffectDuration,
    Duration,
    ImpaleEffect,
    MinimumFrenzyCharges,
    MinimumPowerCharges,
    MinimumEnduranceCharges,
    MaximumFrenzyCharges,
    MaximumPowerCharges,
    MaximumEnduranceCharges,
    MaximumLife,
    MaximumMana,
    MinimumRage,
    MaximumRage,
    MaximumEnergyShield,
    EnergyShieldRechargeRate,
    LifeRegeneration,
    LifeRegenerationPct,
    LifeRegenerationRate,
    ManaRegeneration,
    ManaRegenerationPct,
    ManaRegenerationRate,
    ManaReservationEfficiency,
    CriticalStrikeChance,
    CriticalStrikeMultiplier,
    Armour,
    EvasionRating,
    StunThreshold,
    ChanceToAvoidBeingStunned,
    MaximumFireResistance,
    MaximumColdResistance,
    MaximumLightningResistance,
    MaximumChaosResistance,
    FireResistance,
    ColdResistance,
    LightningResistance,
    ChaosResistance,
    FlaskChargesGained,
    FlaskEffectDuration,
    FlaskRecoveryRate,
    FlaskChargesUsed,
    ManaCost,
    LifeCost,
    Cost,
    PassiveSkillPoints,
    FireDamagePen,
    LightningDamagePen,
    ChaosDamagePen,
    ColdDamagePen,
    ChanceToHit,
    ChanceToEvade,
    ChanceToDealDoubleDamage,
    PhysicalDamageReduction,
    ShockAsThoughDamage,
    AddedAttackTime,
    AllocatesPassiveSkills,
    AddedPassiveSkillsGrantNode,
    AddedPassivesAreJewelSockets,
    AbyssalSockets,
    MaximumFortification,
    ChanceToPoison,
    ChanceToBleed,
    ChanceToShock,
    ChanceToIgnite,
    ChanceToFreeze,
    PoisonDuration,
    AuraEffect,
    SmallPassiveIncreasedEffect,
    PhysicalToLightningConversion,
    PhysicalToColdConversion,
    PhysicalToFireConversion,
    PhysicalToChaosConversion,
    LightningToColdConversion,
    LightningToFireConversion,
    LightningToChaosConversion,
    ColdToFireConversion,
    ColdToChaosConversion,
    FireToChaosConversion,
    FasterIgnite,
    ItemEffectDistanceClass,
    Effect,
    FlaskEffect,
    PhysicalAsFireExtra,
    PhysicalAsColdExtra,
    PhysicalAsLightningExtra,
    PhysicalAsChaosExtra,
    LightningAsColdExtra,
    LightningAsFireExtra,
    LightningAsChaosExtra,
    ColdAsFireExtra,
    ColdAsChaosExtra,
    FireAsChaosExtra,
}

impl StatId {
    pub fn as_usize(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub stats: FxHashMap<StatId, Stat>,
}

lazy_static! {
    static ref DEFAULT_STAT: Stat = Stat::default();
}

impl Stats {
    pub fn stat(&self, s: StatId) -> &Stat {
        self.stats.get(&s).unwrap_or(&DEFAULT_STAT)
    }

    pub fn val(&self, s: StatId) -> i64 {
        if let Some(stat) = self.stats.get(&s) {
            stat.val()
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stat {
    pub base: i64,
    pub flat: i64,
    pub inc: i64,
    pub more: i64,
    pub overrid: Option<i64>,
    pub mods: Vec<Mod>,
    pub mods_disabled: Vec<Mod>,
}

/// Computes a stat from a mod list
/// WARNING: doesn't take into account mutations, conditions or tags
pub fn calc_stat(stat_id: StatId, mods: &[Mod]) -> Stat {
    let mut stat = Stat::default();

    for m in mods.iter().filter(|m| if let Some(stat) = m.as_stat() && stat.stat == stat_id { true } else { false }) {
        stat.adjust_mod(m);
    }

    stat
}

/// Computes stats from a mod list
/// WARNING: doesn't take into account mutations, conditions or tags
pub fn calc_stats(mods: &[Mod]) -> Stats {
    let mut stats: FxHashMap<StatId, Stat> = FxHashMap::default();

    for m in mods {
        if let Some(stat) = m.as_stat() {
            stats.entry(stat.stat).or_default().adjust_mod(m);
        }
    }

    Stats { stats }
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            base: 0,
            flat: 0,
            inc: 0,
            more: 100,
            overrid: None,
            mods: vec![],
            mods_disabled: vec![],
        }
    }
}

impl Stat {
    pub fn adjust_mod(&mut self, m: &Mod) {
        if let Some(stat) = m.as_stat() {
            self.adjust(stat.typ, stat.final_amount());
        } else {
            eprintln!("Trying to adjust stat with non-stat mod");
        }
        self.mods.push(m.to_owned());
    }

    pub fn adjust_mod_move(&mut self, m: Mod) {
        if let Some(stat) = m.as_stat() {
            self.adjust(stat.typ, stat.final_amount());
        } else {
            eprintln!("Trying to adjust stat with non-stat mod");
        }
        self.mods.push(m);
    }

    pub fn adjust(&mut self, t: Type, amount: i64) {
        match t {
            Type::Base => self.base += amount,
            Type::Flat => self.flat += amount,
            Type::Inc => self.inc += amount,
            Type::More => self.more = (self.more * (100 + amount)) / 100,
            Type::Override => {
                if let Some(existing_override) = self.overrid {
                    // Keep the lowest override, unsure if correct
                    if amount < existing_override {
                        self.overrid = Some(amount);
                    }
                } else {
                    self.overrid = Some(amount);
                }
            }
        }
    }

    pub fn mult(&self) -> i64 {
        (100 + self.inc) * self.more
    }

    pub fn mult_amount(&self, amount: i64) -> i64 {
        (amount * self.mult()) / 10000
    }

    fn val100(&self) -> i64 {
        if let Some(overrid) = self.overrid {
            overrid * 100
        } else {
            (self.base * self.mult()) / 100 + (self.flat * 100)
        }
    }

    pub fn with_weapon(&self, weapon: Option<ItemClass>) -> Stat {
        let mut stat = Stat::default();

        for m in &self.mods {
            if m.weapons.is_empty() || (weapon.is_some() && m.weapons.contains(weapon.unwrap())) {
                stat.adjust_mod(m);
            }
        }

        stat
    }

    pub fn with_tags(&self, tags: BitFlags<GemTag>) -> Stat {
        let mut stat = Stat::default();

        for m in self.mods.iter().chain(&self.mods_disabled).filter(|m| m.tags.contains(tags)) {
            stat.adjust_mod(m);
        }

        stat
    }

    pub fn val(&self) -> i64 {
        self.val100() / 100
    }

    /// Rounds the value up (ceiling)
    pub fn val_ceil(&self) -> i64 {
        (self.val100() + 99) / 100
    }

    /// Rounds the value up or down depending on remainder (0.50)
    pub fn val_rounded(&self) -> i64 {
        (self.val100() + 50) / 100
    }

    pub fn assimilate(&mut self, stat: &Stat) {
        self.base += stat.base;
        self.inc += stat.inc;
        self.more = (self.more * stat.more) / 100;
        self.mods.extend(stat.mods.clone());
    }

    pub fn val_custom(&self, val: i64) -> i64 {
        (val * self.mult()) / 10000
    }

    pub fn val_custom_inv(&self, val: i64) -> i64 {
        (val * 10000) / self.mult()
    }

    /// Attempts to revert the multipliers from a pre-computed value
    /// It is not possible to retrieve the original value if the multiplier is lower than 10000
    /// with 100% certainty (e.g with a reduced/less effect multiplier), so this is best effort.
    pub fn revert(&self, val: i64) -> i64 {
        let mult = self.mult();
        let mut original_val = (val * 10000) / mult;
        while (original_val * mult) / 10000 < val {
            original_val += 1;
        }
        original_val
    }

    pub fn add_mod_disabled(&mut self, m: &Mod) {
        self.mods_disabled.push(m.to_owned());
    }
}
