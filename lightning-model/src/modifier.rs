use crate::gem::{Gem, GemTag};
use crate::data::ActiveSkillTypes;
use crate::item::{self, Item, ItemClass};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
/// 2 ways to parse a mod:
///
/// 1. "Automatic": make sure all parts of your mod are declared
///    in TAGS, STATS, BEGINNINGS and ENDINGS.
/// 2. (todo) "Exotic": one-shot parsing of the entire mod
///    through SPECIALS.
///
/// All strings need to be lowercase.
use std::ops::Neg;
use std::str::FromStr;
use std::sync::Mutex;

lazy_static! {
    // Currently limited to one word,
    // need to change parse_stat otherwise.
    static ref TAGS: FxHashMap<&'static str, GemTag> = {
        let mut map = FxHashMap::default();
        map.insert("spell", GemTag::Spell);
        map.insert("melee", GemTag::Melee);
        map.insert("attack", GemTag::Attack);
        map.insert("projectile", GemTag::Projectile);
        map.insert("brand", GemTag::Brand);
        map.insert("mine", GemTag::Mine);
        map.insert("trap", GemTag::Trap);
        map.insert("curse", GemTag::Curse);
        map.insert("minion", GemTag::Minion);
        map.insert("totem", GemTag::Totem);
        map
    };
    static ref DTS: FxHashMap<&'static str, DamageType> = {
        let mut map = FxHashMap::default();
        map.insert("physical", DamageType::Physical);
        map.insert("fire", DamageType::Fire);
        map.insert("cold", DamageType::Cold);
        map.insert("lightning", DamageType::Lightning);
        map.insert("chaos", DamageType::Chaos);
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

const ENDINGS_GEMTAGS: [(&str, GemTag); 3] = [
    ("of aura skills", GemTag::Aura),
    ("with attack skills", GemTag::Attack),
    ("of skills", GemTag::Active_Skill),
];

