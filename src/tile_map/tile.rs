use std::sync::Arc;

use bevy::{math::DVec2, prelude::Res};

use crate::ruleset::{Ruleset, Terrain};

use super::{
    hex::{Direction, Hex, HexLayout},
    HexOrientation, TileMap,
};

pub struct Tile {
    pub hex_position: [i32; 2],
    pub base_terrain: Arc<Terrain>,
    pub terrain_features: Vec<Arc<Terrain>>,
    pub area_id: i32,
}

impl Tile {
    pub fn new(hex_position: [i32; 2], ruleset: &Res<Ruleset>) -> Tile {
        Tile {
            hex_position,
            base_terrain: ruleset.terrains["Ocean"].clone(),
            terrain_features: Vec::new(),
            area_id: -1,
        }
    }

    pub fn tiles_neighbors<'a>(&'a self, tile_map: &'a TileMap) -> Vec<&Tile> {
        self.tiles_at_distance(1, tile_map)
    }

    pub fn tile_neighbor<'a>(
        &'a self,
        tile_map: &'a TileMap,
        direction: Direction,
    ) -> Option<&Tile> {
        let direction_array = tile_map.tile_edge_direction();
        let neighbor_tile_position = direction_array
            .iter()
            .position(|&x| x == direction)
            .unwrap() as i32;
        let hex_position = Hex::from(self.hex_position)
            .hex_neighbor(neighbor_tile_position)
            .to_array();
        tile_map.tile_list.get(&hex_position)
    }

    pub fn has_river(&self, direction: Direction, tile_map: &TileMap) -> bool {
        // this var is ralated to river direction position and river flow direction
        let ralated_direction = match tile_map.map_parameters.hex_layout.orientation {
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
        let direction_array = tile_map.tile_edge_direction();
        let index = direction_array
            .iter()
            .position(|&x| x == direction)
            .unwrap() as i32;
        if index < 3 {
            tile_map.river_list.values().any(|river| {
                river.iter().any(|&(hex_position, river_flow_direction)| {
                    hex_position == self.hex_position // 1. Check whether there is a river in the current tile
                        && ralated_direction// 2. Check whether there is a river in the given direction of the tile according to the river flow direction
                            .iter()
                            .any(|&(river_position_direction, river_flow_directions)| {
                                direction == river_position_direction && river_flow_directions.contains(&river_flow_direction)
                            })
                })
            })
        } else if let Some(neighbor_tile) = self.tile_neighbor(tile_map, direction) {
            let dir = direction.opposite_direction();
            neighbor_tile.has_river(dir, tile_map)
        } else {
            false
        }
    }

    pub fn tiles_in_distance<'a>(&'a self, distance: i32, tile_map: &'a TileMap) -> Vec<&Tile> {
        Hex::from(self.hex_position)
            .hexes_in_distance(distance as u32)
            .iter()
            .filter_map(|x| tile_map.tile_list.get(&x.to_array()))
            .collect()
    }

    pub fn tiles_at_distance<'a>(&'a self, distance: i32, tile_map: &'a TileMap) -> Vec<&Tile> {
        Hex::from(self.hex_position)
            .hexes_at_distance(distance as u32)
            .iter()
            .filter_map(|x| tile_map.tile_list.get(&x.to_array()))
            .collect()
    }

    pub fn pixel_position(&self, layout: HexLayout) -> DVec2 {
        layout.hex_to_pixel(Hex::from(self.hex_position))
    }

    pub fn tile_corner_position(&self, direction: Direction, tile_map: &TileMap) -> DVec2 {
        let direction_array = tile_map.tile_corner_direction();
        let corner_position = direction_array
            .iter()
            .position(|&x| x == direction)
            .unwrap() as i32;
        tile_map
            .map_parameters
            .hex_layout
            .polygon_corner(Hex::from(self.hex_position), corner_position)
    }

    pub fn is_adjacent_to(&self, terrain: &str, tile_map: &TileMap) -> bool {
        self.tiles_neighbors(tile_map).iter().any(|tile| {
            tile.base_terrain.name == terrain
                || tile.terrain_features.iter().any(|x| x.name == terrain)
        })
    }

    pub fn is_mountain(&self) -> bool {
        self.base_terrain.name == "Mountain"
    }

    pub fn is_hill(&self) -> bool {
        self.terrain_features.iter().any(|x| x.name == "Hill")
    }

    pub fn is_water(&self) -> bool {
        self.base_terrain.r#type == "Water"
    }

    pub fn is_freshwater(&self, tile_map: &TileMap) -> bool {
        let direction_array = tile_map.tile_edge_direction();
        let has_river = direction_array
            .iter()
            .any(|&direction| self.has_river(direction, tile_map));
        self.is_land()
            && (self.is_adjacent_to("Lakes", tile_map)
                || self.is_adjacent_to("Oasis", tile_map)
                || has_river)
    }

    pub fn is_land(&self) -> bool {
        self.base_terrain.r#type == "Land"
    }

    pub fn is_coastal_land(&self, tile_map: &TileMap) -> bool {
        self.is_land()
            && self
                .tiles_neighbors(tile_map)
                .iter()
                .any(|tile| tile.is_water())
    }
}
