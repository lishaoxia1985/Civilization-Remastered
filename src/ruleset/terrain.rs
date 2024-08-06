use bevy::utils::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terrain {
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
    pub defence_bonus: f32,
    #[serde(default)]
    pub movement_cost: i8,
    #[serde(default)]
    pub turns_into_type: Option<TerrainType>,
    #[serde(default)]
    pub turns_into_base: Option<BaseTerrain>,
    #[serde(default)]
    pub impassable: bool,
    #[serde(default)]
    pub unbuildable: bool,
    #[serde(default)]
    pub weight: i8,
    #[serde(default)]
    pub override_stats: bool,
    #[serde(default)]
    pub occurs_on_type: Vec<TerrainType>,
    #[serde(default)]
    pub occurs_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub occurs_on_feature: Vec<String>,
    #[serde(rename = "RGB")]
    pub rgb: Option<[u8; 3]>,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum TerrainType {
    Water,
    Flatland,
    Mountain,
    Hill,
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
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
    pub const fn name(&self) -> &str {
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

impl Name for Terrain {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl Terrain {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}
