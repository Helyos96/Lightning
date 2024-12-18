#![allow(dead_code)]

use std::{fs::File, io::{self, BufWriter, Cursor, Read, Seek}, process::Command, str::FromStr};
use byteorder::{LittleEndian};
use csd::parse_csd;
use dat_schema::{Column, DatSchema, Table, Type};
use byteorder::ReadBytesExt;
use lightning_model::data::poe2::tree;
use psg::parse_psg;
use rustc_hash::{FxHashMap, FxHashSet};

mod dat_schema;
mod psg;
mod csd;

#[derive(Clone, Debug)]
struct ForeignRow {
    dat_file: String,
    rowid: u64,
    key: Option<String>,
}

#[derive(Clone, Debug)]
enum Val {
    Bool(bool),
    Integer(i64),
    Float(f32),
    String(String),
    Row(u64),
    ForeignRow(ForeignRow),
    Array(Vec<Val>),
}

impl Val {
    fn string(&self) -> &str {
        if let Val::String(string) = self {
            string
        } else {
            panic!("not a string");
        }
    }

    fn integer(&self) -> i64 {
        if let Val::Integer(i) = *self {
            i
        } else {
            panic!("not a bool");
        }
    }

    fn bool(&self) -> bool {
        if let Val::Bool(b) = *self {
            b
        } else {
            panic!("not a bool");
        }
    }

    fn row(&self) -> u64 {
        if let Val::Row(r) = *self {
            r
        } else {
            panic!("not a row");
        }
    }

    fn foreign_row(&self) -> &ForeignRow {
        if let Val::ForeignRow(fr) = self {
            fr
        } else {
            panic!("not a foreign row");
        }
    }

    fn skill_id(&self) -> u16 {
        if let Val::Integer(i) = *self {
            (i as i16) as u16
        } else {
            panic!("not an integer");
        }
    }
}

fn read_file(name: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(name)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

const PATTERN_VAR_DATA: [u8; 8] = [0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB, 0xBB];
const PATTERN_VAR_END: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

fn make_foreign_row(column: &Column, rid: u64) -> Option<Val> {
    if rid == 0xfefefefefefefefe || rid == 0x100000000000000 {
        return None;
    }
    if let Some(references) = &column.references {
        return Some(Val::ForeignRow(ForeignRow { dat_file: references.table.clone(), rowid: rid, key: references.column.clone() }));
    }
    None
}

fn get_val(column: &Column, cursor: &mut Cursor<&Vec<u8>>, strict: bool) -> io::Result<Option<Val>> {
    let val = match column.r#type {
        Type::bool => {
            let b = cursor.read_u8()?;
            if strict && b > 1 {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Bad bool: {b}")));
            }
            Some(Val::Bool(b != 0))
        },
        Type::foreignrow => {
            let rid = cursor.read_u64::<LittleEndian>()?;
            let _unk = cursor.read_u64::<LittleEndian>()?;
            make_foreign_row(column, rid)
        },
        Type::array => {
            None
        }
        Type::string => {
            let mut buf: Vec<u16> = vec![];
            loop {
                let word = cursor.read_u16::<LittleEndian>()?;
                if word == 0 {
                    break;
                }
                buf.push(word);
            }
            if let Ok(utf16_string) = String::from_utf16(&buf) {
                Some(Val::String(utf16_string))
            } else {
                None
            }
        },
        Type::enumrow => {
            cursor.read_u32::<LittleEndian>()?;
            None
        },
        Type::row => {
            let rid = cursor.read_u64::<LittleEndian>()?;
            Some(Val::Row(rid))
        },
        Type::i32 => {
            let i = cursor.read_i32::<LittleEndian>()?;
            Some(Val::Integer(i as i64))
        },
        Type::i16 => {
            let i = cursor.read_i16::<LittleEndian>()?;
            Some(Val::Integer(i as i64))
        },
        Type::f32 => {
            let f = cursor.read_f32::<LittleEndian>()?;
            Some(Val::Float(f))
        },
    };
    Ok(val)
}

