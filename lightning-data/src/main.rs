#![allow(dead_code)]

use std::{fs::{self, File}, io::{BufReader, BufWriter}, process::Command, str::FromStr};
use argh::FromArgs;
use csd::parse_csd;
use dat_schema::{DatSchema, Table};
use datc64::{dump, ForeignRow, Val};
use lightning_model::data::poe2::tree;
use psg::parse_psg;
use rustc_hash::{FxHashMap, FxHashSet};

mod dat_schema;
mod psg;
mod csd;
mod datc64;
mod utils;

/// Creates a single spritesheet from a bunch of DDS file paths
/// Requires `bun_extract_file` (https://github.com/zao/ooz/releases) and `magick` (https://imagemagick.org/script/download.php) in PATH
/// @name: {name}.png
/// @dds_files: list of DDS file paths within Content.ggpk
/// @single_wh: dimension for a single sprite, both width and height (square)
fn generate_spritesheet(name: &str, dds_files: &FxHashSet<String>, max_items_per_line: usize, length: usize, extract_dds: bool, poe_dir: &str) -> tree::Sprite {
    if extract_dds {
        println!("Extracting {} DDS files with bun_extract_file...", dds_files.len());
        Command::new("bun_extract_file")
            .args(["extract-files", format!("{poe_dir}/Content.ggpk").as_str(), format!("{poe_dir}/out").as_str()])
            .args(dds_files)
            .output()
            .expect("failed to execute bun_extract_file");
    }

    let h = (dds_files.len() + 16) / max_items_per_line;
    let mut dds_filelist = vec![];
    for dds_path in dds_files {
        dds_filelist.push(format!("{poe_dir}/out/{}", dds_path.to_lowercase()));
    }

    println!("Making {name}.png with magick...");
    Command::new("magick")
        .arg("montage")
        .args(&dds_filelist)
        .args(["-background", "None"])
        .args(["-resize", "64x64"])
        .args(["-geometry", "+0+0"])
        .arg("-tile")
        .arg(format!("{}x{}", max_items_per_line, h).as_str())
        .arg(format!("{name}.png"))
        .output()
        .expect("failed to execute magick");

    let mut coords = FxHashMap::default();
    for (i, dds_path) in dds_files.iter().enumerate() {
        coords.insert(dds_path.to_string(), tree::Rect { h: length as u16, w: length as u16, x: ((i % max_items_per_line) * length) as u16, y: ((i / max_items_per_line) * length) as u16 });
    }

    tree::Sprite {
        filename: format!("{name}.png"),
        w: (max_items_per_line * length) as u16,
        h: (((dds_files.len() + max_items_per_line) / max_items_per_line) * length) as u16,
        coords,
    }
}

fn get_foreign_val(dats: &FxHashMap<String, Vec<FxHashMap<String, Val>>>, foreign_row: &ForeignRow, key: Option<&str>) -> Option<Val> {
    if key.is_none() && foreign_row.key.is_none() {
        return None;
    }
    if let Some(dat) = dats.get(&foreign_row.dat_file) {
        if let Some(row) = dat.get(foreign_row.rowid as usize) {
            let key = match key {
                Some(key) => key,
                None => foreign_row.key.as_ref().unwrap()
            };
            if let Some(col) = row.get(key) {
                return Some(col.clone());
            }
        }
    }
    None
}

#[derive(FromArgs)]
/// PoE2 game data extractor & processor
struct Args {
    /// path of exile 2 root dir
    #[argh(option, short = 'p')]
    poe_dir: String,
    /// dat schema JSON file path
    #[argh(option, short = 's')]
    schema: String,
    /// extract all datc64/psg/csd files
    #[argh(switch, short = 'e')]
    extract_dat: bool,
    /// extract required DDS files
    #[argh(switch, short = 'd')]
    extract_dds: bool,
}

