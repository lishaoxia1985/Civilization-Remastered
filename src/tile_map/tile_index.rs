use bevy::{math::DVec2, prelude::Deref};

use crate::{
    grid::{
        hex::{Hex, HexOrientation, OffsetCoordinate},
        Direction,
    },
    map::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        terrain_type::TerrainType,
    },
    ruleset::Ruleset,
};

use super::{MapParameters, TileMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, Hash, PartialOrd, Ord)]
pub struct TileIndex(usize);

impl TileIndex {
    #[inline]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Converts an offset coordinate to a tile index in the map.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to the map parameters, which includes map size and wrapping behavior.
    /// - `offset_coordinate`: The offset coordinate to convert.
    ///
    /// # Returns
    /// - `Result<Self, String>`: Returns an instance of `Self` if the coordinate is valid,
    ///   or an error message if the coordinate is outside the map bounds.
    pub fn from_offset_coordinate(
        map_parameters: &MapParameters,
        offset_coordinate: OffsetCoordinate,
    ) -> Result<Self, String> {
        let map_size = map_parameters.map_size;
        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        // Check if the offset coordinate is inside the map
        let [mut x, mut y] = offset_coordinate.to_array();

        if map_parameters.wrap_x {
            x = x.rem_euclid(width);
        };
        if map_parameters.wrap_y {
            y = y.rem_euclid(height);
        };

        if x >= 0 && x < width && y >= 0 && y < height {
            let index = (x + y * map_size.width) as usize;
            Ok(Self(index))
        } else {
            Err(String::from("offset coordinate is outside the map!"))
        }
    }

    /// Converts a tile index into an offset coordinate based on the map parameters.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the dimensions of the map.
    ///
    /// # Returns
    /// An `OffsetCoordinate` representing the (x, y) position derived from the tile index.
    ///
    /// # Panics
    /// This method will panic if the tile index is out of bounds for the given map size.
    pub fn to_offset_coordinate(&self, map_parameters: &MapParameters) -> OffsetCoordinate {
        let map_width = map_parameters.map_size.width;
        let map_height = map_parameters.map_size.height;

        assert!(
            self.0 < (map_width * map_height) as usize,
            "Index out of bounds"
        );

        let x = self.0 as i32 % map_width;
        let y = self.0 as i32 / map_width;

        OffsetCoordinate::new(x, y)
    }

    /// Converts the current tile index to a hexagonal coordinate based on the map parameters.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the dimensions and layout of the map.
    ///
    /// # Returns
    /// A `Hex` representing the hexagonal coordinates derived from the current tile index.
    ///
    /// # Panics
    /// This method will panic if the tile index is out of bounds for the given map size.
    pub fn to_hex_coordinate(&self, map_parameters: &MapParameters) -> Hex {
        // We don't need to check if the index is valid here, as it has already been checked in `to_offset_coordinate`
        self.to_offset_coordinate(map_parameters)
            .to_hex(map_parameters.offset, map_parameters.hex_layout.orientation)
    }

    /// Calculates the latitude of the tile on the tile map.
    ///
    /// The latitude is defined such that:
    /// - The equator corresponds to a latitude of `0.0`.
    /// - The poles correspond to a latitude of `1.0`.
    ///
    /// As the latitude value approaches `0.0`, the tile is closer to the equator,
    /// while a value approaching `1.0` indicates proximity to the poles.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the size and dimensions of the map.
    ///
    /// # Returns
    /// A `f64` representing the latitude of the tile, with values ranging from `0.0` (equator) to `1.0` (poles).
    ///
    /// # Panics
    /// This method will panic if the tile index is out of bounds for the given map size.
    pub fn latitude(&self, map_parameters: &MapParameters) -> f64 {
        // We don't need to check if the index is valid here, as it has already been checked in `to_offset_coordinate`
        let y = self.to_offset_coordinate(map_parameters).0.y;
        let half_height = map_parameters.map_size.height as f64 / 2.0;
        ((half_height - y as f64) / half_height).abs()
    }

