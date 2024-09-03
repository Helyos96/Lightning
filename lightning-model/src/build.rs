use crate::data::TREE;
use crate::gem::{Gem, Tag};
use crate::item::Item;
use crate::modifier::{DamageType, Mod, Mutation, Property, Type};
use crate::tree::{Class, PassiveTree, TreeData};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub enum Slot {
    Helm,
    BodyArmour,
    Gloves,
    Boots,
    Belt,
    Amulet,
    Weapon,
    Weapon2,
    Ring,
    Ring2,
    Flask(u16), // u16 -> Flask slot
    TreeJewel(u16), // u16 -> Tree node holding the jewel
}

impl TryFrom<(&str, u16)> for Slot {
    type Error = ();

    fn try_from((inventory_id, x): (&str, u16)) -> Result<Self, Self::Error> {
        match inventory_id {
            "Helm" => Ok(Slot::Helm),
            "BodyArmour" => Ok(Slot::BodyArmour),
            "Gloves" => Ok(Slot::Gloves),
            "Boots" => Ok(Slot::Boots),
            "Belt" => Ok(Slot::Belt),
            "Amulet" => Ok(Slot::Amulet),
            "Weapon" => Ok(Slot::Weapon),
            "Weapon2" => Ok(Slot::Weapon2),
            "Ring" => Ok(Slot::Ring),
            "Ring2" => Ok(Slot::Ring2),
            "Flask" => {
                if x <= 4 {
                    Ok(Slot::Flask(x))
                } else {
                    Err(())
                }
            }
            "PassiveJewels" => {
                if let Some(node) = TREE.jewel_slots.get(x as usize) {
                    Ok(Slot::TreeJewel(*node))
                } else {
                    Err(())
                }
            }
            _ => Err(())
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GemLink {
    pub active_gems: Vec<Gem>,
    pub support_gems: Vec<Gem>,
    pub slot: Slot,
}

#[derive(Debug, Clone)]
pub struct Stat {
    base: i64,
    inc: i64,
    more: i64,
    mods: Vec<Mod>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Build {
    pub name: String,
    pub ascendancy: i32,
    pub level: i32,
    pub gem_links: Vec<GemLink>,
    pub equipment: FxHashMap<Slot, Item>, // todo: HashMap Slot
    pub inventory: Vec<Item>,
    pub tree: PassiveTree,
}

impl Build {
    pub fn new_player() -> Build {
        Build {
            name: "Untitled Build".to_string(),
            ascendancy: 0,
            level: 1,
            ..Default::default()
        }
    }

    /// Returns mods from the following sources:
    /// Innate, Passive Tree, Items, Global Skills (Auras..)
    /// todo: add some caching to not parse & collect all mods
    /// every time.
    pub fn calc_mods(&self, include_global: bool) -> Vec<Mod> {
        let class_data = &TREE.classes[&self.tree.class];
        let mut mods = vec![
            Mod {
                stat: "maximum life".to_string(),
                typ: Type::Base,
                amount: 12,
                flags: vec![Mutation::MultiplierProperty((1, Property::Level))],
                ..Default::default()
            },
            Mod {
                stat: "maximum life".to_string(),
                typ: Type::Base,
                amount: 38,
                ..Default::default()
            },
            Mod {
                stat: "maximum life".to_string(),
                typ: Type::Base,
                amount: 1,
                flags: vec![Mutation::MultiplierStat((2, "strength".to_string()))],
                ..Default::default()
            },
            Mod {
                stat: "maximum frenzy charges".to_string(),
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: "maximum power charges".to_string(),
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: "maximum endurance charges".to_string(),
                typ: Type::Base,
                amount: 3,
                ..Default::default()
            },
            Mod {
                stat: "strength".to_string(),
                typ: Type::Base,
                amount: class_data.base_str,
                ..Default::default()
            },
            Mod {
                stat: "dexterity".to_string(),
                typ: Type::Base,
                amount: class_data.base_dex,
                ..Default::default()
            },
            Mod {
                stat: "intelligence".to_string(),
                typ: Type::Base,
                amount: class_data.base_int,
                ..Default::default()
            },
        ];
        mods.extend(self.tree.calc_mods());
        for item in self.equipment.values() {
            mods.extend(item.calc_nonlocal_mods());
        }
        if include_global {
            for gl in &self.gem_links {
                for ag in gl.active_gems.iter().filter(|g| g.data().tags.contains(&Tag::Aura)) {
                    mods.extend(ag.calc_mods());
                }
            }
        }
        mods
    }

    pub fn calc_stat(&self, stat_str: &str, mods: &[Mod], tags: &FxHashSet<Tag>, dt: Option<DamageType>) -> Stat {
        let mut stat = Stat::default();

        for m in mods
            .iter()
            .filter(|m| m.stat == stat_str && tags.is_superset(&m.tags) && (m.dt.is_none() || m.dt == dt))
        {
            let mut amount = m.amount;
            for f in &m.flags {
                match f {
                    Mutation::MultiplierProperty(_mp) => {
                        amount *= 1
                    }
                    Mutation::MultiplierStat(_) => {
                        // todo
                    }
                }
            }
            stat.adjust(m.typ, amount, m);
        }

        stat
    }

    /// Calc all stats irrelevant of damage types.
    /// For any stat that may be affected by damage type,
    /// use calc_stat_dmg.
    pub fn calc_stats(&self, mods: &[Mod], tags: &FxHashSet<Tag>) -> FxHashMap<String, Stat> {
        let mut stats: FxHashMap<String, Stat> = Default::default();
        let mut mods_sec_pass = vec![];
        //let mut mods_third_pass = vec![];

        for m in mods.iter().filter(|m| tags.is_superset(&m.tags) || m.stat == "effect") {
            if !m.flags.is_empty() {
                mods_sec_pass.push(m);
                continue;
            }
            stats.entry(m.stat.clone()).or_default().adjust(m.typ, m.amount, m);
            /*if m.stat == "effect" {
                mods_third_pass.push(m);
            }*/
        }

        for m in mods_sec_pass {
            let mut amount = m.amount;
            for f in &m.flags {
                match f {
                    Mutation::MultiplierProperty(mp) => {
                        amount *= match mp.1 {
                            Property::Level => self.level as i64,
                            _ => 1,
                        }
                    }
                    Mutation::MultiplierStat(ms) => {
                        amount *= match stats.get(&ms.1) {
                            Some(stat) => stat.val() / ms.0,
                            None => 1,
                        }
                    }
                }
            }
            stats.entry(m.stat.clone()).or_default().adjust(m.typ, amount, m);
            /*if m.stat == "effect" {
                mods_third_pass.push(m);
            }*/
        }

        stats
    }
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            base: 0,
            inc: 0,
            more: 100,
            mods: vec![],
        }
    }
}

impl Stat {
    fn adjust(&mut self, t: Type, amount: i64, m: &Mod) {
        match t {
            Type::Base => self.base += amount,
            Type::Inc => self.inc += amount,
            Type::More => self.more = (self.more * (100 + amount)) / 100,
        }
        self.mods.push(m.to_owned());
    }

    fn mult(&self) -> i64 {
        ((100 + self.inc) * self.more) / 100
    }

    fn val100(&self) -> i64 {
        self.base * self.mult()
    }

    pub fn val(&self) -> i64 {
        self.val100() / 100
    }

    pub fn assimilate(&mut self, stat: &Stat) {
        self.base += stat.base;
        self.inc += stat.inc;
        self.more = (self.more * stat.more) / 100;
        self.mods.extend(stat.mods.clone());
    }

    pub fn calc_inv(&self, val: i64) -> i64 {
        (val * 100) / self.mult()
    }
}

#[test]
fn test_build() {
    let player = Build::new_player();
    let stats = player.calc_stats(&player.calc_mods(true), &hset![]);

    assert_eq!(stats["maximum life"].val(), 60);
}
