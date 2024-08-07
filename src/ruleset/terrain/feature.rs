use bevy::utils::HashMap;

use serde::{Deserialize, Serialize};

use super::{base_terrain::BaseTerrain, Name, TerrainFeature};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureInfo {
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
    pub impassable: bool,
    #[serde(default)]
    pub unbuildable: bool,
    #[serde(default)]
    pub override_stats: bool,
    #[serde(default)]
    pub occurs_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

impl TerrainFeature for FeatureInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn r#type(&self) -> String {
        self.r#type.to_owned()
    }

    fn impassable(&self) -> bool {
        self.impassable
    }
}

impl Name for FeatureInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl FeatureInfo {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Feature {
    Forest,
    Jungle,
    Marsh,
    Floodplain,
    Oasis,
    Ice,
    Atoll,
    Fallout,
}

impl Feature {
    pub fn name(&self) -> &str {
        match self {
            Feature::Forest => "Forest",
            Feature::Jungle => "Jungle",
            Feature::Marsh => "Marsh",
            Feature::Floodplain => "Floodplain",
            Feature::Oasis => "Oasis",
            Feature::Ice => "Ice",
            Feature::Atoll => "Atoll",
            Feature::Fallout => "Fallout",
        }
    }
}
