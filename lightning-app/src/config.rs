use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
/// config.json
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub builds_dir: PathBuf,
    pub import_accounts: Vec<String>,
    pub framerate: u64,
    pub vsync: bool,
    #[serde(default)]
    pub show_debug: bool,
    #[serde(default = "default_ui_zoom")]
    pub ui_zoom_factor_pct: u32,
}

fn default_ui_zoom() -> u32 {
    100
}

#[cfg(target_os = "linux")]
pub fn config_dir() -> PathBuf {
    if let Some(path) = dirs::home_dir() {
        return path.join(".local/lightning_poe/");
    }

    PathBuf::from("./")
}

pub fn create_config_builds_dir() -> Result<(), std::io::Error> {
    let path = config_dir().join("builds/");

    if !path.exists() {
        fs::create_dir_all(path)?
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn config_dir() -> PathBuf {
    if let Some(path) = dirs::home_dir() {
        return path.join("lightning_poe/");
    }

    PathBuf::from("./")
}

#[cfg(target_os = "android")]
pub fn config_dir() -> PathBuf {
    PathBuf::from("./")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            builds_dir: config_dir().join("builds/"),
            import_accounts: vec![],
            framerate: 60,
            vsync: true,
            show_debug: false,
            ui_zoom_factor_pct: 100,
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
