use bevy::{ecs::query::QueryData, prelude::Entity};

use crate::ruleset::{BaseTerrain, Feature, NaturalWonder, TerrainType};

use super::{component::AreaId, HexPosition};

#[derive(QueryData)]
pub struct TileQuery {
    pub entity: Entity,
    pub hex_position: &'static HexPosition,
    pub terrain_type: &'static TerrainType,
    pub base_terrain: &'static BaseTerrain,
    pub feature: Option<&'static Feature>,
    pub natural_wonder: Option<&'static NaturalWonder>,
    pub area_id: &'static AreaId,
}
