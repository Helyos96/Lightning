use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
/// config.json
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub builds_dir: PathBuf,
    pub import_accounts: Vec<String>,
}

#[cfg(target_os = "linux")]
fn config_dir() -> PathBuf {
    if let Some(path) = dirs::home_dir() {
        return path.join(".local/lightning_poe/");
    }

    PathBuf::from("./")
}

#[cfg(target_os = "windows")]
fn config_dir() -> PathBuf {
    if let Some(path) = dirs::home_dir() {
        return path.join("lightning_poe/");
    }

    PathBuf::from("./")
}

#[cfg(target_os = "android")]
fn config_dir() -> PathBuf {
    PathBuf::from("./")
}

impl Default for Config {
    fn default() -> Self {
        let path = config_dir().join("config.json");
        if let Ok(file) = fs::File::open(path) {
            if let Ok(config) = serde_json::from_reader(&file) {
                return config;
            }
        }
        Self {
            builds_dir: config_dir().join("builds/"),
            import_accounts: vec![],
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(config_dir())?;
        serde_json::to_writer(&fs::File::create(config_dir().join("config.json"))?, self)?;
        Ok(())
    }
}
