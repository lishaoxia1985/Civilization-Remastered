use rand::{rngs::StdRng, seq::SliceRandom, Rng};

use crate::{
    grid::{hex::HexOrientation, Direction},
    map::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    tile_map::{MapParameters, Tile, TileMap},
};

impl TileMap {
    pub fn add_rivers(&mut self, map_parameters: &MapParameters) {
        let river_source_range_default = 4;
        let sea_water_range_default = 3;
        // tiles_per_river_edge specifies the number of tiles required before a river edge can appear.
        // When tiles_per_river_edge is set to 12, it indicates that for every 12 tiles, there can be 1 river edge.
        const tiles_per_river_edge: u32 = 12;

        // Mountain and Hill are the 1st priority for river starting locations.
        let pass_condition_1 = |tile: &Tile, _: &TileMap, _: &mut StdRng, _: &MapParameters| {
            tile.is_hill() || tile.is_mountain()
        };

        // Land tiles that are not near the ocean are the 2nd priority for river starting locations.
        let pass_condition_2 = |tile: &Tile,
                                tile_map: &TileMap,
                                random_number_generator: &mut StdRng,
                                map_parameters: &MapParameters| {
            !tile.is_water()
                && !tile.is_coastal_land(tile_map, map_parameters)
                && random_number_generator.gen_range(0..8) == 0
        };

        // If there are still not enough rivers generated, the algorithm should run again using Mountain and Hill as the river starting locations.
        let pass_condition_3 =
            |tile: &Tile, tile_map: &TileMap, _: &mut StdRng, _: &MapParameters| {
                let num_tiles = tile_map
                    .area_id_query
                    .iter()
                    .filter(|area_id| **area_id == tile.area_id)
                    .count() as u32;
                let num_river_edges = tile_map.river_edge_count(tile.area_id);
                (tile.is_hill() || tile.is_mountain())
                    && (num_river_edges <= num_tiles / tiles_per_river_edge)
            };

        // At last if there are still not enough rivers generated, the algorithm should run again using any Land tiles as the river starting locations.
        let pass_condition_4 =
            |tile: &Tile, tile_map: &TileMap, _: &mut StdRng, _: &MapParameters| {
                let num_tiles = tile_map
                    .area_id_query
                    .iter()
                    .filter(|area_id| **area_id == tile.area_id)
                    .count() as u32;
                let num_river_edges = tile_map.river_edge_count(tile.area_id);
                !tile.is_water() && (num_river_edges <= num_tiles / tiles_per_river_edge)
            };

        let pass_conditions = [
            pass_condition_1,
            pass_condition_2,
            pass_condition_3,
            pass_condition_4,
        ];

        let mut random_number_generator = self.random_number_generator.clone();

        let mut river_id = 0;

        pass_conditions
            .iter()
            .enumerate()
            .for_each(|(index, pass_condition)| {
                let (river_source_range, sea_water_range) = if index <= 1 {
                    (river_source_range_default, sea_water_range_default)
                } else {
                    (
                        (river_source_range_default / 2),
                        (sea_water_range_default / 2),
                    )
                };

                (0..self.tile_count()).for_each(|tile_index| {
                    let tile = self.tile(tile_index);
                    // Tile should meet these conditions:
                    // 1. It should meet the pass condition
                    // 2. It should be not a natural wonder
                    // 3. It should not be adjacent to a natural wonder
                    // 4. all tiles around it in a given distance `river_source_range` (including self) should be not fresh water
                    // 5. all tiles around it in a given distance `sea_water_range` (including self) should be not water

                    if pass_condition(&tile, self, &mut random_number_generator, map_parameters)
                        && tile.natural_wonder.is_none()
                        && !tile
                            .tile_neighbors(map_parameters)
                            .iter()
                            .any(|&neighbor_index| {
                                self.natural_wonder_query[neighbor_index].is_some()
                            })
                        && !tile
                            .tiles_in_distance(river_source_range, map_parameters)
                            .iter()
                            .any(|tile_index| {
                                let tile = self.tile(*tile_index);
                                tile.is_freshwater(self, map_parameters)
                            })
                        && !tile
                            .tiles_in_distance(sea_water_range, map_parameters)
                            .iter()
                            .any(|tile_index| {
                                let tile = self.tile(*tile_index);
                                tile.is_water()
                            })
                    {
                        let start_tile_index = self.get_inland_corner(tile_index, map_parameters);
                        if let Some(start_tile_index) = start_tile_index {
                            self.do_river(
                                start_tile_index,
                                Direction::None,
                                Direction::None,
                                river_id,
                                &map_parameters,
                            );
                            river_id += 1;
                        }
                    }
                });
            });
        self.random_number_generator = random_number_generator;
    }

