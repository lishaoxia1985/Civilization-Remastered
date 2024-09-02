use bevy::{
    math::DVec2,
    prelude::{Component, Entity, Query},
};

use crate::{
    map::TileQuery,
    ruleset::{BaseTerrain, Feature, Ruleset, TerrainType},
    tile_map::{Direction, Hex, HexOrientation, MapParameters},
    River, TileStorage,
};

#[derive(Component, PartialEq, Eq, Hash)]
pub struct HexPosition(pub [i32; 2]);

impl HexPosition {
    pub fn pixel_position(&self, map_parameters: &MapParameters) -> DVec2 {
        map_parameters.hex_layout.hex_to_pixel(Hex::from(self.0))
    }

    pub fn corner_position(&self, direction: Direction, map_parameters: &MapParameters) -> DVec2 {
        map_parameters
            .hex_layout
            .corner(Hex::from(self.0), direction)
    }

    pub fn index(&self, map_parameters: &MapParameters) -> usize {
        let hex_coordinate = Hex::from(self.0);
        let offset_coordinate = hex_coordinate
            .to_offset_coordinate(map_parameters.offset, map_parameters.hex_layout.orientation);
        map_parameters.offset_coordinate_to_index(offset_coordinate)
    }

    pub fn entity(&self, map_parameters: &MapParameters, tile_storage: &TileStorage) -> Entity {
        let index = self.index(map_parameters);
        tile_storage.tiles[index]
    }

    pub fn entities_at_distance(
        &self,
        distance: i32,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
    ) -> Vec<Entity> {
        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        Hex::from(self.0)
            .hexes_at_distance(distance as u32)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                // Check if the offset coordinate is inside the map
                let [mut x, mut y] = offset_coordinate.to_array();

                if map_parameters.wrap_x {
                    x = (x % width + width) % width
                };
                if map_parameters.wrap_y {
                    y = (y % height + height) % height
                };

                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let index = map_parameters.offset_coordinate_to_index(offset_coordinate);
                    Some(tile_storage.tiles[index])
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn entity_neighbors(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
    ) -> Vec<Entity> {
        self.entities_at_distance(1, tile_storage, map_parameters)
    }

    pub fn entity_neighbor<'a>(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        direction: Direction,
    ) -> Option<Entity> {
        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        let orientation = map_parameters.hex_layout.orientation;
        let neighbor_offset_coordinate = Hex::from(self.0)
            .neighbor(orientation, direction)
            .to_offset_coordinate(map_parameters.offset, orientation);

        // Check if the offset coordinate is inside the map
        let [mut x, mut y] = neighbor_offset_coordinate.to_array();

        if map_parameters.wrap_x {
            x = (x % width + width) % width
        };
        if map_parameters.wrap_y {
            y = (y % height + height) % height
        };

        if !(x >= 0
            && x < map_parameters.map_size.width as i32
            && y >= 0
            && y < map_parameters.map_size.height as i32)
        {
            return None;
        }

        // Calculate the index of the neighbor tile
        let neighbor_index = map_parameters.offset_coordinate_to_index(neighbor_offset_coordinate);
        Some(tile_storage.tiles[neighbor_index])
    }

    pub fn entities_in_distance<'a>(
        &self,
        distance: i32,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
    ) -> Vec<Entity> {
        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        Hex::from(self.0)
            .hexes_in_distance(distance as u32)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                // Check if the offset coordinate is inside the map
                let [mut x, mut y] = offset_coordinate.to_array();

                if map_parameters.wrap_x {
                    x = (x % width + width) % width
                };
                if map_parameters.wrap_y {
                    y = (y % height + height) % height
                };

                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let index = map_parameters.offset_coordinate_to_index(offset_coordinate);
                    Some(tile_storage.tiles[index])
                } else {
                    None
                }
            })
            .collect()
    }

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
                    tile_index == self.entity(map_parameters,tile_storage) // 1. Check whether there is a river in the current tile
                        && river_position_and_flow_direction// 2. Check whether there is a river in the given direction of the tile according to the river flow direction
                            .iter()
                            .any(|&(river_position_direction, river_flow_directions)| {
                                direction == river_position_direction && river_flow_directions.contains(&river_flow_direction)
                            })
                })
            })
        } else if let Some(entity_neighbor) =
            self.entity_neighbor(tile_storage, map_parameters, direction)
        {
            let dir = direction.opposite_direction();

            let neighbor_tile = query_tile.get(entity_neighbor).unwrap();

            neighbor_tile.hex_position.has_river(
                dir,
                tile_storage,
                map_parameters,
                river,
                query_tile,
            )
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
        let entity = self.entity(map_parameters, tile_storage);
        let tile = query_tile.get(entity).unwrap();

        let direction_array = map_parameters.edge_direction_array();
        let has_river = direction_array.iter().any(|&direction| {
            self.has_river(direction, tile_storage, map_parameters, river, query_tile)
        });

        let entity_neighbor = self.entity_neighbors(tile_storage, map_parameters);
        tile.terrain_type != &TerrainType::Water
            && (has_river
                || entity_neighbor.iter().any(|&entity| {
                    let tile = query_tile.get(entity).unwrap();
                    tile.base_terrain == &BaseTerrain::Lake || tile.feature == Some(&Feature::Oasis)
                }))
    }

    pub fn is_impassable(
        &self,
        ruleset: &Ruleset,
        map_parameters: &MapParameters,
        tile_storage: &TileStorage,
        query_tile: &Query<TileQuery>,
    ) -> bool {
        let entity = self.entity(map_parameters, tile_storage);
        let tile = query_tile.get(entity).unwrap();
        tile.terrain_type == &TerrainType::Mountain
            || tile
                .feature
                .map_or(false, |feature| feature.impassable(ruleset))
            || tile
                .natural_wonder
                .map_or(false, |natural_wonder| natural_wonder.impassable(ruleset))
    }

    pub fn is_coastal_land(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query_tile: &Query<TileQuery>,
    ) -> bool {
        let entity = self.entity(map_parameters, tile_storage);
        let tile = query_tile.get(entity).unwrap();
        tile.terrain_type != &TerrainType::Water
            && self
                .entity_neighbors(tile_storage, map_parameters)
                .iter()
                .any(|&entity| {
                    let tile = query_tile.get(entity).unwrap();
                    tile.terrain_type == &TerrainType::Water
                })
    }
}