    /// Returns the terrain type of the tile at the given index.
    #[inline]
    pub fn terrain_type(&self, tile_map: &TileMap) -> TerrainType {
        tile_map.terrain_type_query[self.0]
    }

    /// Returns the base terrain of the tile at the given index.
    #[inline]
    pub fn base_terrain(&self, tile_map: &TileMap) -> BaseTerrain {
        tile_map.base_terrain_query[self.0]
    }

    /// Returns the feature of the tile at the given index.
    #[inline]
    pub fn feature(&self, tile_map: &TileMap) -> Option<Feature> {
        tile_map.feature_query[self.0]
    }

    /// Returns the natural wonder of the tile at the given index.
    #[inline]
    pub fn natural_wonder(&self, tile_map: &TileMap) -> Option<NaturalWonder> {
        tile_map.natural_wonder_query[self.0].clone()
    }

    /// Returns the area id of the tile at the given index.
    #[inline]
    pub fn area_id(&self, tile_map: &TileMap) -> i32 {
        tile_map.area_id_query[self.0]
    }

    pub fn neighbor_tile_indices<'a>(&'a self, map_parameters: &MapParameters) -> Vec<Self> {
        self.tile_indices_at_distance(1, map_parameters)
    }

    /// Retrieves the index of the neighboring tile from the current tile index in the specified direction.
    ///
    /// # Parameters
    /// - `direction`: The direction to locate the neighboring tile.
    /// - `map_parameters`: A reference to the map parameters that include layout and offset information.
    ///
    /// # Returns
    /// An `Option<TileIndex>`. This is `Some` if the neighboring tile exists,
    /// or `None` if the neighboring tile is invalid.
    ///
    /// # Panics
    /// This method will panic if the current tile index is out of bounds for the given map size.
    pub fn neighbor_tile_index<'a>(
        &'a self,
        direction: Direction,
        map_parameters: &MapParameters,
    ) -> Option<Self> {
        let orientation = map_parameters.hex_layout.orientation;
        // We don't need to check if the index is valid here, as it has already been checked in `to_hex_coordinate`
        let neighbor_offset_coordinate = self
            .to_hex_coordinate(map_parameters)
            .neighbor(orientation, direction)
            .to_offset_coordinate(map_parameters.offset, orientation);

        Self::from_offset_coordinate(map_parameters, neighbor_offset_coordinate).ok()
    }

    /// Get the indices of the tiles at the given distance from the current tile.
    pub fn tile_indices_at_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<Self> {
        // We don't need to check if the index is valid here, as it has already been checked in `to_hex_coordinate`
        let hex = self.to_hex_coordinate(map_parameters);
        hex.hexes_at_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                Self::from_offset_coordinate(map_parameters, offset_coordinate).ok()
            })
            .collect()
    }

    pub fn tile_indices_in_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<Self> {
        // We don't need to check if the index is valid here, as it has already been checked in `to_hex_coordinate`
        let hex = self.to_hex_coordinate(map_parameters);
        hex.hexes_in_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                Self::from_offset_coordinate(map_parameters, offset_coordinate).ok()
            })
            .collect()
    }

    pub fn pixel_position(&self, map_parameters: &MapParameters) -> DVec2 {
        // We donn't need to check if the tile index is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(map_parameters);
        map_parameters.hex_layout.hex_to_pixel(hex)
    }

    pub fn corner_position(&self, direction: Direction, map_parameters: &MapParameters) -> DVec2 {
        // We donn't need to check if the tile index is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(map_parameters);
        map_parameters.hex_layout.corner(hex, direction)
    }

    /// Checks if there is a river on the current tile in the specified direction.
    ///
    /// # Parameters
    /// - `direction`: The direction to check for the river.
    /// - `tile_map`: A reference to the TileMap containing river information.
    /// - `map_parameters`: A reference to the map parameters, which include hex layout settings.
    ///
    /// # Returns
    /// - `bool`: Returns true if there is a river in the specified direction, false otherwise.
    pub fn has_river(
        &self,
        direction: Direction,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        let edge_index = map_parameters.hex_layout.orientation.edge_index(direction);
        let check_tile_index;
        let check_edge_direction;
        if edge_index < 3 {
            check_tile_index = *self;
            check_edge_direction = direction;
        } else {
            if let Some(neighbor_tile_index) = self.neighbor_tile_index(direction, map_parameters) {
                check_tile_index = neighbor_tile_index;
                check_edge_direction = direction.opposite_direction();
            } else {
                return false;
            }
        }

        tile_map.river_list.values().flatten().any(
            |&(tile_index, flow_direction)| {
                tile_index == check_tile_index // 1. Check whether there is a river in the current tile
                    && check_edge_direction == edge_direction_for_flow_direction(flow_direction, map_parameters)
            })
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
        self.neighbor_tile_indices(map_parameters)
            .iter()
            .any(|&tile_index| {
                tile_index.base_terrain(tile_map).name() == terrain_name
                    || tile_index
                        .feature(tile_map)
                        .map_or(false, |feature| feature.name() == terrain_name)
            })
    }

    pub fn is_impassable(&self, tile_map: &TileMap, ruleset: &Ruleset) -> bool {
        self.terrain_type(tile_map) == TerrainType::Mountain
            || self
                .feature(tile_map)
                .map_or(false, |feature| feature.impassable(ruleset))
            || self
                .natural_wonder(tile_map)
                .map_or(false, |natural_wonder| natural_wonder.impassable(ruleset))
    }

    pub fn is_freshwater(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        let direction_array = map_parameters.edge_direction_array();
        let has_river = direction_array
            .iter()
            .any(|&direction| self.has_river(direction, tile_map, map_parameters));
        (self.terrain_type(tile_map) != TerrainType::Water)
            && (self.is_adjacent_to("Lake", tile_map, map_parameters)
                || self.is_adjacent_to("Oasis", tile_map, map_parameters)
                || has_river)
    }

    pub fn is_coastal_land(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        self.terrain_type(tile_map) != TerrainType::Water
            && self
                .neighbor_tile_indices(map_parameters)
                .iter()
                .any(|&tile_index| tile_index.terrain_type(tile_map) == TerrainType::Water)
    }
}

