#![allow(non_snake_case)]

//! Import build data from pathofexile.com

use crate::build::{self, Build, GemLink, Slot};
use crate::data::base_item::{self, Rarity};
use crate::data::tree::{Ascendancy, Class, ExpansionJewel};
use crate::data::{GEMS, ITEMS, TREE};
use crate::gem;
use crate::item;
use serde::Deserialize;
use rustc_hash::FxHashMap;
use serde_with::{serde_as, DisplayFromStr};
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
    corrupted: bool,
    #[serde(default)]
    properties: Vec<Property>,
    ilvl: Option<i64>,
    x: Option<u16>,
}

#[derive(Deserialize)]
struct ItemsSkillsChar {
    items: Vec<Item>,
    character: Character,
}

#[derive(Deserialize)]
struct GroupImport {
    proxy: String,
    nodes: Vec<String>,
    x: f32,
    y: f32,
    orbits: Vec<u16>
}

#[derive(Deserialize)]
struct PassiveTree {
    hashes: Vec<u32>,
    hashes_ex: Vec<u32>,
    items: Vec<Item>,
    #[serde(default)]
    mastery_effects: FxHashMap<String, u32>,
    alternate_ascendancy: Option<i32>,
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
        if let Some(gem_id) =
            GEMS.iter().find_map(|(key, val)| {
                if val.display_name() == gem.baseType {
                    return Some(key);
                }
                None
            })
        {
            // Parsing stuff is just beautiful
            let level = u32::from_str(
                gem.properties.iter().find(|p| p.name == "Level").unwrap().values[0].0.split(' ').collect::<Vec<&str>>()[0],
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
        } else if let Some(jewel) = conv_item(gem) {
            jewels.push(jewel);
        } else {
            eprintln!("Failed to import item {}", gem.baseType);
        }
    }

    (gemlink, jewels)
}

fn conv_item(item: &Item) -> Option<item::Item> {
    if !ITEMS.contains_key(&item.baseType) {
        return None;
    }
    let mut mods_expl = item.explicitMods.clone();
    mods_expl.extend(item.craftedMods.clone());
    mods_expl.extend(item.fracturedMods.clone());
    Some(item::Item {
        base_item: item.baseType.clone(),
        name: item.name.clone(),
        rarity: item.rarity,
        mods_impl: item.implicitMods.clone(),
        mods_expl,
        mods_enchant: item.enchantMods.clone(),
        quality: item.quality(),
        corrupted: item.corrupted,
        item_level: item.ilvl.unwrap_or(0),
    })
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
    let url = format!("https://pathofexile.com/character-window/get-passive-skills?realm=pc&accountName={account}&character={character}").replace('#', "%23");
    let tree = client.get(url).send()?.json::<PassiveTree>()?;

    // Items, Skills, CharData
    let url = format!("https://pathofexile.com/character-window/get-items?realm=pc&accountName={account}&character={character}").replace('#', "%23");
    let items = client.get(url).send()?.json::<ItemsSkillsChar>()?;

    let mut build = Build::new_player();
    let mut abyssal_jewel_idx = 0;
    build.name = character.to_string();
    build.set_property_int(crate::build::property::Int::Level, items.character.level);
    build.tree.nodes = tree.hashes;
    if let Ok(class) = Class::from_str(&items.character.class_or_ascendancy) {
        build.tree.set_class(class);
    } else if let Ok(ascendancy) = Ascendancy::from_str(&items.character.class_or_ascendancy) {
        build.tree.set_ascendancy(Some(ascendancy));
    } else {
        return Err(Box::new(ParseError));
    }

    if let Some(alternate_ascendancy) = tree.alternate_ascendancy {
        if let Some(aa) = TREE.alternate_ascendancies.get((alternate_ascendancy - 1) as usize) {
            let bloodline_str = &aa.id;
            if let Ok(bloodline) = Ascendancy::from_str(bloodline_str) {
                build.tree.set_bloodline(Some(bloodline));
            } else {
                eprintln!("Failed to match alternate ascendancy {}", bloodline_str);
            }
        } else {
            eprintln!("Failed to find alternate ascendancy index {}", alternate_ascendancy - 1);
        }
    }

    for (mastery, selected) in &tree.mastery_effects {
        if let Ok(mastery) = u32::from_str(mastery) {
            build.tree.masteries.insert(mastery as u32, *selected as u32);
        } else {
            eprintln!("Couldn't parse mastery effect id: {mastery}");
        }
    }

    for item in tree.items.iter().chain(items.items.iter()) {
        if let Some(socketed_items) = &item.socketedItems {
            let (gemlink, jewels) = extract_socketed(socketed_items);
            build.gem_links.push(gemlink);
            for jewel in jewels {
                build.inventory.push(jewel);
                build.equipment.insert(Slot::AbyssalJewel(abyssal_jewel_idx), build.inventory.len() - 1);
                abyssal_jewel_idx += 1;
            }
        }
        if let Some(inventory_id) = &item.inventoryId {
            if let Some(item_inv) = conv_item(item) {
                build.inventory.push(item_inv);
                if let Ok(slot) = Slot::try_from((inventory_id.as_str(), item.x.unwrap_or(0))) {
                    build.equipment.insert(slot, build.inventory.len() - 1);
                }
            }
        }
    }

    build.import_account = Some((account.to_string(), character.to_string()));
    Ok(build)
}