    /// This function is called to create a river.
    ///
    /// # Notice
    /// In original Civ V, the end of the river is water or the edge of the map.
    /// In this function, we have not implemented that the river flows the edge of the map yet.
    /// That because when we implement it, we should concern the map parameters.
    /// For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
    /// In original Civ V, we only need to consider the case where the map is WrapX and the hex is pointy.
    fn do_river(
        &mut self,
        start_tile_index: usize,
        this_flow_direction: Direction,
        original_flow_direction: Direction,
        river_id: i32,
        map_parameters: &MapParameters,
    ) {
        // This array contains the list of tuples.
        // In this tuple, the elemment means as follows:
        // 1. The first element indicates the next possible flow direction of the river.
        // 2. The second element represents the direction of a neighboring tile relative to the current tile.
        //    We evaluate the weight value of these neighboring tiles using a certain algorithm and select the minimum one to determine the next flow direction of the river.
        let flow_direction_and_neighbor_tile_direction = match map_parameters.hex_layout.orientation
        {
            HexOrientation::Pointy => [
                (Direction::North, Direction::NorthWest),
                (Direction::NorthEast, Direction::NorthEast),
                (Direction::SouthEast, Direction::East),
                (Direction::South, Direction::SouthWest),
                (Direction::SouthWest, Direction::West),
                (Direction::NorthWest, Direction::NorthWest),
            ],
            HexOrientation::Flat => [
                (Direction::East, Direction::NorthEast),
                (Direction::SouthEast, Direction::South),
                (Direction::SouthWest, Direction::SouthWest),
                (Direction::West, Direction::NorthWest),
                (Direction::NorthWest, Direction::NorthWest),
                (Direction::NorthEast, Direction::North),
            ],
        };

        /************ Do river start ************/

        // If the start plot have a river, exit the function
        // That will also prevent the river from forming a loop
        if self
            .river_list
            .values()
            .flatten()
            .any(|&(tile_index, _)| tile_index == start_tile_index)
        {
            return;
        }

        let mut start_tile_index = start_tile_index;
        let mut this_flow_direction = this_flow_direction;
        let mut original_flow_direction = original_flow_direction;

        loop {
            let mut river_tile_index;
            let mut best_flow_direction = Direction::None;
            match map_parameters.hex_layout.orientation {
                HexOrientation::Pointy => match this_flow_direction {
                    Direction::East | Direction::West => unreachable!(),
                    Direction::North => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::NorthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || neighbor_tile.has_river(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river(
                                    Direction::SouthWest,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile_index = neighbor_tile.index(map_parameters);
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthEast => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::East, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::East, self, map_parameters)
                                || neighbor_tile.has_river(
                                    Direction::SouthWest,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthEast => {
                        let start_tile = self.tile(start_tile_index);
                        if let Some(neighbor_index) =
                            start_tile.tile_neighbor(Direction::East, map_parameters)
                        {
                            river_tile_index = neighbor_index
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::SouthEast, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_index2) =
                            river_tile.tile_neighbor(Direction::SouthWest, map_parameters)
                        {
                            let neighbor_tile2 = self.tile(neighbor_index2);
                            if neighbor_tile2.is_water()
                                || neighbor_tile2.has_river(Direction::East, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::South => {
                        let start_tile = self.tile(start_tile_index);
                        if let Some(neighbor_index) =
                            start_tile.tile_neighbor(Direction::SouthWest, map_parameters)
                        {
                            river_tile_index = neighbor_index
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::SouthEast, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_index2) =
                            river_tile.tile_neighbor(Direction::East, map_parameters)
                        {
                            let neighbor_tile2 = self.tile(neighbor_index2);
                            if neighbor_tile2.has_river(Direction::SouthWest, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthWest => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthWest, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || neighbor_tile.has_river(Direction::East, self, map_parameters)
                                || river_tile.has_river(Direction::SouthWest, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthWest => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::West, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || neighbor_tile.has_river(Direction::East, self, map_parameters)
                                || neighbor_tile.has_river(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile_index = neighbor_tile.index(map_parameters);
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::None => {
                        river_tile_index = start_tile_index;
                    }
                },
                HexOrientation::Flat => match this_flow_direction {
                    Direction::North | Direction::South => unreachable!(),
                    Direction::NorthEast => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::NorthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::NorthEast, self, map_parameters)
                                || neighbor_tile.has_river(Direction::South, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::East => {
                        let start_tile = self.tile(start_tile_index);
                        if let Some(neighbor_index) =
                            start_tile.tile_neighbor(Direction::NorthEast, map_parameters)
                        {
                            river_tile_index = neighbor_index
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::SouthEast, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_index2) =
                            river_tile.tile_neighbor(Direction::South, map_parameters)
                        {
                            let neighbor_tile2 = self.tile(neighbor_index2);
                            if neighbor_tile2.is_water()
                                || neighbor_tile2.has_river(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthEast => {
                        let start_tile = self.tile(start_tile_index);
                        if let Some(neighbor_index) =
                            start_tile.tile_neighbor(Direction::South, map_parameters)
                        {
                            river_tile_index = neighbor_index
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthEast, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::SouthEast, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_index2) =
                            river_tile.tile_neighbor(Direction::NorthEast, map_parameters)
                        {
                            let neighbor_tile2 = self.tile(neighbor_index2);
                            if neighbor_tile2.is_water()
                                || neighbor_tile2.has_river(Direction::South, self, map_parameters)
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthWest => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::South, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || river_tile.has_river(Direction::South, self, map_parameters)
                                || neighbor_tile.has_river(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::West => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::SouthWest, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || neighbor_tile.has_river(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile_index = neighbor_tile.index(map_parameters);
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthWest => {
                        river_tile_index = start_tile_index;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile_index, this_flow_direction));
                        let river_tile = self.tile(river_tile_index);
                        if let Some(neighbor_index) =
                            river_tile.tile_neighbor(Direction::North, map_parameters)
                        {
                            let neighbor_tile = self.tile(neighbor_index);
                            if neighbor_tile.is_water()
                                || neighbor_tile.has_river(Direction::South, self, map_parameters)
                                || neighbor_tile.has_river(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile_index = neighbor_tile.index(map_parameters);
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::None => {
                        river_tile_index = start_tile_index;
                    }
                },
            }

            let river_tile = self.tile(river_tile_index);
            if river_tile.is_water() {
                return;
            }

            fn river_value_at_tile(
                tile_index: usize,
                tile_map: &mut TileMap,
                map_parameters: &MapParameters,
            ) -> i32 {
                fn tile_elevation(tile: &Tile) -> i32 {
                    match tile.terrain_type {
                        TerrainType::Mountain => 4,
                        TerrainType::Hill => 3,
                        TerrainType::Water => 2,
                        TerrainType::Flatland => 1,
                    }
                }

                let tile = tile_map.tile(tile_index);

                if tile.natural_wonder.is_some()
                    || tile
                        .tile_neighbors(map_parameters)
                        .iter()
                        .any(|&neighbor_index| {
                            let neighbor_tile = tile_map.tile(neighbor_index);
                            neighbor_tile.natural_wonder.is_some()
                        })
                {
                    return -1;
                }

                let mut sum = tile_elevation(&tile) * 20;
                let direction_array = map_parameters.edge_direction_array();
                direction_array.iter().for_each(|&direction| {
                    if let Some(adjacent_index) = tile.tile_neighbor(direction, map_parameters) {
                        let adjacent_tile = tile_map.tile(adjacent_index);
                        sum += tile_elevation(&adjacent_tile);
                        if adjacent_tile.base_terrain == BaseTerrain::Desert {
                            sum += 4;
                        }
                    } else {
                        sum += 40;
                    }
                });
                sum += tile_map.random_number_generator.gen_range(0..10);
                sum
            }

            // This vec contains the list of tuples.
            // In this tuple, the elemment means as follows:
            // 1. The first element indicates the next possible flow direction of the river.
            // 2. The second element represents the index of the tile neighboring to the current tile.
            //    We evaluate the weight value of these neighboring tiles using a certain algorithm and select the minimum one to determine the next flow direction of the river.
            //    The neighbor should meet the following conditions:
            //    1. The next flow direction is not the opposite of the original flow direction.
            //    2. The next flow direction is None (when this_flow_direction is `Direction::None`) or one of the possible flow directions of the current tile.
            let flow_direction_and_neighbor_tile_index = flow_direction_and_neighbor_tile_direction
                .into_iter()
                .filter_map(|(flow_direction, direction)| {
                    if flow_direction.opposite_direction() != original_flow_direction // The next flow direction is not the opposite of the original flow direction.
                            && (this_flow_direction == Direction::None
                                || next_flow_directions(this_flow_direction, map_parameters)
                                    .contains(&flow_direction))
                    {
                        river_tile
                            .tile_neighbor(direction, map_parameters)
                            .map(|neighbor_index| (flow_direction, neighbor_index))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if best_flow_direction == Direction::None {
                let mut best_value = i32::MAX;
                flow_direction_and_neighbor_tile_index.into_iter().for_each(
                    |(flow_direction, neighbor_tile_index)| {
                        let mut value =
                            river_value_at_tile(neighbor_tile_index, self, map_parameters);
                        if flow_direction == original_flow_direction {
                            value = (value * 3) / 4;
                        }
                        if value < best_value {
                            best_value = value;
                            best_flow_direction = flow_direction;
                        }
                    },
                );
                /* Tackle with the situation when river flows to the edge of map */

                /* This code handles the situation when the river flows to the edge of the map,
                but we have not implemented this part yet, so we will ignore it here.
                When we implement it, we should concern the map parameters.
                For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
                */

                /* End tackle with the situation when river flows to the edge of map */
            }

            if best_flow_direction != Direction::None {
                if original_flow_direction == Direction::None {
                    original_flow_direction = best_flow_direction;
                }
                start_tile_index = river_tile_index;
                this_flow_direction = best_flow_direction;
            } else {
                return;
            }
        }
        /************ Do river End ************/
    }

    /// Finds an *inland* corner of this plot at which to place a river.
    fn get_inland_corner(
        &mut self,
        tile_index: usize,
        map_parameters: &MapParameters,
    ) -> Option<usize> {
        let mut plot_list = vec![tile_index];

        let corner_candidate_direction = &map_parameters.edge_direction_array()[3..6];

        corner_candidate_direction
            .iter()
            .filter_map(|&direction| {
                let tile = self.tile(tile_index);
                tile.tile_neighbor(direction, map_parameters)
            })
            .for_each(|neighbor_index| {
                plot_list.push(neighbor_index);
            });

        // Remove plots that are not corners
        // A corner is a plot whose `map_parameters.edge_direction_array()[0..3]` neighbor exists and is not water
        plot_list.retain(|&tile_index| {
            let tile = self.tile(tile_index);
            map_parameters.edge_direction_array()[0..3]
                .iter()
                .all(|&direction| {
                    let neighbor_index = tile.tile_neighbor(direction, map_parameters);
                    if let Some(neighbor_index) = neighbor_index {
                        let neighbor_tile = self.tile(neighbor_index);
                        !neighbor_tile.is_water()
                    } else {
                        false
                    }
                })
        });

        // Choose a random corner
        plot_list.choose(&mut self.random_number_generator).copied()
    }

    /// Returns the number of river edges in the current area according to `area_id`
    fn river_edge_count(&self, current_area_id: i32) -> u32 {
        self.river_list
            .values()
            .flatten()
            .filter(|(tile_index, _)| {
                let area_id = self.area_id_query[*tile_index];
                area_id == current_area_id
            })
            .count() as u32
    }
}

/// Returns the next possible flow directions of the river according to the current flow direction.
const fn next_flow_directions(
    flow_direction: Direction,
    map_parameters: &MapParameters,
) -> [Direction; 2] {
    let hex_orientation = map_parameters.hex_layout.orientation;
    [
        hex_orientation.corner_clockwise(flow_direction), // turn_right_flow_direction
        hex_orientation.corner_counter_clockwise(flow_direction), // turn_left_flow_direction
    ]
}
