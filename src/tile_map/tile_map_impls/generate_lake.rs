use bevy::utils::HashSet;
use rand::Rng;

use crate::{
    map::{base_terrain::BaseTerrain, terrain_type::TerrainType, AreaIdAndSize},
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    /// This function generates lakes on the map.
    ///
    /// This fun is used because when we create the world map by System `spawn_tile_type`, some water areas will be created surrounded by land.
    /// If these water areas are small enough, they will be considered as lakes and will be replaced by the `TerrainType::Lake` terrain.
    pub fn generate_lake(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &AreaIdAndSize,
    ) {
        (0..self.tile_count()).into_iter().for_each(|tile_index| {
            let tile = self.tile(tile_index);
            if tile.terrain_type == TerrainType::Water
                && area_id_and_size.0[&tile.area_id] <= map_parameters.lake_max_area_size
            {
                self.base_terrain_query[tile_index] = BaseTerrain::Lake;
            }
        });
    }

    pub fn add_lakes(&mut self, map_parameters: &MapParameters) {
        let large_lake_num = map_parameters.large_lake_num;

        let mut num_large_lakes_added = 0;
        let lake_plot_rand = 25;

        (0..self.tile_count()).into_iter().for_each(|tile_index| {
            if self.can_add_lake(tile_index, &map_parameters)
                && self.random_number_generator.gen_range(0..lake_plot_rand) == 0
            {
                if num_large_lakes_added < large_lake_num {
                    let add_more_lakes = self.add_more_lake(tile_index, &map_parameters);

                    if add_more_lakes {
                        num_large_lakes_added += 1;
                    }
                }
                self.terrain_type_query[tile_index] = TerrainType::Water;
                self.base_terrain_query[tile_index] = BaseTerrain::Lake;
                self.feature_query[tile_index] = None;
            }
        });
    }

    fn add_more_lake(&mut self, tile_index: usize, map_parameters: &MapParameters) -> bool {
        let mut large_lake = 0;

        let edge_direction_array = map_parameters.edge_direction_array();

        let tile = self.tile(tile_index);

        edge_direction_array.into_iter().for_each(|direction| {
            let neighbor_index = tile.tile_neighbor(direction, map_parameters);
            if let Some(neighbor_index) = neighbor_index {
                if self.can_add_lake(neighbor_index, map_parameters)
                    && self.random_number_generator.gen_range(0..(large_lake + 4)) < 3
                {
                    self.terrain_type_query[neighbor_index] = TerrainType::Water;
                    self.base_terrain_query[neighbor_index] = BaseTerrain::Lake;
                    self.feature_query[neighbor_index] = None;
                    large_lake += 1;
                }
            }
        });

        large_lake > 2
    }

    /// This function checks if a tile can have a lake.
    ///
    /// The tile that can have a lake should meet these conditions:
    /// 1. The tile is not water
    /// 2. The tile is not a natural wonder
    /// 3. The tile is not adjacent to a river
    /// 4. The tile is not adjacent to water
    /// 5. The tile is not adjacent to a natural wonder
    fn can_add_lake(&self, tile_index: usize, map_parameters: &MapParameters) -> bool {
        let edge_direction_array = map_parameters.edge_direction_array();

        let tile = self.tile(tile_index);

        // Check if the current tile is suitable for a lake
        if tile.terrain_type == TerrainType::Water
            || tile.natural_wonder.is_some()
            || edge_direction_array
                .iter()
                .any(|&direction| tile.has_river(direction, self, map_parameters))
        {
            return false;
        }

        let neighbor_indices = tile.tile_neighbors(&map_parameters);

        // Check if all neighbor tiles are also suitable
        neighbor_indices.iter().all(|&neighbor_index| {
            let tile = self.tile(neighbor_index);
            tile.terrain_type != TerrainType::Water && tile.natural_wonder.is_none()
        })
    }
}
