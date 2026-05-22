use crate::{build::stat::StatId, modifier::{Mod, Type}};
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Buff {
	Onslaught,
	DiamondShrine,
	MassiveShrine,
	ResistanceShrine,
	EchoingShrine,
	AccelerationShrine,
	Malediction,
}

lazy_static! {
	pub static ref BUFF_MODS: FxHashMap<Buff, Vec<Mod>> = {
		use Buff::*;
		let mut ret = FxHashMap::default();
		ret.insert(Onslaught, vec![
			Mod::stat(StatId::AttackSpeed, Type::Inc, 20),
			Mod::stat(StatId::CastSpeed, Type::Inc, 20),
			Mod::stat(StatId::MovementSpeed, Type::Inc, 20),
		]);
		ret.insert(DiamondShrine, vec![
			Mod::stat(StatId::CriticalStrikeChance, Type::Override, 100),
		]);
		ret.insert(MassiveShrine, vec![
			Mod::stat(StatId::AreaOfEffect, Type::Inc, 40),
			Mod::stat(StatId::MaximumLife, Type::Inc, 40),
		]);
		ret.insert(ResistanceShrine, vec![
			Mod::stat(StatId::FireResistance, Type::Base, 50),
			Mod::stat(StatId::ColdResistance, Type::Base, 50),
			Mod::stat(StatId::LightningResistance, Type::Base, 50),
			Mod::stat(StatId::MaximumFireResistance, Type::Base, 10),
			Mod::stat(StatId::MaximumColdResistance, Type::Base, 10),
			Mod::stat(StatId::MaximumLightningResistance, Type::Base, 10),
		]);
		ret.insert(EchoingShrine, vec![
			Mod::stat(StatId::AttackSpeed, Type::More, 100),
			Mod::stat(StatId::CastSpeed, Type::More, 100),
		]);
		ret.insert(AccelerationShrine, vec![
			Mod::stat(StatId::ActionSpeed, Type::Inc, 50),
		]);
		ret.insert(Malediction, vec![
			Mod::stat(StatId::DamageTaken, Type::Inc, 10),
			Mod::stat(StatId::Damage, Type::More, -10),
		]);
		ret
	};
}