fn main() {
    let args: Args = argh::from_env();
    let poe_dir = &args.poe_dir;

    let schema_file = fs::File::open(args.schema).expect("Failed to open dat schema");
    let dat_schema: DatSchema = serde_json::from_reader(BufReader::new(schema_file)).expect("Failed to deserialize dat schema");

    if args.extract_dat {
        println!("Extracting all datc64/psg/csd files..");
        Command::new("bun_extract_file")
            .args(["extract-files", "--regex", format!("{poe_dir}/Content.ggpk").as_str(), "C:/PoE2/out", "data/.*", "metadata/.*psg", "metadata/.*csd"])
            .output()
            .expect("failed to execute bun_extract_file");
    }

    let tables: Vec<Table> = dat_schema.tables.iter().filter(|t| [2,3].contains(&t.valid_for)).cloned().collect();
    let mut datc64_dumps = FxHashMap::default();
    let mut success = 0;
    for table in &tables {
        match dump(poe_dir, &dat_schema, &table.name, false) {
            Err(err) => eprintln!("{}: {err}", table.name),
            Ok(table_dump) => {
                datc64_dumps.insert(table.name.clone(), table_dump);
                success += 1;
            }
        }
    }
    println!("Success datc64 parses: {success}/{}", tables.len());

    if let Ok(passive_skills) = dump(poe_dir, &dat_schema, "PassiveSkills", false) {
        if let Ok(graph) = parse_psg(&format!("{poe_dir}/out/metadata/passiveskillgraph.psg")) {
            let mut translations = parse_csd(format!("{poe_dir}/out/metadata/statdescriptions/passive_skill_stat_descriptions.csd").as_str()).unwrap();
            translations.0.extend(parse_csd(format!("{poe_dir}/out/metadata/statdescriptions/stat_descriptions.csd").as_str()).unwrap().0);
            let mut nodes = FxHashMap::default();
            let mut groups = FxHashMap::default();
            for (i, group) in graph.groups.iter().enumerate() {
                groups.insert(i as u16 + 1, tree::Group {
                    is_proxy: false,
                    x: group.x,
                    y: group.y,
                    orbits: vec![],
                    background: None,
                    nodes: group.nodes.iter().map(|ng| ng.passive_skill as u16).collect(),
                });
            }
            for node_graph in graph.groups.iter().flat_map(|g| &g.nodes) {
                if let Some(node_dat) = passive_skills.iter().find(|nd| nd["PassiveSkillGraphId"].integer() as u16 == node_graph.passive_skill as u16) {
                    if node_dat.contains_key("Icon_DDSFile") && node_dat.contains_key("Name") && !node_dat["Name"].string().starts_with("[DNT]") {
                        let mut stats = vec![];
                        if let Some(Val::Array(stats_dat)) = node_dat.get("Stats") {
                            let mut stat_val_idx = 1;
                            for stat in stats_dat {
                                if let Some(stat_id) = get_foreign_val(&datc64_dumps, stat.foreign_row(), Some("Id")) {
                                    if let Some(nb_args) = translations.nb_args(stat_id.string()) {
                                        // TODO: doing something wrong when nb_arbs >= 2
                                        let mut stat_vals = vec![];
                                        for _ in 0..nb_args {
                                            stat_vals.push(node_dat.get(&format!("Stat{}Value", stat_val_idx)).unwrap().integer());
                                            stat_val_idx += 1;
                                        }
                                        if let Some(stat_text) = translations.format(stat_id.string(), &stat_vals) {
                                            stats.push(stat_text);
                                        } else {
                                            println!("failed desc '{}' args {:?}", stat_id.string(), stat_vals);
                                        }
                                    } else {
                                        println!("failed desc '{}'", stat_id.string());
                                    }
                                }
                            }
                        }
                        let mut ascendancy = None;
                        if let Some(val) = node_dat.get("AscendancyKey") {
                            if let Some(ascendancy_name) = get_foreign_val(&datc64_dumps, val.foreign_row(), Some("Name")) {
                                if let Ok(ascendancy_parsed) = tree::Ascendancy::from_str(ascendancy_name.string()) {
                                    ascendancy = Some(ascendancy_parsed);
                                }
                            }
                        }
                        let node = tree::Node {
                            skill: node_graph.passive_skill as u16,
                            stats,
                            icon: node_dat["Icon_DDSFile"].string().to_string(),
                            name: node_dat["Name"].string().to_string(),
                            is_notable: node_dat["IsNotable"].bool(),
                            is_keystone: node_dat["IsKeystone"].bool(),
                            is_ascendancy_start: node_dat["IsAscendancyStartingNode"].bool(),
                            is_jewel_socket: node_dat["IsJewelSocket"].bool(),
                            is_just_icon: node_dat["IsJustIcon"].bool(),
                            ascendancy,
                            class_start_index: None,
                            group: Some(node_graph.group as u16),
                            orbit: Some(node_graph.radius as u16),
                            orbit_index: Some(node_graph.position as u16),
                            out: Some(node_graph.connections.iter().map(|ng| (*ng).0 as u16).collect()),
                            r#in: None,
                        };
                        nodes.insert(node_graph.passive_skill as u16, node);
                    }
                }
            }
            for (i, root_node) in graph.root_nodes.iter().enumerate() {
                nodes.get_mut(&(root_node.0 as u16)).unwrap().class_start_index = Some(i as i32);
            }
            
            let dds_files: FxHashSet<String> = nodes.iter().map(|n| n.1.icon.to_string()).collect();
            let mut sprites = FxHashMap::default();
            let skills_ss = generate_spritesheet("skills-3", &dds_files, 16, 64, args.extract_dds, &args.poe_dir);
            sprites.insert("normalActive".to_string(), skills_ss);
            let jewel_slots = nodes.iter().filter(|n| n.1.is_jewel_socket).map(|n| *n.0).collect();

            // Fill node.in
            let mut nodes_final = nodes.clone();
            for node in &mut nodes_final {
                let in_nodes: Vec<u16> = nodes.iter().filter(|(_, n)| n.out.is_some() && n.out.as_ref().unwrap().contains(node.0)).map(|(id, _)| *id).collect();
                if in_nodes.len() > 0 {
                    node.1.r#in = Some(in_nodes);
                }
            }

            println!("nodes len: {}", nodes_final.len());
            let tree = tree::TreeData {
                classes: FxHashMap::default(),
                constants: tree::Constants {
                    orbit_radii: vec![0,82,162,335,493,662,846,249,1020,1200],
                    skills_per_orbit: vec![1,12,24,24,72,72,72,24,72,144],
                },
                groups,
                jewel_slots,
                min_x: -30000,
                max_x: 30000,
                min_y: -30000,
                max_y: 30000,
                nodes: nodes_final,
                sprites,
            };
            if let Ok(file) = File::create("tree.json") {
                let mut writer = BufWriter::new(file);
                serde_json::to_writer(&mut writer, &tree).unwrap();
            }
        }
    }
}
