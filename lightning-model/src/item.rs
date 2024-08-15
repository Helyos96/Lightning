use crate::data::ITEMS;
use crate::modifier::{parse_mod, Mod, Source};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMinMax {
    min: u32,
    max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Properties {
    armour: Option<PropertyMinMax>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseItem {
    name: String,
    implicits: Vec<String>,
    item_class: String,
    properties: Properties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub base_item: String,
    pub mods_impl: Vec<String>,
    pub mods_expl: Vec<String>,
}

impl Item {
    pub fn data(&self) -> &'static BaseItem {
        &ITEMS[&self.base_item]
    }

    pub fn calc_mods(&self) -> Vec<Mod> {
        let mut mods = vec![];

        for m in &self.mods_impl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers);
            }
        }
        for m in &self.mods_expl {
            if let Some(modifiers) = parse_mod(m, Source::Item) {
                mods.extend(modifiers);
            }
        }

        mods
    }
}
