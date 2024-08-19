use bevy::{
    math::DVec2,
    prelude::{Component, Entity, Query, Res, With},
};

use crate::{
    ruleset::{BaseTerrain, Feature, NaturalWonder, Ruleset},
    TileStorage,
};

use super::{
    hex::{Direction, Hex, HexLayout},
    HexOrientation, MapParameters, TerrainType, TileMap,
};

#[derive(Component)]
pub struct Tile {
    pub hex_position: [i32; 2],
    pub terrain_type: TerrainType,
    pub base_terrain: BaseTerrain,
    /// if it's not None, Terrain Feature's name may be one of the following:
    /// - Forest, Jungle, Marsh, Floodplain, Oasis, Ice, Fallout.
    /// - Any natural wonder.
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
        TileMap::offset_coordinate_to_index(map_parameters.map_size, offset_coordinate)
    }

    fn check_component<T: Component + std::cmp::PartialEq>(
        &self,
        check: &T,
        query: &Query<(Entity, &T)>,
        map_parameters: &MapParameters,
        tile_storage: &TileStorage,
    ) -> bool {
        if let Ok((_, component)) = query.get(self.entity(map_parameters, tile_storage)) {
            if component == check {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn entity(&self, map_parameters: &MapParameters, tile_storage: &TileStorage) -> Entity {
        let index = self.index(map_parameters);
        tile_storage.tiles[index]
    }

    pub fn entites_at_distance(
        &self,
        distance: i32,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
    ) -> Vec<Entity> {
        Hex::from(self.hex_position)
            .hexes_at_distance(distance as u32)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                // Check if the offset coordinate is inside the map
                let [x, y] = offset_coordinate.to_array();
                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let index = TileMap::offset_coordinate_to_index(
                        map_parameters.map_size,
                        offset_coordinate,
                    );
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
        self.entites_at_distance(1, tile_storage, map_parameters)
    }

    pub fn entity_is_water(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query: &Query<(Entity, &TerrainType)>,
    ) -> bool {
        /* query
        .get(self.entity(map_parameters, tile_storage))
        .unwrap()
        .1
        == &TerrainType::Water */
        self.check_component(&TerrainType::Water, query, map_parameters, tile_storage)
    }

    pub fn entity_is_flatland(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query: &Query<(Entity, &TerrainType), With<Tile>>,
    ) -> bool {
        query
            .get(self.entity(map_parameters, tile_storage))
            .unwrap()
            .1
            == &TerrainType::Flatland
    }

    pub fn entity_is_hill(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query: &Query<(Entity, &TerrainType), With<Tile>>,
    ) -> bool {
        query
            .get(self.entity(map_parameters, tile_storage))
            .unwrap()
            .1
            == &TerrainType::Hill
    }

    pub fn entity_is_mountain(
        &self,
        tile_storage: &TileStorage,
        map_parameters: &MapParameters,
        query: &Query<(Entity, &TerrainType), With<Tile>>,
    ) -> bool {
        query
            .get(self.entity(map_parameters, tile_storage))
            .unwrap()
            .1
            == &TerrainType::Mountain
    }

    pub fn tile_neighbors<'a>(
        &'a self,
        tile_map: &'a TileMap,
        map_parameters: &MapParameters,
    ) -> Vec<&Tile> {
        self.tiles_at_distance(1, tile_map, map_parameters)
    }

    pub fn tile_neighbor<'a>(
        &'a self,
        tile_map: &'a TileMap,
        direction: Direction,
        map_parameters: &MapParameters,
    ) -> Option<&Tile> {
        let orientation = map_parameters.hex_layout.orientation;
        let neighbor_offset_coordinate = Hex::from(self.hex_position)
            .neighbor(orientation, direction)
            .to_offset_coordinate(map_parameters.offset, orientation);

        // Check if the offset coordinate is inside the map
        let [x, y] = neighbor_offset_coordinate.to_array();
        if !(x >= 0
            && x < map_parameters.map_size.width as i32
            && y >= 0
            && y < map_parameters.map_size.height as i32)
        {
            return None;
        }

        // Calculate the index of the neighbor tile
        let neighbor_index = TileMap::offset_coordinate_to_index(
            map_parameters.map_size,
            neighbor_offset_coordinate,
        );

        // Return the neighbor tile if it exists
        tile_map.tile_list.get(neighbor_index)
    }

    pub fn has_river(
        &self,
        direction: Direction,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
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
            tile_map.river_list.values().any(|river| {
                river.iter().any(|&(tile_index, river_flow_direction)| {
                    tile_index == self.index(map_parameters) // 1. Check whether there is a river in the current tile
                        && river_position_and_flow_direction// 2. Check whether there is a river in the given direction of the tile according to the river flow direction
                            .iter()
                            .any(|&(river_position_direction, river_flow_directions)| {
                                direction == river_position_direction && river_flow_directions.contains(&river_flow_direction)
                            })
                })
            })
        } else if let Some(neighbor_tile) = self.tile_neighbor(tile_map, direction, map_parameters)
        {
            let dir = direction.opposite_direction();
            neighbor_tile.has_river(dir, tile_map, map_parameters)
        } else {
            false
        }
    }

    pub fn tiles_in_distance<'a>(
        &'a self,
        distance: i32,
        tile_map: &'a TileMap,
        map_parameters: &MapParameters,
    ) -> Vec<&Tile> {
        Hex::from(self.hex_position)
            .hexes_in_distance(distance as u32)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                // Check if the offset coordinate is inside the map
                let [x, y] = offset_coordinate.to_array();
                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let index = TileMap::offset_coordinate_to_index(
                        map_parameters.map_size,
                        offset_coordinate,
                    );
                    tile_map.tile_list.get(index)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn tiles_at_distance<'a>(
        &'a self,
        distance: i32,
        tile_map: &'a TileMap,
        map_parameters: &MapParameters,
    ) -> Vec<&Tile> {
        Hex::from(self.hex_position)
            .hexes_at_distance(distance as u32)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                // Check if the offset coordinate is inside the map
                let [x, y] = offset_coordinate.to_array();
                if x >= 0
                    && x < map_parameters.map_size.width as i32
                    && y >= 0
                    && y < map_parameters.map_size.height as i32
                {
                    let index = TileMap::offset_coordinate_to_index(
                        map_parameters.map_size,
                        offset_coordinate,
                    );
                    tile_map.tile_list.get(index)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn pixel_position(&self, layout: HexLayout) -> DVec2 {
        layout.hex_to_pixel(Hex::from(self.hex_position))
    }

    pub fn tile_corner_position(
        &self,
        direction: Direction,
        map_parameters: &MapParameters,
    ) -> DVec2 {
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
        self.tile_neighbors(tile_map, map_parameters)
            .iter()
            .any(|tile| {
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

    pub fn is_impassable(&self, ruleset: &Res<Ruleset>) -> bool {
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
        let direction_array = tile_map.tile_edge_direction(map_parameters);
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
                .tile_neighbors(tile_map, map_parameters)
                .iter()
                .any(|tile| tile.is_water())
    }
}
