use std::io::{self, Cursor, Seek};
use byteorder::{LittleEndian, ReadBytesExt};
use crate::utils::read_file;

/// Parser for passiveskillgraph.psg and other PSG files (PoE2 0.1.0)
/// Adapted from https://github.com/Project-Path-of-Exile-Wiki/PyPoE/blob/dev/PyPoE/poe/file/psg.py

#[derive(Debug)]
pub struct Graph {
    pub root_nodes: Vec<(u32, u32)>,
    pub groups: Vec<Group>,
}

#[derive(Debug)]
pub struct Group {
    pub x: f32,
    pub y: f32,
    pub nodes: Vec<Node>,
    pub flag: u32,
}

#[derive(Debug)]
pub struct Node {
    pub group: u32,
    pub passive_skill: u32,
    pub radius: u32,
    pub position: u32,
    // (node, unk)
    pub connections: Vec<(u32, i32)>,
}

pub fn parse_psg(filename: &str) -> io::Result<Graph> {
    let buf = read_file(filename)?;
    let mut cursor = Cursor::new(&buf);
    // Ignore 13 unk bytes
    cursor.seek_relative(13)?;
    let root_length = cursor.read_u32::<LittleEndian>()?;
    let root_nodes: Vec<(u32, u32)> = (0..root_length).into_iter().map(|_| (cursor.read_u32::<LittleEndian>().unwrap(), cursor.read_u32::<LittleEndian>().unwrap())).collect();
    let group_length = cursor.read_u32::<LittleEndian>()?;
    let mut groups = vec![];
    for group in 0..group_length {
        let x = cursor.read_f32::<LittleEndian>()?;
        let y = cursor.read_f32::<LittleEndian>()?;
        let flag = cursor.read_u32::<LittleEndian>()?;
        cursor.seek_relative(4 + 1)?;
        let passive_length = cursor.read_u32::<LittleEndian>()?;
        let mut nodes = vec![];
        for _ in 0..passive_length {
            let rowid = cursor.read_u32::<LittleEndian>()?;
            let radius = cursor.read_u32::<LittleEndian>()?;
            let position = cursor.read_u32::<LittleEndian>()?;
            let connections_length = cursor.read_u32::<LittleEndian>()?;
            let connections: Vec<(u32, i32)> = (0..connections_length).into_iter().map(|_| (cursor.read_u32::<LittleEndian>().unwrap(), cursor.read_i32::<LittleEndian>().unwrap())).collect();
            nodes.push(Node {
                group: group + 1,
                passive_skill: rowid,
                radius,
                position,
                connections,
            });
        }
        groups.push(Group {
            x,
            y,
            nodes,
            flag,
        });
    }
    Ok(Graph { root_nodes, groups })
}