lazy_static! {
    static ref ENDINGS_WEAPON_RESTRICTIONS: [(&'static str, FxHashSet<ItemClass>); 8] = [
        ("with axes", hset![ItemClass::OneHandAxe, ItemClass::TwoHandAxe]),
        ("with swords", hset![ItemClass::OneHandSword, ItemClass::TwoHandSword, ItemClass::ThrustingOneHandSword]),
        ("with maces", hset![ItemClass::OneHandMace, ItemClass::TwoHandMace]),
        ("with two handed melee weapons", hset![ItemClass::TwoHandSword, ItemClass::TwoHandMace, ItemClass::TwoHandAxe]),
        ("while holding a shield", hset![ItemClass::Shield]),
        ("while wielding a staff", hset![ItemClass::Staff]),
        ("with bow skills", hset![ItemClass::Bow]),
        ("with bows", hset![ItemClass::Bow]),
    ];
}

lazy_static! {
    static ref BEGINNINGS: Vec<(Regex, Box<dyn Fn(&Captures) -> Option<Vec<Mod>> + Send + Sync>)> = vec![
        (
            regex!(r"^(minions (?:have|deal) )?([0-9]+)% increased ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                let insert_minion_tag = c.get(1).is_some();
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0.to_string(),
                        typ: Type::Inc,
                        amount: i64::from_str(&c[2]).unwrap(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% decreased ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0.to_string(),
                        typ: Type::Inc,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^(minions have )?((\+|-)?[0-9]+)%? (?:to )?(?:all )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[4])?;
                let insert_minion_tag = c.get(1).is_some();
                Some(stat_tags.iter().map(|s| {
                    let mut ret = Mod {
                        stat: s.0.to_string(),
                        typ: Type::Base,
                        amount: i64::from_str(&c[2]).unwrap(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    };
                    if insert_minion_tag {
                        ret.tags.insert(GemTag::Minion);
                    }
                    ret
                }).collect())
            })
        ), (
            regex!(r"^\-([0-9]+)%? (to )?([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[3])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0.to_string(),
                        typ: Type::Base,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% more ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0.to_string(),
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^([0-9]+)% less ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags = parse_stat(&c[2])?;
                Some(stat_tags.iter().map(|s| {
                    Mod {
                        stat: s.0.to_string(),
                        typ: Type::More,
                        amount: i64::from_str(&c[1]).unwrap().neg(),
                        tags: s.1.clone(),
                        dt: s.2,
                        ..Default::default()
                    }
                }).collect())
            })
        ), (
            regex!(r"^\+([0-9]+)%? to ([a-z ]+) and ([a-z ]+)$"),
            Box::new(|c| {
                let stat_tags_1 = parse_stat_nomulti(&c[2])?;
                let stat_tags_2 = parse_stat_nomulti(&c[3])?;
                Some(vec![Mod {
                    stat: stat_tags_1.0.to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_1.1,
                    dt: stat_tags_1.2,
                    ..Default::default()
                }, Mod {
                    stat: stat_tags_2.0.to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    tags: stat_tags_2.1,
                    dt: stat_tags_2.2,
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
                    dt: Some(*dt_1),
                    ..Default::default()
                }, Mod {
                    stat: "resistance".to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    dt: Some(*dt_2),
                    ..Default::default()
                }])
            })
        ), (
            regex!(r"^adds ([0-9]+) to ([0-9]+) ([a-z]+) damage$"),
            Box::new(|c| {
                let dt = DTS.get(&c[3])?;
                Some(vec![Mod {
                    stat: "minimum damage".to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[1]).unwrap(),
                    dt: Some(*dt),
                    ..Default::default()
                }, Mod {
                    stat: "maximum damage".to_string(),
                    typ: Type::Base,
                    amount: i64::from_str(&c[2]).unwrap(),
                    dt: Some(*dt),
                    ..Default::default()
                }])
            })
        ),
    ];

    static ref MULTISTATS: FxHashMap<&'static str, Vec<&'static str>> = {
        let mut map = FxHashMap::default();
        map.insert("attributes", vec!["strength", "dexterity", "intelligence"]);
        map.insert("maximum elemental resistances", vec!["maximum fire resistance", "maximum cold resistance", "maximum lightning resistance"]);
        map.insert("elemental resistances", vec!["fire resistance", "cold resistance", "lightning resistance"]);
        map.insert("maximum resistances", vec!["maximum fire resistance", "maximum cold resistance", "maximum lightning resistance", "maximum chaos resistance"]);
        map.insert("resistances", vec!["fire resistance", "cold resistance", "lightning resistance", "chaos resistance"]);
        map.insert("elemental damage", vec!["fire damage", "cold damage", "lightning damage"]);
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
        "projectile speed",

        "chance to block attack damage",
        "chance to block spell damage",

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
        "chance to suppress spell damage",
        "chance to avoid being stunned",

        "maximum fire resistance",
        "maximum cold resistance",
        "maximum lightning resistance",
        "maximum chaos resistance",
        "resistance",

        "flask charges gained",
        "flask effect duration",
    ];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Type {
    #[default]
    Base,
    Inc,
    More,
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
    Tag(GemTag),
    Weapon(FxHashSet<ItemClass>),
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

#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    #[default]
    Innate,
    Node(u16),
    Mastery((u16, u16)),
    Item,
    Gem,
}

#[derive(Default, Debug, Clone)]
pub struct Mod {
    pub stat: String,
    pub typ: Type,
    pub amount: i64,
    pub flags: Vec<Mutation>,
    pub tags: FxHashSet<GemTag>,
    pub dt: Option<DamageType>,
    pub source: Source,
    pub weapons: FxHashSet<ItemClass>
}

fn parse_ending(m: &str) -> Option<(usize, Ending)> {
    for ending in ENDINGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Mutation(ending.1.clone())));
        }
    }
    for ending in ENDINGS_GEMTAGS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Tag(ending.1)));
        }
    }
    for ending in ENDINGS_WEAPON_RESTRICTIONS.iter() {
        if m.ends_with(ending.0) {
            return Some((ending.0.len(), Ending::Weapon(ending.1.clone())));
        }
    }

    None
}

