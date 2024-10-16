use bevy::utils::{HashMap, HashSet};
use rand::prelude::SliceRandom;
use rand::Rng;

use crate::{
    grid::hex::Hex,
    map::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        terrain_type::TerrainType,
    },
    ruleset::{Ruleset, Unique},
    tile_map::{tile_index::TileIndex, MapParameters, TileMap},
};

impl TileMap {
    pub fn generate_natural_wonder(&mut self, ruleset: &Ruleset, map_parameters: &MapParameters) {
        let natural_wonder_list: Vec<_> = ruleset.natural_wonders.keys().collect();

        let mut natural_wonder_and_tile_index_and_score = HashMap::new();

        // Find all land areas and size
        let land_area_id_and_size: HashSet<_> = self
            .iter_tile_indices()
            .filter(|&tile_index| tile_index.terrain_type(self) != TerrainType::Water)
            .map(|tile_index| {
                let area_id = tile_index.area_id(self);
                (area_id, self.area_id_and_size[&area_id])
            })
            .collect();

        let mut land_area_id_and_size: Vec<_> = land_area_id_and_size.into_iter().collect();

        // First, sort by area_size in descending order using std::cmp::Reverse
        // If area_size is the same, sort by land_id in ascending order
        land_area_id_and_size
            .sort_unstable_by_key(|&(land_id, area_size)| (std::cmp::Reverse(area_size), land_id));

        for tile_index in self.iter_tile_indices() {
            for &natural_wonder_name in &natural_wonder_list {
                let possible_natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                match natural_wonder_name.as_str() {
                    "Great Barrier Reef" => {
                        if let Some(neighbor_tile_index) = tile_index.neighbor_tile_index(
                            map_parameters.edge_direction_array()[1],
                            &map_parameters,
                        ) {
                            let mut all_neigbor_tile_indices = HashSet::new();

                            all_neigbor_tile_indices
                                .extend(tile_index.neighbor_tile_indices(&map_parameters));
                            all_neigbor_tile_indices
                                .extend(neighbor_tile_index.neighbor_tile_indices(&map_parameters));

                            all_neigbor_tile_indices.remove(&tile_index);
                            all_neigbor_tile_indices.remove(&neighbor_tile_index);

                            // The tile should meet the following conditions:
                            // 1. All neighboring tiles exist
                            // 2. All neighboring tiles are water and not lake, not ice
                            // 3. At least 4 neighboring tiles are coast
                            if all_neigbor_tile_indices.len() == 8
                                && all_neigbor_tile_indices.iter().all(|&tile_index| {
                                    tile_index.terrain_type(self) == TerrainType::Water
                                        && tile_index.base_terrain(self) != BaseTerrain::Lake
                                        && tile_index.feature(self) != Some(Feature::Ice)
                                })
                                && all_neigbor_tile_indices
                                    .iter()
                                    .filter(|tile_index| {
                                        tile_index.base_terrain(self) == BaseTerrain::Coast
                                    })
                                    .count()
                                    >= 4
                            {
                                natural_wonder_and_tile_index_and_score
                                    .entry(natural_wonder_name)
                                    .or_insert_with(Vec::new)
                                    .push((tile_index, 1));
                            }
                        }
                    }
                    _ => {
                        if tile_index.is_freshwater(self, &map_parameters)
                            != possible_natural_wonder.is_fresh_water
                        {
                            continue;
                        };

                        if !possible_natural_wonder
                            .occurs_on_type
                            .contains(&tile_index.terrain_type(self))
                            || !possible_natural_wonder
                                .occurs_on_base
                                .contains(&tile_index.base_terrain(self))
                        {
                            continue;
                        }

                        let check_unique_conditions =
                            possible_natural_wonder.uniques.iter().all(|unique| {
                                let unique = Unique::new(unique);
                                match unique.placeholder_text.as_str() {
                                    "Must be adjacent to [] [] tiles" => {
                                        let count = tile_index
                                            .neighbor_tile_indices(&map_parameters)
                                            .iter()
                                            .filter(|tile_index| {
                                                self.matches_wonder_filter(
                                                    **tile_index,
                                                    unique.params[1].as_str(),
                                                )
                                            })
                                            .count();
                                        count == unique.params[0].parse::<usize>().unwrap()
                                    }
                                    "Must be adjacent to [] to [] [] tiles" => {
                                        let count = tile_index
                                            .neighbor_tile_indices(&map_parameters)
                                            .iter()
                                            .filter(|tile_index| {
                                                self.matches_wonder_filter(
                                                    **tile_index,
                                                    unique.params[2].as_str(),
                                                )
                                            })
                                            .count();
                                        count >= unique.params[0].parse::<usize>().unwrap()
                                            && count <= unique.params[1].parse::<usize>().unwrap()
                                    }
                                    "Must not be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile isn't on the landmass with the given index
                                        !land_area_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| {
                                                id == tile_index.area_id(self)
                                            })
                                    }
                                    "Must be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile is on the landmass with the given index
                                        land_area_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| {
                                                id == tile_index.area_id(self)
                                            })
                                    }
                                    _ => true,
                                }
                            });
                        // end check unique conditions

                        if check_unique_conditions {
                            natural_wonder_and_tile_index_and_score
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push((tile_index, 1));
                        }
                    }
                }
            }
        }

        // Get the natural wonders that can be placed
        let mut selected_natural_wonder_list: Vec<_> = natural_wonder_and_tile_index_and_score
            .keys()
            .cloned()
            .collect();
        /* The order of selected_natural_wonder_list is random, so we should arrange this list in order
        to ensure that the obtained Vec is the same every time. */
        selected_natural_wonder_list.sort_unstable();
        // Shuffle the list that we can choose natural wonder randomly
        selected_natural_wonder_list.shuffle(&mut self.random_number_generator);

        // Store current how many natural wonders have been placed
        let mut j = 0;
        // Store the index of the tile where the natural wonder has been placed
        let mut placed_natural_wonder_tile_indices: Vec<TileIndex> = Vec::new();

        // start to place wonder
        selected_natural_wonder_list
            .into_iter()
            .for_each(|natural_wonder_name| {
                if j <= map_parameters.natural_wonder_num {
                    // For every natural wonder, give a score to the position where the natural wonder can place.
                    // The score is related to the min value of the distance from the position to all the placed natural wonders
                    // If no natural wonder has placed, we choose the random place where the current natural wonder can place for the current natural wonder

                    // the score method start
                    let tile_index_and_score = natural_wonder_and_tile_index_and_score
                        .get_mut(natural_wonder_name)
                        .unwrap();
                    for (tile_x_index, score) in tile_index_and_score.iter_mut() {
                        let closest_natural_wonder_dist = placed_natural_wonder_tile_indices
                            .iter()
                            .map(|tile_y_index| {
                                let position_x_hex = tile_x_index.to_hex_coordinate(map_parameters);
                                let position_y_hex = tile_y_index.to_hex_coordinate(map_parameters);
                                Hex::hex_distance(
                                    Hex::from(position_x_hex),
                                    Hex::from(position_y_hex),
                                )
                            })
                            .min()
                            .unwrap_or(1000000);
                        *score = if closest_natural_wonder_dist <= 10 {
                            100 * closest_natural_wonder_dist
                        } else {
                            1000 + (closest_natural_wonder_dist - 10)
                        } + self.random_number_generator.gen_range(0..100);
                    }
                    // the score method end

                    // choose the max score position as the candidate position for the current natural wonder
                    let max_score_tile_index = tile_index_and_score
                        .iter()
                        .max_by_key(|&(_, score)| score)
                        .map(|&(index, _)| index)
                        .unwrap();

                    if !placed_natural_wonder_tile_indices.contains(&max_score_tile_index) {
                        let natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                        // At first, we should remove feature from the tile
                        self.feature_query[*max_score_tile_index] = None;

                        match natural_wonder_name.as_str() {
                            "Great Barrier Reef" => {
                                // The neighbor tile absolutely exists because we have checked it before. So we can unwrap it.
                                let neighbor_tile_index = max_score_tile_index
                                    .neighbor_tile_index(
                                        map_parameters.edge_direction_array()[1],
                                        &map_parameters,
                                    )
                                    .unwrap();

                                // Get the indices of the neighbor tiles of the max score tile
                                let max_score_tile_neighbor_indices: Vec<_> =
                                    max_score_tile_index.neighbor_tile_indices(&map_parameters);

                                // Get the indices of the neighbor tiles of 'the neighbor tile of the max score tile'
                                let neighbor_tile_neighbor_indices: Vec<_> =
                                    neighbor_tile_index.neighbor_tile_indices(&map_parameters);

                                max_score_tile_neighbor_indices.into_iter().for_each(
                                    |tile_index| {
                                        self.terrain_type_query[*tile_index] = TerrainType::Water;
                                        self.base_terrain_query[*tile_index] = BaseTerrain::Coast;
                                    },
                                );
                                neighbor_tile_neighbor_indices
                                    .into_iter()
                                    .for_each(|tile_index| {
                                        self.terrain_type_query[*tile_index] = TerrainType::Water;
                                        self.base_terrain_query[*tile_index] = BaseTerrain::Coast;
                                    });
                                // place the natural wonder on the candidate position and its adjacent tile
                                self.natural_wonder_query[*max_score_tile_index] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                self.natural_wonder_query[*neighbor_tile_index] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tile_indices.push(max_score_tile_index);
                                placed_natural_wonder_tile_indices.push(neighbor_tile_index);
                            }
                            "Rock of Gibraltar" => {
                                let neighbor_indices: Vec<_> =
                                    max_score_tile_index.neighbor_tile_indices(&map_parameters);

                                neighbor_indices
                                    .into_iter()
                                    .for_each(|neighbor_tile_index| {
                                        if neighbor_tile_index.terrain_type(self)
                                            == TerrainType::Water
                                        {
                                            self.base_terrain_query[*neighbor_tile_index] =
                                                BaseTerrain::Coast;
                                        } else {
                                            self.terrain_type_query[*neighbor_tile_index] =
                                                TerrainType::Mountain;
                                        }
                                    });
                                // Edit the choice tile's terrain_type to match the natural wonder
                                self.terrain_type_query[*max_score_tile_index] =
                                    TerrainType::Flatland;
                                // Edit the choice tile's base_terrain to match the natural wonder
                                self.base_terrain_query[*max_score_tile_index] =
                                    BaseTerrain::Grassland;
                                // place the natural wonder on the candidate position
                                self.natural_wonder_query[*max_score_tile_index] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tile_indices.push(max_score_tile_index);
                            }
                            _ => {
                                // Edit the choice tile's terrain_type to match the natural wonder
                                if let Some(turn_into_terrain_type) = natural_wonder.turns_into_type
                                {
                                    self.terrain_type_query[*max_score_tile_index] =
                                        turn_into_terrain_type;
                                };
                                // Edit the choice tile's base_terrain to match the natural wonder
                                if let Some(turn_into_base_terrain) = natural_wonder.turns_into_base
                                {
                                    self.base_terrain_query[*max_score_tile_index] =
                                        turn_into_base_terrain;
                                }
                                // place the natural wonder on the candidate position
                                self.natural_wonder_query[*max_score_tile_index] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tile_indices.push(max_score_tile_index);
                            }
                        }
                        j += 1;
                    }
                }
            });

        // If the natural wonder is not water, and its neighbors have water tile,
        // then change the water neighbor tiles to lake or coast.
        placed_natural_wonder_tile_indices
            .iter()
            .for_each(|&tile_index| {
                if tile_index.terrain_type(self) != TerrainType::Water {
                    let neighbor_tile_indices: Vec<_> =
                        tile_index.neighbor_tile_indices(&map_parameters);

                    neighbor_tile_indices
                        .iter()
                        .for_each(|&neighbor_tile_index| {
                            if neighbor_tile_index.terrain_type(self) == TerrainType::Water {
                                let neighbor_neighbor_tile_indices =
                                    neighbor_tile_index.neighbor_tile_indices(&map_parameters);

                                if neighbor_neighbor_tile_indices.iter().any(
                                    |&neighbor_neighbor_tile_index| {
                                        neighbor_neighbor_tile_index.base_terrain(self)
                                            == BaseTerrain::Lake
                                    },
                                ) {
                                    self.base_terrain_query[*neighbor_tile_index] =
                                        BaseTerrain::Lake;
                                } else {
                                    self.base_terrain_query[*neighbor_tile_index] =
                                        BaseTerrain::Coast;
                                };
                            };
                        });
                }
            });
    }

    fn matches_wonder_filter(&self, tile_index: TileIndex, filter: &str) -> bool {
        let terrain_type = tile_index.terrain_type(self);
        let base_terrain = tile_index.base_terrain(self);
        let feature = tile_index.feature(self);

        match filter {
            "Elevated" => matches!(terrain_type, TerrainType::Mountain | TerrainType::Hill),
            "Land" => terrain_type != TerrainType::Water,
            _ => {
                terrain_type.name() == filter
                    || base_terrain.name() == filter
                    || feature.map_or(false, |f| f.name() == filter)
            }
        }
    }
}
