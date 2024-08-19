use bevy::{prelude::Component, utils::HashMap};

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseTerrainInfo {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub food: i8,
    #[serde(default)]
    pub production: i8,
    #[serde(default)]
    pub science: i8,
    #[serde(default)]
    pub gold: i8,
    #[serde(default)]
    pub culture: i8,
    #[serde(default)]
    pub faith: i8,
    #[serde(default)]
    pub happiness: i8,
    #[serde(default)]
    pub movement_cost: i8,
    #[serde(rename = "RGB")]
    pub rgb: Option<[u8; 3]>,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

impl Name for BaseTerrainInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl BaseTerrainInfo {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug, Component)]
pub enum BaseTerrain {
    Ocean,
    Lake,
    Coast,
    Grassland,
    Desert,
    Plain,
    Tundra,
    Snow,
}

impl BaseTerrain {
    pub fn name(&self) -> &str {
        match self {
            BaseTerrain::Ocean => "Ocean",
            BaseTerrain::Lake => "Lake",
            BaseTerrain::Coast => "Coast",
            BaseTerrain::Grassland => "Grassland",
            BaseTerrain::Desert => "Desert",
            BaseTerrain::Plain => "Plain",
            BaseTerrain::Tundra => "Tundra",
            BaseTerrain::Snow => "Snow",
        }
    }
}
