use bevy::{
    math::DVec2,
    prelude::{Component, Entity},
};

use crate::{
    grid::hex::{Direction, Hex},
    tile_map::MapParameters,
    TileStorage,
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
}
