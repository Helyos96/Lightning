use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use super::StatId;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Int {
    Level,
    PowerCharges,
    FrenzyCharges,
    EnduranceCharges,
    Rage,
}

#[derive(Debug, Copy, Clone)]
pub enum Val {
    Val(i64),
    Stat(StatId),
}

#[derive(Debug, Copy, Clone)]
pub struct IntData {
    pub min: Val,
    pub max: Val,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Bool {
    Blinded,
    Onslaught,
    Fortified,
    DealtCritRecently,
    Leeching,
    OnFullLife,
    OnLowLife,
}

pub fn int_data(p: Int) -> &'static IntData {
    match p {
        Int::Level => &IntData {min: Val::Val(1), max: Val::Val(100)},
        Int::FrenzyCharges => &IntData {min: Val::Stat(StatId::MinimumFrenzyCharges), max: Val::Stat(StatId::MaximumFrenzyCharges)},
        Int::PowerCharges => &IntData {min: Val::Stat(StatId::MinimumPowerCharges), max: Val::Stat(StatId::MaximumPowerCharges)},
        Int::EnduranceCharges => &IntData {min: Val::Stat(StatId::MinimumEnduranceCharges), max: Val::Stat(StatId::MaximumEnduranceCharges)},
        Int::Rage => &IntData {min: Val::Stat(StatId::MinimumRage), max: Val::Stat(StatId::MaximumRage)},
    }
}
