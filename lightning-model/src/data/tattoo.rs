use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TattooType {
    Node,
    Mastery,
    Keystone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TattooData {
    pub stats: Vec<String>,
    pub tattoo_type: TattooType,
    pub icon: String,
    pub active_effect_image: String,
}