fn parse_stat_nomulti(input: &str) -> Option<(&str, FxHashSet<GemTag>, Option<DamageType>)> {
    let mut tags = hset![];
    let mut dt = None;

    let stat = &STATS.iter().find_map(|s| {
        if input.ends_with(s) {
            return Some(*s);
        }
        None
    })?;

    let remainder = &input[0..input.len() - stat.len()];

    for chunk in remainder.split_terminator(' ') {
        if let Some(t) = TAGS.get(chunk) {
            tags.insert(*t);
        } else if let Some(dt_parsed) = DTS.get(chunk) {
            if dt.is_some() {
                eprintln!("ERR: stat {input} has multiple damage types");
            }
            dt = Some(*dt_parsed);
        } else {
            return None;
        }
    }

    Some((stat.to_owned(), tags, dt))
}

/// Attempts to parse a chunk like "melee physical damage"
fn parse_stat(input: &str) -> Option<Vec<(&str, FxHashSet<GemTag>, Option<DamageType>)>> {
    if let Some(stats) = MULTISTATS.get(input) {
        let mut ret = vec![];
        for stat in stats {
            if let Some(parse) = parse_stat_nomulti(stat) {
                ret.push(parse);
            }
        }
        if ret.is_empty() {
            return None;
        } else {
            return Some(ret);
        }
    }

    if let Some(stat) = parse_stat_nomulti(input) {
        return Some(vec![stat]);
    }

    None
}

lazy_static! {
    static ref CACHE: Mutex<FxHashMap<String, Option<Vec<Mod>>>> = Mutex::new(FxHashMap::default());
}

/// Attempts to parse a modifier like "30℅ increased poison damage while focussed"
/// 1. todo: try to match the entire string against SPECIALS
/// 2. if not special, parse right to left:
///    2.1. any amount of ENDINGS
///    2.2. a BEGINNING
pub fn parse_mod(input: &str, source: Source) -> Option<Vec<Mod>> {
    if let Some(mods_opt) = CACHE.lock().unwrap().get(input) {
        match mods_opt {
            Some(mods) => return Some(mods.to_owned()),
            None => return None,
        }
    }

    let mut m = &input[0..];
    let mut flags = vec![];
    let mut tags = hset![];
    let mut weapons = hset![];

    while let Some(ending) = parse_ending(&m.to_lowercase()) {
        m = &m[0..m.len() - ending.0 - 1];
        match ending.1 {
            Ending::Mutation(flag) => {
                flags.push(flag);
            }
            Ending::Tag(tag) => {
                tags.insert(tag);
            }
            Ending::Weapon(weapon) => {
                weapons.extend(weapon);
            }
        }
    }

    for begin in BEGINNINGS.iter() {
        if let Some(cap) = begin.0.captures(&m.to_lowercase()) {
            if let Some(mut mods) = begin.1(&cap) {
                for modifier in &mut mods {
                    modifier.tags.extend(tags.clone());
                    modifier.flags.extend(flags.clone());
                    modifier.weapons.extend(weapons.clone());
                    modifier.source = source;
                }
                CACHE.lock().unwrap().insert(input.to_string(), Some(mods.clone()));
                println!("Success: {}: {:?}", input, &mods);
                return Some(mods);
            }
        }
    }

    println!("failed: {input}");
    CACHE.lock().unwrap().insert(input.to_string(), None);
    None
}

#[test]
fn test_parse() {
    assert!(parse_mod("50% increased damage", Source::Innate).is_some());
    assert!(parse_mod("50% decreased damage", Source::Innate).is_some());
    assert!(parse_mod("50% more damage", Source::Innate).is_some());
    assert!(parse_mod("50% less damage", Source::Innate).is_some());
    assert!(parse_mod("+5 damage", Source::Innate).is_some());
    assert!(parse_mod("-5 damage", Source::Innate).is_some());
    assert!(parse_mod("+1 maximum life per level", Source::Innate).is_some());
    assert!(parse_mod("+5% to cold resistance", Source::Innate).is_some());
    assert!(parse_mod("-5% fire resistance", Source::Innate).is_some());
    assert!(parse_mod("50% increased melee physical damage", Source::Innate).is_some());
    assert!(parse_mod("50% increased melee physical damage per level", Source::Innate).is_some());
}
