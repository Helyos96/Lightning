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

pub fn fetch_build(account: &str, character: &str) -> Result<Build, Box<dyn std::error::Error>> {
    let player = import::character(account, character)?;
    Ok(player)
}

pub fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
