#![allow(non_snake_case)]

/// Import build data from pathofexile.com

use crate::build::{self, Build, GemLink, Slot};
use crate::data::base_item::Rarity;
use crate::data::tree::{Ascendancy, Class};
use crate::data::GEMS;
use crate::gem;
use crate::item;
use serde::Deserialize;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::io;
use std::str::FromStr;

#[derive(Deserialize)]
struct Character {
    level: i64,
    #[serde(rename = "class")]
    class_or_ascendancy: String,
}

#[derive(Debug, Deserialize)]
struct Property {
    name: String,
    values: Vec<(String, i32)>,
}

#[derive(Debug, Deserialize)]
struct Item {
    baseType: String,
    name: String,
    #[serde(default)]
    rarity: Rarity,
    #[serde(default)]
    implicitMods: Vec<String>,
    #[serde(default)]
    explicitMods: Vec<String>,
    #[serde(default)]
    fracturedMods: Vec<String>,
    #[serde(default)]
    enchantMods: Vec<String>,
    #[serde(default)]
    craftedMods: Vec<String>,
    socketedItems: Option<Vec<Item>>,
    inventoryId: Option<String>,
    #[serde(default)]
    properties: Vec<Property>,
    x: Option<u16>,
}

#[derive(Deserialize)]
struct ItemsSkillsChar {
    items: Vec<Item>,
    character: Character,
}

#[derive(Deserialize)]
struct PassiveTree {
    hashes: Vec<u16>,
    hashes_ex: Vec<u16>,
    items: Vec<Item>,
    #[serde(default)]
    mastery_effects: FxHashMap<String, u32>,
}

impl Item {
    pub fn quality(&self) -> i64 {
        if let Some(quality_prop) = self.properties.iter().find(|p| p.name == "Quality") {
            if !quality_prop.values.is_empty() {
                let string = quality_prop.values[0].0.replace(['+', '%'], "");
                if let Ok(quality) = i64::from_str(&string) {
                    return quality;
                }
            }
        }
        0
    }
}

fn extract_socketed(gems: &Vec<Item>) -> (GemLink, Vec<item::Item>) {
    let mut gemlink = GemLink {
        gems: vec![],
        slot: build::Slot::Helm,
    };
    let mut jewels = vec![];

    for gem in gems {
        if let Some(gem_id) = GEMS.iter().find_map(|(key, val)| {
            if val.base_item.display_name == gem.baseType {
                return Some(key);
            }
            None
        }) {
            // Parsing stuff is just beautiful
            let level = u32::from_str(
                gem.properties.iter().find(|p| p.name == "Level").unwrap().values[0]
                    .0
                    .split(' ')
                    .collect::<Vec<&str>>()[0],
            ).unwrap_or(1);
            let mut qual = 0;
            if let Some(qual_entry) = gem.properties.iter().find(|p| p.name == "Quality") {
                qual = i32::from_str(&qual_entry.values[0].0.replace(['+', '%'], "")).unwrap_or(0);
            }
            let new_gem = gem::Gem {
                id: gem_id.to_string(),
                enabled: true,
                level,
                qual,
                alt_qual: 0,
            };
            gemlink.gems.push(new_gem);
        } else {
            jewels.push(conv_item(gem));
        }
    }

    (gemlink, jewels)
}

fn conv_item(item: &Item) -> item::Item {
    let mut mods_expl = item.explicitMods.clone();
    mods_expl.extend(item.craftedMods.clone());
    mods_expl.extend(item.fracturedMods.clone());
    item::Item {
        base_item: item.baseType.clone(),
        name: item.name.clone(),
        rarity: item.rarity,
        mods_impl: item.implicitMods.clone(),
        mods_expl,
        mods_enchant: item.enchantMods.clone(),
        quality: item.quality(),
    }
}

#[derive(Debug, Clone)]
struct ParseError;
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to parse")
    }
}
impl std::error::Error for ParseError {}

pub fn character(account: &str, character: &str) -> Result<Build, Box<dyn Error>> {
    let client = reqwest::blocking::ClientBuilder::new().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0").build()?;

    // Passive Tree
    let url = format!("https://pathofexile.com/character-window/get-passive-skills?realm=pc&accountName={account}&character={character}");
    println!("{url}");
    let tree = client.get(url).send()?.json::<PassiveTree>()?;

    // Items, Skills, CharData
    let url = format!("https://pathofexile.com/character-window/get-items?realm=pc&accountName={account}&character={character}");
    println!("{url}");
    let items = client.get(url).send()?.json::<ItemsSkillsChar>()?;

    let mut build = Build::new_player();
    build.name = character.to_string();
    build.set_property_int(crate::build::property::Int::Level, items.character.level);
    build.tree.nodes = tree.hashes;
    build.tree.nodes_ex = tree.hashes_ex;
    if let Ok(class) = Class::from_str(&items.character.class_or_ascendancy) {
        build.tree.set_class(class);
    } else if let Ok(ascendancy) = Ascendancy::from_str(&items.character.class_or_ascendancy) {
        build.tree.set_ascendancy(Some(ascendancy));
    } else {
        return Err(Box::new(ParseError));
    }

    for (mastery, selected) in &tree.mastery_effects {
        if let Ok(mastery) = u32::from_str(mastery) {
            build.tree.masteries.insert(mastery as u16, *selected as u16);
        } else {
            eprintln!("Couldn't parse mastery effect id: {}", mastery);
        }
    }

    for item in tree.items.iter().chain(items.items.iter()) {
        if let Some(socketed_items) = &item.socketedItems {
            let (gemlink, jewels) = extract_socketed(socketed_items);
            build.gem_links.push(gemlink);
            // TODO: abyss jewels
            build.inventory.extend(jewels);
        }
        if let Some(inventory_id) = &item.inventoryId {
            if let Ok(slot) = Slot::try_from((inventory_id.as_str(), item.x.unwrap_or(0))) {
                build.equipment.insert(slot, conv_item(item));
            } else {
                build.inventory.push(conv_item(item));
            }
        }
    }

    Ok(build)
}
