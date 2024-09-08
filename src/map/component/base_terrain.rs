use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

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
