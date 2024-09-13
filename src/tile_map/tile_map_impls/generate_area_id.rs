use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::utils::hashbrown::HashSet;

use crate::{
    map::{terrain_type::TerrainType, AreaIdAndSize},
    tile_map::{MapParameters, Tile, TileMap},
};

impl TileMap {
    pub fn bfs(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &mut AreaIdAndSize,
        filter_condition: impl Fn(&Tile) -> bool,
    ) {
        let mut area_tile_indices = (0..self.tile_count())
            .into_iter()
            .filter(|&tile_index| {
                let tile = self.tile(tile_index);
                filter_condition(&tile)
            })
            .map(|tile_index| tile_index)
            .collect::<HashSet<_>>();

        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&initial_area_tile_index) = area_tile_indices.iter().next() {
            self.area_id_query[initial_area_tile_index] = current_area_id;
            area_tile_indices.remove(&initial_area_tile_index);

            // Store all the entities in the current area.
            let mut tile_indices_in_current_area = HashSet::new();
            tile_indices_in_current_area.insert(initial_area_tile_index);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tile_indices_to_check = VecDeque::new();
            tile_indices_to_check.push_back(initial_area_tile_index);

            while let Some(tile_index_we_are_checking) = tile_indices_to_check.pop_front() {
                let tile_we_are_checking = self.tile(tile_index_we_are_checking);
                tile_we_are_checking
                    .tile_neighbors(map_parameters)
                    .iter()
                    .for_each(|&tile_index| {
                        if !tile_indices_in_current_area.contains(&tile_index)
                            && area_tile_indices.contains(&tile_index)
                        {
                            tile_indices_in_current_area.insert(tile_index);
                            self.area_id_query[tile_index] = current_area_id;
                            tile_indices_to_check.push_back(tile_index);
                            area_tile_indices.remove(&tile_index);
                        }
                    });
            }
            area_id_and_size
                .0
                .insert(current_area_id, tile_indices_in_current_area.len() as u32);
            current_area_id += 1;
        }
    }

    pub fn dfs(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &mut AreaIdAndSize,
        filter_condition: impl Fn(&Tile) -> bool,
    ) {
        let mut area_tile_indices = (0..self.tile_count())
            .into_iter()
            .filter(|&tile_index| {
                let tile = self.tile(tile_index);
                filter_condition(&tile)
            })
            .map(|tile_index| tile_index)
            .collect::<HashSet<_>>();

        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&initial_area_tile_index) = area_tile_indices.iter().next() {
            self.area_id_query[initial_area_tile_index] = current_area_id;
            area_tile_indices.remove(&initial_area_tile_index);

            // Store all the entities in the current area.
            let mut tile_indices_in_current_area = HashSet::new();
            tile_indices_in_current_area.insert(initial_area_tile_index);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tile_indices_to_check = Vec::new();
            tile_indices_to_check.push(initial_area_tile_index);

            while let Some(tile_index_we_are_checking) = tile_indices_to_check.pop() {
                let tile_we_are_checking = self.tile(tile_index_we_are_checking);
                tile_we_are_checking
                    .tile_neighbors(map_parameters)
                    .iter()
                    .for_each(|&tile_index| {
                        if !tile_indices_in_current_area.contains(&tile_index)
                            && area_tile_indices.contains(&tile_index)
                        {
                            tile_indices_in_current_area.insert(tile_index);
                            self.area_id_query[tile_index] = current_area_id;
                            tile_indices_to_check.push(tile_index);
                            area_tile_indices.remove(&tile_index);
                        }
                    });
            }
            area_id_and_size
                .0
                .insert(current_area_id, tile_indices_in_current_area.len() as u32);
            current_area_id += 1;
        }
    }

    pub fn recalculate_areas(
        &mut self,
        map_parameters: &MapParameters,
        mut area_id_and_size: &mut AreaIdAndSize,
    ) {
        area_id_and_size.0.clear();

        self.area_id_query = vec![-1; self.tile_count()];

        let water_condition = |tile: &Tile| tile.terrain_type == TerrainType::Water;

        let hill_and_flatland_condition = |tile: &Tile| {
            tile.terrain_type == TerrainType::Flatland || tile.terrain_type == TerrainType::Hill
        };

        let mountain_condition = |tile: &Tile| tile.terrain_type == TerrainType::Mountain;

        let conditions = vec![
            water_condition,
            hill_and_flatland_condition,
            mountain_condition,
        ];

        conditions.iter().for_each(|condition| {
            self.bfs(&map_parameters, &mut area_id_and_size, condition);
        });
    }

    pub fn reassign_area_id(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &mut AreaIdAndSize,
    ) {
        const MIN_AREA_SIZE: u32 = 7;

        // Get id of the smaller area whose size < MIN_AREA_SIZE
        let small_area_id: Vec<_> = area_id_and_size
            .0
            .iter()
            .filter(|(_, size)| **size < MIN_AREA_SIZE)
            .map(|(&id, _)| id)
            .collect();

        small_area_id.into_iter().for_each(|current_area_id| {
            let tile_indices_current_area = self
                .area_id_query
                .iter()
                .enumerate()
                .filter(|(_, area_id)| **area_id == current_area_id)
                .map(|(tile_index, _)| tile_index)
                .collect::<Vec<_>>();

            // Check if the current area is water
            let area_is_water =
                self.terrain_type_query[tile_indices_current_area[0]] == TerrainType::Water;

            // Get the border entities of the current area, these entities don't belong to the area, but they surround the area.
            // Using BTreeSet to store the border entities will make sure the entities are processed in the same order every time.
            // That means that we can get the same 'surround_area_size_and_id' every time.
            let mut border_tile_indices = BTreeSet::new();

            tile_indices_current_area.iter().for_each(|&tile_index| {
                let tile = self.tile(tile_index);
                // Get the neighbor entities of the current tile
                let neighbor_indices = tile.tile_neighbors(&map_parameters);
                // Get the neighbor entities that don't belong to the current area and add them to the border entities
                neighbor_indices.into_iter().for_each(|neighbor_index| {
                    let neighbor_is_water =
                        self.terrain_type_query[neighbor_index] == TerrainType::Water;
                    if area_is_water == neighbor_is_water
                        && !tile_indices_current_area.contains(&neighbor_index)
                    {
                        border_tile_indices.insert(neighbor_index);
                    }
                });
            });

            // Get the size and area id of the surround area
            // Notice: different surround area may have the same size, we use BTreeMap only to retain the last added pair (area_size, area_id)
            let surround_area_size_and_id: BTreeMap<u32, i32> = border_tile_indices
                .iter()
                .map(|&tile_index| {
                    let area_id = self.area_id_query[tile_index];
                    let area_size = area_id_and_size.0[&area_id];
                    (area_size, area_id)
                })
                .collect();

            // Merge the current small area with the largest surround area (area_size >= MIN_AREA_SIZE) and (water or land area) is the same as the current area
            // Get the area id of the largest surround area and assign it to the current area
            if let Some((&area_size, &new_area_id)) = surround_area_size_and_id.last_key_value() {
                if area_size >= MIN_AREA_SIZE {
                    let first_tile_index = tile_indices_current_area[0];
                    let old_area_id = self.area_id_query[first_tile_index];

                    area_id_and_size.0.remove(&old_area_id);

                    area_id_and_size
                        .0
                        .entry(new_area_id)
                        .and_modify(|e| *e += tile_indices_current_area.len() as u32);

                    tile_indices_current_area.iter().for_each(|&tile_index| {
                        self.area_id_query[tile_index] = new_area_id;
                    })
                }
            }
        });
    }
}