/// Returns the edge direction that corresponds to a given flow direction in a hexagonal grid,
/// based on the specified layout orientation.
///
/// This function maps flow directions to their respective edge directions within a hexagonal
/// layout, accounting for both pointy and flat orientations.
///
/// # Parameters
/// - `flow_direction`: The direction of the river flow.
/// - `map_parameters`: A reference to `MapParameters`, which contains the hexagonal layout orientation.
///
/// # Returns
/// The corresponding edge direction refers to the direction of the river edge located on the current tile.
/// For example, when hex is `HexOrientation::Pointy`, if the river is flowing North or South, the edge direction is East.
///
/// # Panics
/// This function will panic if an invalid flow direction is provided.
fn edge_direction_for_flow_direction(
    flow_direction: Direction,
    map_parameters: &MapParameters,
) -> Direction {
    match map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => match flow_direction {
            Direction::North | Direction::South => Direction::East,
            Direction::NorthEast | Direction::SouthWest => Direction::SouthEast,
            Direction::NorthWest | Direction::SouthEast => Direction::SouthWest,
            _ => panic!("Invalid flow direction"),
        },
        HexOrientation::Flat => match flow_direction {
            Direction::NorthWest | Direction::SouthEast => Direction::NorthEast,
            Direction::NorthEast | Direction::SouthWest => Direction::SouthEast,
            Direction::East | Direction::West => Direction::South,
            _ => panic!("Invalid flow direction"),
        },
    }
}
