use bevy::math::DVec2;

use crate::{
    grid::{
        hex::{Hex, HexLayout, HexOrientation, OffsetCoordinate},
        Direction,
    },
    map::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        terrain_type::TerrainType,
    },
    ruleset::Ruleset,
};

use super::{MapParameters, TileMap};

pub struct Tile {
    pub hex_position: [i32; 2],
    pub terrain_type: TerrainType,
    pub base_terrain: BaseTerrain,
    pub feature: Option<Feature>,
    pub natural_wonder: Option<NaturalWonder>,
    pub area_id: i32,
}

impl Tile {
    pub fn new(hex_position: [i32; 2]) -> Tile {
        Tile {
            hex_position,
            terrain_type: TerrainType::Water,
            base_terrain: BaseTerrain::Ocean,
            feature: None,
            natural_wonder: None,
            area_id: -1,
        }
    }

    pub fn index(&self, map_parameters: &MapParameters) -> usize {
        let hex_coordinate = Hex::from(self.hex_position);
        let offset_coordinate = hex_coordinate
            .to_offset_coordinate(map_parameters.offset, map_parameters.hex_layout.orientation);
        map_parameters.offset_coordinate_to_index(offset_coordinate)
    }

    pub fn tile_neighbors<'a>(&'a self, map_parameters: &MapParameters) -> Vec<usize> {
        self.tiles_at_distance(1, map_parameters)
    }

    /// Returns the index of the tile neighbor to the given tile.
    pub fn tile_neighbor<'a>(
        &'a self,
        direction: Direction,
        map_parameters: &MapParameters,
    ) -> Option<usize> {
        let orientation = map_parameters.hex_layout.orientation;
        let neighbor_offset_coordinate = Hex::from(self.hex_position)
            .neighbor(orientation, direction)
            .to_offset_coordinate(map_parameters.offset, orientation);

        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        let [mut x, mut y] = neighbor_offset_coordinate.to_array();

        if map_parameters.wrap_x {
            x = (x % width + width) % width
        };
        if map_parameters.wrap_y {
            y = (y % height + height) % height
        };

        // Check if the offset coordinate is inside the map
        if x >= 0
            && x < map_parameters.map_size.width as i32
            && y >= 0
            && y < map_parameters.map_size.height as i32
        {
            let neighbor_offset_coordinate = OffsetCoordinate::new(x, y);
            Some(map_parameters.offset_coordinate_to_index(neighbor_offset_coordinate))
        } else {
            None
        }
    }

    /// Checks if the tile has a river flowing in the given direction.
    /// Returns true if the tile has a river flowing in the given direction, false otherwise.
    pub fn has_river(
        &self,
        direction: Direction,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        // river_edge_direction refers to the direction of the river edge located on the current tile.
        // flow_direction refers to the direction of the river flow.
        // For example, when hex is `HexOrientation::Pointy`, if the river is flowing North or South, the river_edge_direction is East.
        let river_edge_direction_and_flow_directions = match map_parameters.hex_layout.orientation {
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
        let check_tile_index;
        let check_edge_direction;
        if edge_index < 3 {
            check_tile_index = self.index(map_parameters);
            check_edge_direction = direction;
        } else {
            if let Some(neighbor_tile_index) = self.tile_neighbor(direction, map_parameters) {
                check_tile_index = neighbor_tile_index;
                check_edge_direction = direction.opposite_direction();
            } else {
                return false;
            }
        }

        tile_map.river_list.values().flatten().any(
            |&(tile_index, river_flow_direction)| {
                tile_index == check_tile_index // 1. Check whether there is a river in the current tile
                    && river_edge_direction_and_flow_directions// 2. Check whether there is a river in the given direction of the tile according to the river flow direction
                        .iter()
                        .any(|&(river_edge_direction, river_flow_directions)| {
                            check_edge_direction == river_edge_direction && river_flow_directions.contains(&river_flow_direction)
                        })
            })
    }

    pub fn tiles_in_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<usize> {
        Hex::from(self.hex_position)
            .hexes_in_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                let width = map_parameters.map_size.width as i32;
                let height = map_parameters.map_size.height as i32;
                let [mut x, mut y] = offset_coordinate.to_array();

                if map_parameters.wrap_x {
                    x = (x % width + width) % width
                };
                if map_parameters.wrap_y {
                    y = (y % height + height) % height
                };

                // Check if the offset coordinate is inside the map
                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    Some(map_parameters.offset_coordinate_to_index(offset_coordinate))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the indices of the tiles at the given distance from the current tile.
    pub fn tiles_at_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<usize> {
        Hex::from(self.hex_position)
            .hexes_at_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                let width = map_parameters.map_size.width as i32;
                let height = map_parameters.map_size.height as i32;
                let [mut x, mut y] = offset_coordinate.to_array();

                if map_parameters.wrap_x {
                    x = (x % width + width) % width
                };
                if map_parameters.wrap_y {
                    y = (y % height + height) % height
                };

                // Check if the offset coordinate is inside the map
                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    Some(map_parameters.offset_coordinate_to_index(offset_coordinate))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn pixel_position(&self, layout: HexLayout) -> DVec2 {
        layout.hex_to_pixel(Hex::from(self.hex_position))
    }

    pub fn corner_position(&self, direction: Direction, map_parameters: &MapParameters) -> DVec2 {
        map_parameters
            .hex_layout
            .corner(Hex::from(self.hex_position), direction)
    }

    /// Check if the tile is adjacent to the terrain name
    ///
    /// `terrain_name` can be a BaseTerrain name or a Feature name, but not a TerrainType or Natural name.
    pub fn is_adjacent_to(
        &self,
        terrain_name: &str,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        self.tile_neighbors(map_parameters)
            .iter()
            .any(|&tile_index| {
                let tile = tile_map.tile(tile_index);
                tile.base_terrain.name() == terrain_name
                    || tile
                        .feature
                        .as_ref()
                        .map_or(false, |feature| feature.name() == terrain_name)
            })
    }

    pub fn is_mountain(&self) -> bool {
        self.terrain_type == TerrainType::Mountain
    }

    pub fn is_hill(&self) -> bool {
        self.terrain_type == TerrainType::Hill
    }

    /// Check if the tile is land, when it returns true, it means it is not water or hill or mountain.
    pub fn is_flatland(&self) -> bool {
        self.terrain_type == TerrainType::Flatland
    }

    pub fn is_water(&self) -> bool {
        self.terrain_type == TerrainType::Water
    }

    pub fn is_natural_wonder(&self) -> bool {
        self.natural_wonder.is_some()
    }

    pub fn is_impassable(&self, ruleset: &Ruleset) -> bool {
        self.is_mountain()
            || self
                .feature
                .as_ref()
                .map_or(false, |feature| feature.impassable(ruleset))
            || self
                .natural_wonder
                .as_ref()
                .map_or(false, |natural_wonder| natural_wonder.impassable(ruleset))
    }

    pub fn is_freshwater(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        let direction_array = map_parameters.edge_direction_array();
        let has_river = direction_array
            .iter()
            .any(|&direction| self.has_river(direction, tile_map, map_parameters));
        (!self.is_water())
            && (self.is_adjacent_to("Lake", tile_map, map_parameters)
                || self.is_adjacent_to("Oasis", tile_map, map_parameters)
                || has_river)
    }

    pub fn is_coastal_land(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        !self.is_water()
            && self
                .tile_neighbors(map_parameters)
                .iter()
                .any(|tile_index: &usize| {
                    let tile = tile_map.tile(*tile_index);
                    tile.is_water()
                })
    }
}
