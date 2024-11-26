use lightning_model::data::base_item::BaseItem;
use lightning_model::data::default_monster_stats::MonsterStats;
use lightning_model::data::gem::GemData;
use lightning_model::data::tree::TreeData;
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
    let monster_stats: FxHashMap<i64, MonsterStats> = {
        serde_json::from_slice(include_bytes!("../../data/default_monster_stats.json")).expect("Failed to deserialize default monster stats")
    };

    let mut f = io::BufWriter::new(fs::File::create("data/gems.bc").unwrap());
    bincode::serialize_into(&mut f, &gems).expect("Failed to ser gems");
    let mut f = io::BufWriter::new(fs::File::create("data/base_items.bc").unwrap());
    bincode::serialize_into(&mut f, &items).expect("Failed to ser base_items");
    let mut f = io::BufWriter::new(fs::File::create("data/tree.bc").unwrap());
    bincode::serialize_into(&mut f, &tree).expect("Failed to ser tree");
    let mut f = io::BufWriter::new(fs::File::create("data/default_monster_stats.bc").unwrap());
    bincode::serialize_into(&mut f, &monster_stats).expect("Failed to ser default monster stats");
}
