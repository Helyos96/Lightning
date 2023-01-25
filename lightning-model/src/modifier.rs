use crate::gem::{Gem, Tag};
use crate::item::Item;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
/// 2 ways to parse a mod:
///
/// 1. "Automatic": make sure all parts of your mod are declared
/// in TAGS, STATS, BEGINNINGS and ENDINGS.
/// 2. (todo) "Exotic": one-shot parsing of the entire mod
/// through SPECIALS.
///
/// All strings need to be lowercase.
use std::ops::Neg;
use std::str::FromStr;
use std::sync::Mutex;

lazy_static! {
    // Currently limited to one word,
    // need to change parse_stat otherwise.
    static ref TAGS: FxHashMap<&'static str, Tag> = {
        let mut map = FxHashMap::default();
        map.insert("spell", Tag::Spell);
        map.insert("melee", Tag::Melee);
        map.insert("attack", Tag::Attack);
        map.insert("projectile", Tag::Projectile);
        map.insert("brand", Tag::Brand);
        map.insert("mine", Tag::Mine);
        map.insert("trap", Tag::Trap);
        map.insert("curse", Tag::Curse);
        map.insert("minion", Tag::Minion);
        map.insert("totem", Tag::Totem);
        map
    };
    static ref DTS: FxHashMap<&'static str, FxHashSet<DamageType>> = {
        let mut map = FxHashMap::default();
        map.insert("physical", hset![DamageType::Physical]);
        map.insert("fire", hset![DamageType::Fire]);
        map.insert("cold", hset![DamageType::Cold]);
        map.insert("lightning", hset![DamageType::Lightning]);
        map.insert("chaos", hset![DamageType::Chaos]);
        map.insert("elemental", hset![DamageType::Fire, DamageType::Cold, DamageType::Lightning]);
        map
    };
}

const ENDINGS: [(&str, Mutation); 4] = [
    ("per level", Mutation::MultiplierProperty((1, Property::Level))),
    (
        "per frenzy charge",
        Mutation::MultiplierProperty((1, Property::FrenzyCharges)),
    ),
    (
        "per power charge",
        Mutation::MultiplierProperty((1, Property::PowerCharges)),
    ),
    (
        "per endurance charge",
        Mutation::MultiplierProperty((1, Property::EnduranceCharges)),
    ),
];

const ENDINGS_TAGS: [(&str, Tag); 5] = [
    ("of aura skills", Tag::Aura),
    ("with attack skills", Tag::Attack),
    ("with bow skills", Tag::Bow),
    ("with bows", Tag::Bow),
    ("of skills", Tag::Active_Skill),
];

