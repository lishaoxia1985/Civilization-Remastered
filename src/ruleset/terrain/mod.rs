pub mod base_terrain;
pub mod feature;
pub mod natural_wonder;
pub mod terrain_type;

pub use base_terrain::BaseTerrain;

use super::Name;

pub trait TerrainFeature {
    fn name(&self) -> String;
    fn r#type(&self) -> String;
    fn impassable(&self) -> bool;
}
