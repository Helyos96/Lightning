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
use std::cell::Cell;
use std::error::Error;
use std::io;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

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
    #[serde(default)]
    mutatedMods: Vec<String>,
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
struct JewelDataImport {
    subgraph: Option<SubgraphImport>,
}

#[derive(Deserialize)]
struct SubgraphImport {
    groups: FxHashMap<String, GroupImport>,
    nodes: FxHashMap<String, NodeImport>,
}

#[derive(Deserialize)]
struct NodeImport {
    group: String,
    orbit: u16,
    #[serde(rename = "orbitIndex")]
    orbit_index: u16,
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
struct SkillOverride {
    name: String,
}

#[derive(Deserialize)]
struct PassiveTreeImport {
    hashes: Vec<u32>,
    hashes_ex: Vec<u32>,
    items: Vec<Item>,
    #[serde(default)]
    mastery_effects: FxHashMap<String, u32>,
    alternate_ascendancy: Option<i32>,
    #[serde(default)]
    skill_overrides: FxHashMap<u32, SkillOverride>,
    #[serde(default)]
    jewel_data: FxHashMap<String, JewelDataImport>,
}

impl Item {
    pub fn prop(&self, name: &str) -> Option<i64> {
        let prop = self.properties.iter().find(|p| p.name == name)?;
        if prop.values.is_empty() {
            return None;
        }
        i64::from_str(&prop.values[0].0.replace(['+', '%'], "")).ok()
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
            let new_gem = gem::Gem::new(gem_id.to_string(), true, level, qual, 0);
            gemlink.gems.push(Arc::new(new_gem));
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
    mods_expl.extend(item.mutatedMods.clone());
    let mut item_ret = item::Item {
        base_item: item.baseType.clone(),
        name: item.name.clone(),
        rarity: item.rarity,
        mods_impl: item.implicitMods.clone(),
        mods_expl,
        mods_enchant: item.enchantMods.clone(),
        quality: item.prop("Quality").unwrap_or(0),
        corrupted: item.corrupted,
        item_level: item.ilvl.unwrap_or(0),
        base_percentile: 0,
        ..Default::default()
    };

    let (armour, evasion, energy_shield) = (item.prop("Armour"), item.prop("Evasion Rating"), item.prop("Energy Shield"));
    if armour.is_some() || evasion.is_some() || energy_shield.is_some() {
        item_ret.reverse_base_percentile(armour.unwrap_or(0), evasion.unwrap_or(0), energy_shield.unwrap_or(0));
    }

    Some(item_ret)
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
    println!("{url}");
    let tree_import = client.get(url).send()?.json::<PassiveTreeImport>()?;

    // Items, Skills, CharData
    let url = format!("https://pathofexile.com/character-window/get-items?realm=pc&accountName={account}&character={character}").replace('#', "%23");
    println!("{url}");
    let items_import = client.get(url).send()?.json::<ItemsSkillsChar>()?;

    let mut build = Build::new_player();
    let mut abyssal_jewel_idx = 0;
    build.name = character.to_string();
    build.set_property_int(crate::build::property::Int::Level, items_import.character.level);
    build.tree.nodes = tree_import.hashes;

    if let Ok(class) = Class::from_str(&items_import.character.class_or_ascendancy) {
        build.tree.set_class(class);
    } else if let Ok(ascendancy) = Ascendancy::from_str(&items_import.character.class_or_ascendancy) {
        build.tree.set_ascendancy(Some(ascendancy));
    } else {
        return Err(Box::new(ParseError));
    }

    for (mastery, selected) in &tree_import.mastery_effects {
        if let Ok(mastery) = u32::from_str(mastery) {
            build.tree.masteries.insert(mastery as u32, *selected as u32);
        } else {
            eprintln!("Couldn't parse mastery effect id: {mastery}");
        }
    }

    for (node_id, data) in tree_import.skill_overrides {
        build.tree.set_tattoo(node_id, Some(&data.name));
    }

    if let Some(alternate_ascendancy) = tree_import.alternate_ascendancy {
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

    let mut to_equip = vec![];
    for item in tree_import.items.iter().chain(items_import.items.iter()) {
        if let Some(socketed_items) = &item.socketedItems {
            let (gemlink, jewels) = extract_socketed(socketed_items);
            build.gem_links.push(gemlink);
            for jewel in jewels {
                build.inventory.push(Arc::new(jewel));
                build.equip(Slot::AbyssalJewel(abyssal_jewel_idx), build.inventory.len() - 1);
                abyssal_jewel_idx += 1;
            }
        }
        if let Some(inventory_id) = &item.inventoryId {
            if let Some(item_inv) = conv_item(item) {
                build.inventory.push(Arc::new(item_inv));
                if let Ok(slot) = Slot::try_from((inventory_id.as_str(), item.x.unwrap_or(0))) {
                    to_equip.push((slot, build.inventory.len() - 1));
                }
            }
        }
    }

    // Make sure the cluster jewels are equipped in order large->medium->small
    // to prevent bad connections / node generation
    to_equip.sort_unstable_by(|(_, inv_id_a),(_, inv_id_b)| {
        let item_a_base = &build.inventory[*inv_id_a].data().name;
        let item_b_base = &build.inventory[*inv_id_b].data().name;

        if item_a_base == item_b_base {
            return std::cmp::Ordering::Equal;
        }

        if item_a_base == "Large Cluster Jewel" {
            std::cmp::Ordering::Less
        } else if item_b_base == "Large Cluster Jewel" {
            std::cmp::Ordering::Greater
        } else if item_a_base == "Medium Cluster Jewel" {
            std::cmp::Ordering::Less
        } else if item_b_base == "Medium Cluster Jewel" {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    for (slot, inv_id) in to_equip {
        build.equip(slot, inv_id);
    }

    // Map hashes_ex to our tree for allocated cluster nodes
    let mut ex_node_lookup = FxHashMap::default();
    for jewel_data in tree_import.jewel_data.values() {
        if let Some(subgraph) = &jewel_data.subgraph {
            for (node_id_str, node) in &subgraph.nodes {
                if let Ok(node_id) = u32::from_str(node_id_str) &&
                   let Some(group) = subgraph.groups.get(&node.group) &&
                   let Ok(proxy_id) = u32::from_str(&group.proxy)
                {
                    ex_node_lookup.insert(node_id, (proxy_id, node.orbit, node.orbit_index));
                }
            }
        }
    }
    for hash_ex in &tree_import.hashes_ex {
        if let Some((proxy_id, orbit, orbit_index)) = ex_node_lookup.get(hash_ex) {
            if let Some(proxy_node) = TREE.nodes.get(proxy_id) {
                if let Some(group_id) = proxy_node.group {
                    for (_, generated_node) in &build.tree.nodes_cluster {
                        if generated_node.group == Some(group_id) &&
                           generated_node.orbit == Some(*orbit) &&
                           generated_node.orbit_index == Some(*orbit_index)
                        {
                            build.tree.nodes.push(generated_node.skill);
                            break;
                        }
                    }
                }
            }
        }
    }

    build.import_account = Some((account.to_string(), character.to_string()));
    build.campaign_choice = if items_import.character.level >= 67 {
        build::CampaignChoice::ActTen
    } else if items_import.character.level >= 45 {
        build::CampaignChoice::ActFive
    } else {
        build::CampaignChoice::Beach
    };

    Ok(build)
}
