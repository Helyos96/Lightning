use crate::gem::GemData;
use crate::item::BaseItem;
use crate::tree::TreeData;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::fs;
use std::io;

lazy_static! {
    pub static ref GEMS: FxHashMap<String, GemData> =
        bincode::deserialize(include_bytes!("../data/gems.bc")).expect("Failed to deserialize GEMS");
    pub static ref ITEMS: FxHashMap<String, BaseItem> =
        bincode::deserialize(include_bytes!("../data/base_items.bc")).expect("Failed to deserialize base items");
    pub static ref TREE: TreeData =
        bincode::deserialize(include_bytes!("../data/tree.bc")).expect("Failed to deserialize tree");
}
