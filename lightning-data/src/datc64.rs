use std::{io::{self, Cursor, Seek}};
use byteorder::{LittleEndian, ReadBytesExt};
use rustc_hash::FxHashMap;
use crate::{dat_schema::{Column, DatSchema, Type}, utils::read_file};

#[derive(Clone, Debug)]
pub struct ForeignRow {
    pub dat_file: String,
    pub rowid: u64,
    pub key: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Val {
    Bool(bool),
    Integer(i64),
    Float(f32),
    String(String),
    Row(u64),
    ForeignRow(ForeignRow),
    Array(Vec<Val>),
}

impl Val {
    pub fn string(&self) -> &str {
        if let Val::String(string) = self {
            string
        } else {
            panic!("not a string");
        }
    }

    pub fn integer(&self) -> i64 {
        if let Val::Integer(i) = *self {
            i
        } else {
            panic!("not a bool");
        }
    }

    pub fn bool(&self) -> bool {
        if let Val::Bool(b) = *self {
            b
        } else {
            panic!("not a bool");
        }
    }

    pub fn row(&self) -> u64 {
        if let Val::Row(r) = *self {
            r
        } else {
            panic!("not a row");
        }
    }

    pub fn foreign_row(&self) -> &ForeignRow {
        if let Val::ForeignRow(fr) = self {
            fr
        } else {
            panic!("not a foreign row");
        }
    }

    pub fn skill_id(&self) -> u16 {
        if let Val::Integer(i) = *self {
            (i as i16) as u16
        } else {
            panic!("not an integer");
        }
    }
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

pub fn dump(dat_schema: &DatSchema, name: &str, strict: bool) -> io::Result<Vec<FxHashMap<String, Val>>> {
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
