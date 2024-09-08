use bevy::{
    ecs::query::QueryData,
    prelude::{Entity, Query},
};

use crate::{
    grid::hex::{Direction, HexOrientation},
    ruleset::Ruleset,
    tile_map::MapParameters,
};

use super::{
    base_terrain::BaseTerrain, component::AreaId, feature::Feature, natural_wonder::NaturalWonder,
    terrain_type::TerrainType, HexPosition, River, TileStorage,
};

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

impl TileQueryItem<'_> {
    pub fn has_river(
        &self,
        direction: Direction,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        river: &River,
        query_tile: &Query<TileQuery>,
    ) -> bool {
        // This variable is related to river direction position and river flow direction
        let river_position_and_flow_direction = match map_parameters.hex_layout.orientation {
            HexOrientation::Pointy => [
                (Direction::East, [Direction::North, Direction::South]),
                (
                    Direction::SouthEast,
                    [Direction::NorthEast, Direction::SouthWest],
                ),
                (
                    Direction::SouthWest,
                    [Direction::NorthWest, Direction::SouthEast],
                ),
            ],
            HexOrientation::Flat => [
                (
                    Direction::NorthEast,
                    [Direction::NorthWest, Direction::SouthEast],
                ),
                (
                    Direction::SouthEast,
                    [Direction::NorthEast, Direction::SouthWest],
                ),
                (Direction::South, [Direction::East, Direction::West]),
            ],
        };

        let edge_index = map_parameters.hex_layout.orientation.edge_index(direction);

        if edge_index < 3 {
            river.0.values().any(|river| {
                river.iter().any(|&(tile_index, river_flow_direction)| {
                    tile_index == self.hex_position.entity(map_parameters,tile_storage) // 1. Check whether there is a river in the current tile
                        && river_position_and_flow_direction// 2. Check whether there is a river in the given direction of the tile according to the river flow direction
                            .iter()
                            .any(|&(river_position_direction, river_flow_directions)| {
                                direction == river_position_direction && river_flow_directions.contains(&river_flow_direction)
                            })
                })
            })
        } else if let Some(entity_neighbor) =
            self.hex_position
                .entity_neighbor(tile_storage, map_parameters, direction)
        {
            let dir = direction.opposite_direction();

            let neighbor_tile = query_tile.get(entity_neighbor).unwrap();

            neighbor_tile.has_river(dir, tile_storage, map_parameters, river, query_tile)
        } else {
            false
        }
    }

    pub fn is_freshwater(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        river: &River,
        query_tile: &Query<TileQuery>,
    ) -> bool {
        let direction_array = map_parameters.edge_direction_array();
        let has_river = direction_array.iter().any(|&direction| {
            self.has_river(direction, tile_storage, map_parameters, river, query_tile)
        });

        let entity_neighbor = self
            .hex_position
            .entity_neighbors(tile_storage, map_parameters);
        self.terrain_type != &TerrainType::Water
            && (has_river
                || entity_neighbor.iter().any(|&entity| {
                    let tile = query_tile.get(entity).unwrap();
                    tile.base_terrain == &BaseTerrain::Lake || tile.feature == Some(&Feature::Oasis)
                }))
    }

    pub fn is_impassable(&self, ruleset: &Ruleset) -> bool {
        self.terrain_type == &TerrainType::Mountain
            || self
                .feature
                .map_or(false, |feature| feature.impassable(ruleset))
            || self
                .natural_wonder
                .map_or(false, |natural_wonder| natural_wonder.impassable(ruleset))
    }

    pub fn is_coastal_land(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query_tile: &Query<TileQuery>,
    ) -> bool {
        self.terrain_type != &TerrainType::Water
            && self
                .hex_position
                .entity_neighbors(tile_storage, map_parameters)
                .iter()
                .any(|&entity| {
                    let tile = query_tile.get(entity).unwrap();
                    tile.terrain_type == &TerrainType::Water
                })
    }
}
