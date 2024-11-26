use rustc_hash::{FxHashMap, FxHashSet};
use crate::{data::{base_item::ItemClass, gem::GemTag}, modifier::{Mod, Type}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StatId {
    #[default]
    Strength,
    Dexterity,
    Intelligence,
    Attributes,
    AttackSpeed,
    CastSpeed,
    WarcrySpeed,
    CooldownRecoverySpeed,
    ProjectileSpeed,
    TrapThrowingSpeed,
    ChanceToBlockAttackDamage,
    ChanceToBlockSpellDamage,
    ChanceToSuppressSpellDamage,
    FireDamageOverTimeMultiplier,
    ColdDamageOverTimeMultiplier,
    ChaosDamageOverTimeMultiplier,
    PhysicalDamageOverTimeMultiplier,
    DamageOverTimeMultiplier,
    FireDamageOverTime,
    ColdDamageOverTime,
    ChaosDamageOverTime,
    PhysicalDamageOverTime,
    DamageOverTime,
    MinFireDamage,
    MaxFireDamage,
    FireDamage,
    ColdDamage,
    LightningDamage,
    ChaosDamage,
    MinPhysicalDamage,
    MaxPhysicalDamage,
    PhysicalDamage,
    Damage,
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
    EnergyShield,
    EnergyShieldRechargeRate,
    LifeRegenerationRate,
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
    LifeRegeneration,
    LifeRegenerationPct,
    PassiveSkillPoints,
    FireDamagePen,
    LightningDamagePen,
    ChaosDamagePen,
    ColdDamagePen,
    ChanceToHit,
    ChanceToEvade,
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub stats: FxHashMap<StatId, Stat>,
}

impl Stats {
    pub fn stat(&self, s: StatId) -> Stat {
        self.stats.get(&s).cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct Stat {
    base: i64,
    inc: i64,
    more: i64,
    overrid: Option<i64>,
    mods: Vec<Mod>,
}

/// Computes a stat from a mod list
/// WARNING: doesn't take into account mutations or conditions
pub fn calc_stat(stat_id: StatId, mods: &[Mod], tags: &FxHashSet<GemTag>) -> Stat {
    let mut stat = Stat::default();

    for m in mods
        .iter()
        .filter(|m| m.stat == stat_id && tags.is_superset(&m.tags))
    {
        stat.adjust(m.typ, m.amount, m);
    }

    stat
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            base: 0,
            inc: 0,
            more: 100,
            overrid: None,
            mods: vec![],
        }
    }
}

impl Stat {
    pub fn adjust_mod(&mut self, m: &Mod) {
        self.adjust(m.typ, m.amount, m);
    }

    pub fn adjust(&mut self, t: Type, amount: i64, m: &Mod) {
        match t {
            Type::Base => self.base += amount,
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
        let mut modifier = m.to_owned();
        modifier.amount = amount;
        self.mods.push(modifier);
    }

    fn mult(&self) -> i64 {
        (100 + self.inc) * self.more
    }

    fn val100(&self) -> i64 {
        if let Some(overrid) = self.overrid {
            overrid * 100
        } else {
            (self.base * self.mult()) / 100
        }
    }

    pub fn with_weapon(&self, weapon: ItemClass) -> Stat {
        let mut stat = Stat::default();

        for m in &self.mods {
            if m.weapons.is_empty() || m.weapons.contains(&weapon) {
                stat.adjust(m.typ, m.amount, m);
            }
        }

        stat
    }

    pub fn val(&self) -> i64 {
        self.val100() / 100
    }

    pub fn val_rounded_up(&self) -> i64 {
        (self.val100() as f64 / 100.0).ceil() as i64
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
}