lazy_static! {
    static ref BEGINNINGS: Vec<(Regex, Box<dyn Fn(&Captures) -> Option<Vec<Mod>> + Send + Sync>)> = vec![
        (
            regex!(r"^(minions (?:have|deal) )?([0-9]+)% increased ([a-z ]+)$"),
            Box::new(|c| {
                let mut stat_tags = parse_stat(&c[3])?;
                if c.get(1).is_some() {
                    stat_tags.1.insert(Tag::Minion);
                }
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::Inc,
                        amount: i64::from_str(&c[2]).unwrap(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% decreased ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::Inc,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^(minions have )?\+([0-9]+)%? (?:to )?(?:all )?([a-z ]+)$"),
            Box::new(|c| {
                let mut stat_tags = parse_stat(&c[3])?;
                if c.get(1).is_some() {
                    stat_tags.1.insert(Tag::Minion);
                }
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::Base,
                        amount: i64::from_str(&c[2]).unwrap(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^\-([0-9]+)%? (to )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::Base,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% more ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% less ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.0.iter().map(|s| {
                    Mod {
                        stat: s.to_string(),
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: stat_tags.1.clone(),
                        dts: stat_tags.2.clone(),
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z ]+) and ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat(&c[2])?;
                let stat_tags_2 = parse_stat(&c[3])?;
                Some(vec![Mod {
                    stat: stat_tags_1.0[0].to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_1.1,
                    dts: stat_tags_1.2,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0[0].to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_2.1,
                    dts: stat_tags_2.2,
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z]+) and ([a-z]+) resistances$"),
            Box::new(|c| {
                let dt_1 = DTS.get(&c[2])?;
                let dt_2 = DTS.get(&c[3])?;
                Some(vec![Mod {
                    stat: "resistance".to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    dts: dt_1.clone(),
                    ..Default::default()
                }, Mod {
                    stat: "resistance".to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    dts: dt_2.clone(),
                    ..Default::default()
                }])
            })
        ),
    ];

    static ref MULTISTATS: FxHashMap<&'static str, Vec<&'static str>> = {
        let mut map = FxHashMap::default();
        map.insert("attributes", vec!["strength", "dexterity", "intelligence"]);
        map.insert("resistances", vec!["resistance"]);
        map
    };
    // Order is important for overlapping stats
    // like "area of effect" and "effect"
    static ref STATS: Vec<&'static str> = vec![
        "strength",
        "dexterity",
        "intelligence",
        "attributes",

        "attack speed",
        "cast speed",
        "warcry speed",
        "cooldown recovery speed",

        "area of effect",
        "effect",
        "damage over time multiplier",
        "damage over time",
        "damage",
        "accuracy rating",
        "movement speed",
        "skill effect duration",
        "duration",

        "maximum frenzy charges",
        "maximum power charges",
        "maximum endurance charges",

        "maximum life",
        "maximum mana",
        "maximum rage",
        "maximum energy shield",
        "energy shield recharge rate",
        "life regeneration rate",
        "mana regeneration rate",
        "mana reservation efficiency",

        "critical strike chance",
        "critical strike multiplier",

        "armour",
        "evasion rating",
        "stun threshold",
        "resistance",
        "resistances",

        "flask charges gained",
    ];
}

#[derive(Debug, Copy, Clone)]
pub enum Type {
    Base,
    Inc,
    More,
}

impl Default for Type {
    fn default() -> Self {
        Type::Base
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Property {
    Level,
    PowerCharges,
    FrenzyCharges,
    EnduranceCharges,
}

pub enum Ending {
    Mutation(Mutation),
    Tag(Tag),
}

#[derive(Debug, Clone)]
pub enum Mutation {
    MultiplierStat((i64, String)),
    MultiplierProperty((i64, Property)),
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum DamageType {
    Physical,
    Cold,
    Fire,
    Lightning,
    Chaos,
}

#[derive(Debug, Clone)]
pub enum Source {
    Innate,
    Node(u16),
    Mastery((u16, u16)),
    Item(Item),
    Gem(Gem),
}

impl Default for Source {
    fn default() -> Self {
        Self::Innate
    }
}

#[derive(Default, Debug, Clone)]
pub struct Mod {
    pub stat: String,
    pub typ: Type,
    pub amount: i64,
    pub flags: Vec<Mutation>,
    pub tags: FxHashSet<Tag>,
    pub dts: FxHashSet<DamageType>,
    pub source: Source,
}

fn parse_ending(m: &str) -> Option<(usize, Ending)> {
    for ending in ENDINGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Mutation(ending.1.clone())));
        }
    }
    for ending in ENDINGS_TAGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Tag(ending.1)));
        }
    }

    None
}

/// Attempts to parse a chunk like "melee physical damage"
fn parse_stat(mut input: &str) -> Option<(Vec<&str>, FxHashSet<Tag>, FxHashSet<DamageType>)> {
    let mut tags = hset![];
    let mut dts = hset![];

    let stats = &vec![STATS.iter().find_map(|s| {
        if input.ends_with(s) {
            return Some(*s);
        }
        None
    })?];

    input = &input[0..input.len() - stats[0].len()];
    let stats = MULTISTATS.get(stats[0]).unwrap_or(stats);

    for chunk in input.split_terminator(' ') {
        if let Some(t) = TAGS.get(chunk) {
            tags.insert(*t);
        } else if let Some(dt) = DTS.get(chunk) {
            dts = dt.clone();
        } else {
            return None;
        }
    }

    Some((stats.to_owned(), tags, dts))
}

lazy_static! {
    static ref CACHE: Mutex<FxHashMap<String, Option<Vec<Mod>>>> = Mutex::new(FxHashMap::default());
}

/// Attempts to parse a modifier like "30℅ increased poison damage while focussed"
/// 1. todo: try to match the entire string against SPECIALS
/// 2. if not special, parse right to left:
/// 2.1. any amount of ENDINGS
/// 2.2. a BEGINNING
pub fn parse_mod(input: &str) -> Option<Vec<Mod>> {
    if let Some(mods_opt) = CACHE.lock().unwrap().get(input) {
        match mods_opt {
            Some(mods) => return Some(mods.to_owned()),
            None => return None,
        }
    }

    let mut m = &input[0..];
    let mut flags = vec![];
    let mut tags = hset![];

    while let Some(ending) = parse_ending(&m.to_lowercase()) {
        m = &m[0..m.len() - ending.0 - 1];
        match ending.1 {
            Ending::Mutation(flag) => {
                flags.push(flag);
            }
            Ending::Tag(tag) => {
                tags.insert(tag);
            }
        }
    }

    for begin in BEGINNINGS.iter() {
        if let Some(cap) = begin.0.captures(&m.to_lowercase()) {
            if let Some(mut mods) = begin.1(&cap) {
                for modifier in &mut mods {
                    modifier.tags.extend(tags.clone());
                    modifier.flags.extend(flags.clone());
                }
                CACHE.lock().unwrap().insert(input.to_string(), Some(mods.clone()));
                println!("Success: {}: {:?}", input, &mods);
                return Some(mods);
            }
        }
    }

    println!("failed: {}", input);
    CACHE.lock().unwrap().insert(input.to_string(), None);
    None
}

#[test]
fn test_parse() {
    assert!(parse_mod(&"50% increased damage").is_some());
    assert!(parse_mod(&"50% decreased damage").is_some());
    assert!(parse_mod(&"50% more damage").is_some());
    assert!(parse_mod(&"50% less damage").is_some());
    assert!(parse_mod(&"+5 damage").is_some());
    assert!(parse_mod(&"-5 damage").is_some());
    assert!(parse_mod(&"+1 maximum life per level").is_some());
    assert!(parse_mod(&"+5% to cold resistance").is_some());
    assert!(parse_mod(&"-5% fire resistance").is_some());
    assert!(parse_mod(&"50% increased melee physical damage").is_some());
    assert!(parse_mod(&"50% increased melee physical damage per level").is_some());
}
