use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug, Component)]
pub enum TerrainType {
    Water,
    Flatland,
    Mountain,
    Hill,
}

impl TerrainType {
    pub fn name(&self) -> &str {
        match self {
            TerrainType::Water => "Water",
            TerrainType::Flatland => "Flatland",
            TerrainType::Mountain => "Mountain",
            TerrainType::Hill => "Hill",
        }
    }
}
