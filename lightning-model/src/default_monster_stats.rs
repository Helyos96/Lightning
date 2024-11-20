use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MonsterStats {
	pub accuracy: i64,
	pub ally_life: i64,
	pub armour: i64,
	pub evasion: i64,
	pub life: i64,
	pub physical_damage: f32,
}
