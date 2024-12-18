use std::io::{self, Cursor};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::Deserialize;

/// Structures for deserializing https://github.com/poe-tool-dev/dat-schema/releases/download/latest/schema.min.json

#[derive(Deserialize)]
pub struct DatSchema {
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(default)]
    pub tables: Vec<Table>
}

#[derive(Deserialize, Clone)]
pub struct Table {
    #[serde(default)]
    pub columns: Vec<Column>,
    pub name: String,
    #[serde(rename = "validFor")]
    pub valid_for: u64
}

#[derive(Deserialize, Clone)]
pub struct Column {
    pub array: bool,
    pub description: Option<String>,
    pub file: Option<String>,
    pub name: Option<String>,
    pub references: Option<Reference>,
    pub r#type: Type,
    pub unique: bool,
}

#[derive(Deserialize, Clone)]
pub struct Reference {
    pub column: Option<String>,
    pub table: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Type {
    bool,
    array,
    foreignrow,
    string,
    enumrow,
    row,
    i32,
    i16,
    f32,
}

impl Type {
    pub fn is_var_data(&self) -> bool {
        use Type::*;
        match self {
            string|array => true,
            _ => false,
        }
    }

    pub fn var_offset(&self, cursor: &mut Cursor<&Vec<u8>>) -> io::Result<u64> {
        use Type::*;
        assert!(self.is_var_data());
        match self {
            string => {
                Ok(cursor.read_u64::<LittleEndian>()?)
            },
            array => {
                cursor.read_u64::<LittleEndian>()?;
                Ok(cursor.read_u64::<LittleEndian>()?)
            },
            _ => Ok(0),
        }
    }
}
