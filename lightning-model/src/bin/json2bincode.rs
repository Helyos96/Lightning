use lightning_model::gem::GemData;
use lightning_model::item::BaseItem;
use lightning_model::tree::TreeData;
use rustc_hash::FxHashMap;
use std::fs;
use std::io;

fn main() {
    let gems: FxHashMap<String, GemData> = {
        serde_json::from_slice(include_bytes!("../../data/gems.json")).expect("Failed to deserialize gems")
    };
    let items: FxHashMap<String, BaseItem> = {
        serde_json::from_slice(include_bytes!("../../data/base_items.json")).expect("Failed to deserialize base items")
    };
    let tree: TreeData = {
        serde_json::from_slice(include_bytes!("../../data/tree.json")).expect("Failed to deserialize tree")
    };

    let mut f = io::BufWriter::new(fs::File::create("data/gems.bc").unwrap());
    bincode::serialize_into(&mut f, &gems).expect("Failed to ser gems");
    let mut f = io::BufWriter::new(fs::File::create("data/base_items.bc").unwrap());
    bincode::serialize_into(&mut f, &items).expect("Failed to ser base_items");
    let mut f = io::BufWriter::new(fs::File::create("data/tree.bc").unwrap());
    bincode::serialize_into(&mut f, &tree).expect("Failed to ser tree");
}
