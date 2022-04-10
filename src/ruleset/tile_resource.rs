use bevy::utils::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileResource {
    name: String,
    resource_type: String,
    #[serde(default)]
    terrains_can_be_found_on: Vec<String>,
    #[serde(default)]
    food: i8,
    #[serde(default)]
    production: i8,
    #[serde(default)]
    science: i8,
    #[serde(default)]
    gold: i8,
    #[serde(default)]
    culture: i8,
    #[serde(default)]
    faith: i8,
    #[serde(default)]
    happiness: i8,
    #[serde(default)]
    improvement: String,
    #[serde(default)]
    revealed_by: String,
    improvement_stats: Option<HashMap<String, i8>>,
    #[serde(default)]
    uniques: Vec<String>,
    major_deposit_amount: Option<HashMap<String, i8>>,
    minor_deposit_amount: Option<HashMap<String, i8>>,
}

impl Name for TileResource {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