fn dump_datc64(dat_schema: &DatSchema, name: &str, strict: bool) -> io::Result<Vec<FxHashMap<String, Val>>> {
    if let Some(table) = dat_schema.tables.iter().find(|t| t.name == name) {
        let buf = read_file(&format!(r"C:\PoE2\out\data\{}.datc64", name.to_lowercase()))?;
        if let Some(var_offset) = find_pattern(&buf, &PATTERN_VAR_DATA) {
            let mut ret = vec![];
            let mut cursor = Cursor::new(&buf);
            let nb_rows = cursor.read_u32::<LittleEndian>()?;
            let row_len = (var_offset - 4) / nb_rows as usize;
            for row in 0..nb_rows {
                cursor.set_position(4 + (row as u64 * row_len as u64));
                let mut col_data = FxHashMap::default();
                for column in &table.columns {
                    if column.array {
                        let length = cursor.read_i64::<LittleEndian>()?;
                        let offset = cursor.read_u64::<LittleEndian>()?;
                        let mut cursor = cursor.clone();
                        if cursor.seek(io::SeekFrom::Start(var_offset as u64 + offset)).is_ok() {
                            if length <= 0 || length > 100000 {
                                continue;
                            }
                            let mut array = vec![];
                            for _ in 0..length {
                                let val = get_val(column, &mut cursor, strict);
                                if let Ok(Some(val)) = val {
                                    array.push(val);
                                } else {
                                    break;
                                }
                            }
                            if let Some(name) = &column.name {
                                col_data.insert(name.clone(), Val::Array(array));
                            }
                        }
                    } else {
                        if column.r#type.is_var_data() {
                            let mut new_cursor = cursor.clone();
                            let offset = column.r#type.var_offset(&mut cursor)?;
                            if new_cursor.seek(io::SeekFrom::Start(var_offset as u64 + offset)).is_ok() {
                                if let Some(name) = &column.name {
                                    let val = get_val(column, &mut new_cursor, false);
                                    if let Ok(Some(val)) = val {
                                        col_data.insert(name.clone(), val);
                                    }
                                }
                            }
                        } else {
                            let val = get_val(column, &mut cursor, false)?;
                            if let Some(name) = &column.name {
                                if val.is_some() {
                                    col_data.insert(name.clone(), val.unwrap());
                                }
                            }
                        }
                    }
                }
                if row == 0 && cursor.position() != (4 + ((row + 1) as u64 * row_len as u64)) {
                    println!("Warning: {}.datc64: Bad cursor: {}, expected {}", name.to_lowercase(), cursor.position(), (4 + ((row + 1) as u64 * row_len as u64)));
                }
                ret.push(col_data);
            }
            return Ok(ret);
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "couldn't find var pattern"));
        }
    } else {
        return Err(io::Error::new(io::ErrorKind::Other, format!("No dat schema for {name}")));
    }
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if data.len() < pattern.len() {
        return None;
    }
    for i in 0..data.len() - pattern.len() {
        if &data[i..i+pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
}

/// Creates a single spritesheet from a bunch of DDS file paths
/// Requires `bun_extract_file` (https://github.com/zao/ooz) and `magick` (https://imagemagick.org/script/download.php) in PATH
/// @name: {name}.png
/// @dds_files: list of DDS file paths within Content.ggpk
/// @single_wh: dimension for a single sprite, both width and height (square)
fn generate_spritesheet(name: &str, dds_files: &FxHashSet<String>) -> tree::Sprite {
    const MAX_ITEMS_PER_LINE: usize = 16;
    const LENGTH: usize = 64;

    println!("Extracting {} DDS files with bun_extract_file...", dds_files.len());
    Command::new("bun_extract_file")
        .args(["extract-files", "C:/PoE2/Content.ggpk", "C:/PoE2/out"])
        .args(dds_files)
        .output()
        .expect("failed to execute bun_extract_file");

    let h = (dds_files.len() + 16) / MAX_ITEMS_PER_LINE;
    let mut dds_filelist = vec![];

    for dds_path in dds_files {
        dds_filelist.push(format!("C:/PoE2/out/{}", dds_path.to_lowercase()));
    }

    println!("Making {name}.png with magick...");
    Command::new("magick")
        .arg("montage")
        .args(&dds_filelist)
        .args(["-background", "None"])
        .args(["-resize", "64x64"])
        .args(["-geometry", "+0+0"])
        .arg("-tile")
        .arg(format!("{}x{}", MAX_ITEMS_PER_LINE, h).as_str())
        .arg(format!("{name}.png"))
        .output()
        .expect("failed to execute magick");

    let mut coords = FxHashMap::default();
    for (i, dds_path) in dds_files.iter().enumerate() {
        coords.insert(dds_path.to_string(), tree::Rect { h: LENGTH as u16, w: LENGTH as u16, x: ((i % MAX_ITEMS_PER_LINE) * LENGTH) as u16, y: ((i / MAX_ITEMS_PER_LINE) * LENGTH) as u16 });
    }

    tree::Sprite {
        filename: format!("{name}.png"),
        w: (MAX_ITEMS_PER_LINE * LENGTH) as u16,
        h: (((dds_files.len() + MAX_ITEMS_PER_LINE) / MAX_ITEMS_PER_LINE) * LENGTH) as u16,
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

fn main() {
    let dat_schema: DatSchema = {
        serde_json::from_slice(include_bytes!("schema.min.json")).expect("Failed to deserialize dat_schema")
    };
    let tables: Vec<Table> = dat_schema.tables.iter().filter(|t| [2,3].contains(&t.valid_for)).cloned().collect();
    let mut datc64_dumps = FxHashMap::default();
    let mut success = 0;
    for table in &tables {
        match dump_datc64(&dat_schema, &table.name, false) {
            Err(err) => eprintln!("{}: {err}", table.name),
            Ok(table_dump) => {
                datc64_dumps.insert(table.name.clone(), table_dump);
                success += 1;
            }
        }
    }
    println!("Success dac64 parses: {success}/{}", tables.len());

    if let Ok(ret) = dump_datc64(&dat_schema,"PassiveSkills", false) {
        if let Ok(graph) = parse_psg() {
            let mut translations = parse_csd("C:/PoE2/out/metadata/statdescriptions/passive_skill_stat_descriptions.csd").unwrap();
            translations.0.extend(parse_csd("C:/PoE2/out/metadata/statdescriptions/stat_descriptions.csd").unwrap().0);
            //dbg!(&translations);
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
                if let Some(node_dat) = ret.iter().find(|nd| nd["PassiveSkillGraphId"].skill_id() == node_graph.passive_skill as u16) {
                    if node_dat.contains_key("Icon_DDSFile") && node_dat.contains_key("Name") && !node_dat["Name"].string().starts_with("[DNT]") {
                        let mut stats = vec![];
                        if let Some(Val::Array(stats_dat)) = node_dat.get("Stats") {
                            let mut stat_val_idx = 1;
                            for stat in stats_dat {
                                if let Some(stat_id) = get_foreign_val(&datc64_dumps, stat.foreign_row(), Some("Id")) {
                                    // TODO: Stats with more than 1 arg
                                    let stat_val = node_dat.get(&format!("Stat{}Value", stat_val_idx)).unwrap().integer();
                                    stat_val_idx += 1;
                                    if let Some(stat_text) = translations.format(stat_id.string(), &[stat_val]) {
                                        stats.push(stat_text);
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
            
            let dds_files: FxHashSet<String> = nodes.iter().map(|n| n.1.icon.to_string()).collect();
            let mut sprites = FxHashMap::default();
            let skills_ss = generate_spritesheet("skills-3", &dds_files);
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
