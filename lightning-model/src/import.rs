#![allow(non_snake_case)]

/// Import build data from pathofexile.com

use crate::build::{self, Build, GemLink};
use crate::data::GEMS;
use crate::gem;
use crate::item;
use crate::tree::Class;
use serde::Deserialize;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::io;
use std::str::FromStr;

#[derive(Deserialize)]
struct Character {
    level: i32,
    #[serde(rename = "class")]
    ascendancy: String,
}

#[derive(Debug, Deserialize)]
struct Property {
    name: String,
    values: Vec<(String, i32)>,
}

#[derive(Debug, Deserialize)]
struct Item {
    baseType: String,
    implicitMods: Option<Vec<String>>,
    explicitMods: Option<Vec<String>>,
    craftedMods: Option<Vec<String>>,
    socketedItems: Option<Vec<Item>>,
    inventoryId: Option<String>,
    #[serde(default)]
    properties: Vec<Property>,
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

fn extract_socketed(gems: &Vec<Item>) -> (GemLink, Vec<item::Item>) {
    let mut gemlink = GemLink {
        active_gems: vec![],
        support_gems: vec![],
        slot: build::Slot::Helm,
    };
    let mut jewels = vec![];

    for gem in gems {
        if let Some(gem_id) = GEMS.iter().find_map(|(key, val)| {
            if let Some(base_item) = &val.base_item {
                if base_item.display_name == gem.baseType {
                    return Some(key);
                }
            }
            None
        }) {
            let gem_data = &GEMS[gem_id];
            // Parsing stuff is just beautiful
            let level = usize::from_str(
                gem.properties.iter().find(|p| p.name == "Level").unwrap().values[0]
                    .0
                    .split(' ')
                    .collect::<Vec<&str>>()[0],
            )
            .unwrap();
            let new_gem = gem::Gem {
                id: gem_id.to_string(),
                level,
                qual: 0,
                alt_qual: 0,
            };
            match gem_data.is_support {
                true => gemlink.support_gems.push(new_gem),
                false => gemlink.active_gems.push(new_gem),
            }
        } else {
            jewels.push(conv_item(gem));
        }
    }

    (gemlink, jewels)
}

fn conv_item(item: &Item) -> item::Item {
    let mut mods_expl = item.explicitMods.as_ref().unwrap_or(&vec![]).clone();
    mods_expl.extend(item.craftedMods.as_ref().unwrap_or(&vec![]).clone());
    item::Item {
        base_item: item.baseType.clone(),
        mods_impl: item.implicitMods.as_ref().unwrap_or(&vec![]).clone(),
        mods_expl,
    }
}

pub fn character(account: &str, character: &str) -> Result<Build, Box<dyn Error>> {
    // Passive Tree
    let url = "https://pathofexile.com/character-window/get-passive-skills?realm=pc&accountName=".to_string()
        + account
        + "&character="
        + character;
    println!("{url}");
    let client = reqwest::blocking::ClientBuilder::new().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0").build()?;
    let tree = client.get(url).send()?.json::<PassiveTree>()?;

    // Items, Skills, CharData
    let url = "https://pathofexile.com/character-window/get-items?realm=pc&accountName=".to_string()
        + account
        + "&character="
        + character;
    println!("{url}");
    let items = client.get(url).send()?.json::<ItemsSkillsChar>()?;

    let mut build = Build::new_player();
    build.name = character.to_string();
    build.level = items.character.level;
    build.tree.nodes = tree.hashes;
    build.tree.nodes_ex = tree.hashes_ex;
    build.tree.set_class(Class::from_ascendancy_str(items.character.ascendancy.as_str()));

    for (mastery, selected) in &tree.mastery_effects {
        let mastery = u32::from_str(mastery)?;
        build.tree.masteries.push((mastery as u16, *selected as u16));
    }

    for item in tree.items {
        build.equipment.push(conv_item(&item));
    }

    for item in items.items {
        if let Some(socketed_items) = &item.socketedItems {
            let (gemlink, jewels) = extract_socketed(socketed_items);
            build.gem_links.push(gemlink);
            build.equipment.extend(jewels);
        }
        build.equipment.push(conv_item(&item));
    }

    Ok(build)
}
