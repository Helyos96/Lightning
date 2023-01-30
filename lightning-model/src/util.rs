use crate::build::Build;
use crate::import;
use serde::{Deserialize, Deserializer};
use std::fs;
use std::path::PathBuf;

pub fn load_build(path: &PathBuf) -> Result<Build, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let player: Build = serde_json::from_str(&data)?;
    Ok(player)
}

