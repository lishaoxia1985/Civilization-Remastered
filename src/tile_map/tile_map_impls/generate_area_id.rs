use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::utils::hashbrown::HashSet;

use crate::{
    map::{terrain_type::TerrainType, AreaIdAndSize},
    tile_map::{tile_index::TileIndex, MapParameters, TileMap},
};

impl TileMap {
    pub fn bfs(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &mut AreaIdAndSize,
        mut area_tile_indices: HashSet<TileIndex>,
    ) {
        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&initial_area_tile_index) = area_tile_indices.iter().next() {
            self.area_id_query[*initial_area_tile_index] = current_area_id;
            area_tile_indices.remove(&initial_area_tile_index);

            // Store all the entities in the current area.
            let mut tile_indices_in_current_area = HashSet::new();
            tile_indices_in_current_area.insert(initial_area_tile_index);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tile_indices_to_check = VecDeque::new();
            tile_indices_to_check.push_back(initial_area_tile_index);

            while let Some(tile_index_we_are_checking) = tile_indices_to_check.pop_front() {
                tile_index_we_are_checking
                    .neighbor_tile_indices(map_parameters)
                    .iter()
                    .for_each(|&tile_index| {
                        if !tile_indices_in_current_area.contains(&tile_index)
                            && area_tile_indices.contains(&tile_index)
                        {
                            tile_indices_in_current_area.insert(tile_index);
                            self.area_id_query[*tile_index] = current_area_id;
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
        mut area_tile_indices: HashSet<TileIndex>,
    ) {
        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&initial_area_tile_index) = area_tile_indices.iter().next() {
            self.area_id_query[*initial_area_tile_index] = current_area_id;
            area_tile_indices.remove(&initial_area_tile_index);

            // Store all the entities in the current area.
            let mut tile_indices_in_current_area = HashSet::new();
            tile_indices_in_current_area.insert(initial_area_tile_index);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tile_indices_to_check = Vec::new();
            tile_indices_to_check.push(initial_area_tile_index);

            while let Some(tile_index_we_are_checking) = tile_indices_to_check.pop() {
                tile_index_we_are_checking
                    .neighbor_tile_indices(map_parameters)
                    .iter()
                    .for_each(|&tile_index| {
                        if !tile_indices_in_current_area.contains(&tile_index)
                            && area_tile_indices.contains(&tile_index)
                        {
                            tile_indices_in_current_area.insert(tile_index);
                            self.area_id_query[*tile_index] = current_area_id;
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
        area_id_and_size: &mut AreaIdAndSize,
    ) {
        area_id_and_size.0.clear();

        let height = map_parameters.map_size.height;
        let width = map_parameters.map_size.width;

        let size = (height * width) as usize;

        self.area_id_query = vec![-1; size];

        let mut water_tile_indices = HashSet::new();
        let mut hill_and_flatland_tile_indices = HashSet::new();
        let mut mountain_tile_indices = HashSet::new();

        self.tile_indices_iter().for_each(|tile_index| {
            match tile_index.terrain_type(self) {
                TerrainType::Water => water_tile_indices.insert(tile_index),
                TerrainType::Flatland | TerrainType::Hill => {
                    hill_and_flatland_tile_indices.insert(tile_index)
                }
                TerrainType::Mountain => mountain_tile_indices.insert(tile_index),
            };
        });

        self.bfs(map_parameters, area_id_and_size, water_tile_indices);
        self.bfs(
            map_parameters,
            area_id_and_size,
            hill_and_flatland_tile_indices,
        );
        self.bfs(map_parameters, area_id_and_size, mountain_tile_indices);

        self.reassign_area_id(map_parameters, area_id_and_size);
    }

    pub fn reassign_area_id(
        &mut self,
        map_parameters: &MapParameters,
        area_id_and_size: &mut AreaIdAndSize,
    ) {
        const MIN_AREA_SIZE: u32 = 7;

        // Get the id of the smaller area whose size < MIN_AREA_SIZE
        let small_area_id: Vec<_> = area_id_and_size
            .0
            .iter()
            .filter_map(|(&id, &size)| (size < MIN_AREA_SIZE).then_some(id))
            .collect();

        small_area_id.into_iter().for_each(|current_area_id| {
            let tile_indices_in_current_area = self
                .tile_indices_iter()
                .filter(|tile_index| tile_index.area_id(self) == current_area_id)
                .collect::<Vec<_>>();

            let first_tile_index = tile_indices_in_current_area[0];
            // Check if the current area is water
            let current_area_is_water = first_tile_index.terrain_type(self) == TerrainType::Water;

            // Get the border entities of the current area, these entities don't belong to the area, but they surround the area.
            // Using BTreeSet to store the border entities will make sure the entities are processed in the same order every time.
            // That means that we can get the same 'surround_area_size_and_id' every time.
            let mut border_tile_indices = BTreeSet::new();

            tile_indices_in_current_area.iter().for_each(|&tile_index| {
                // Get the neighbor entities of the current tile
                let neighbor_tile_indices = tile_index.neighbor_tile_indices(&map_parameters);
                // Get the neighbor entities that don't belong to the current area and add them to the border entities
                neighbor_tile_indices
                    .into_iter()
                    .for_each(|neighbor_tile_index| {
                        let neighbor_tile_is_water =
                            neighbor_tile_index.terrain_type(self) == TerrainType::Water;
                        // The neigbor tile is border tile if it meets the following conditions:
                        // 1. If the current area is water the neighbor tile is water, or if the current area is land the neighbor tile is land.
                        // 2. The neighbor tile doesn't belong to the current area.
                        if current_area_is_water == neighbor_tile_is_water
                            && !tile_indices_in_current_area.contains(&neighbor_tile_index)
                        {
                            border_tile_indices.insert(neighbor_tile_index);
                        }
                    });
            });

            // Get the size and area id of the surround area
            // Notice: different surround area may have the same size, we use BTreeMap only to retain the last added pair (area_size, area_id)
            let surround_area_size_and_id: BTreeMap<u32, i32> = border_tile_indices
                .iter()
                .map(|tile_index| {
                    let area_id = tile_index.area_id(self);
                    let area_size = area_id_and_size.0[&area_id];
                    (area_size, area_id)
                })
                .collect();

            // Merge the current small area with the largest surround area (area_size >= MIN_AREA_SIZE) and (water or land area) is the same as the current area
            // Get the area id of the largest surround area and assign it to the current area
            if let Some((&area_size, &new_area_id)) = surround_area_size_and_id.last_key_value() {
                if area_size >= MIN_AREA_SIZE {
                    let old_area_id = first_tile_index.area_id(self);

                    area_id_and_size.0.remove(&old_area_id);

                    area_id_and_size
                        .0
                        .entry(new_area_id)
                        .and_modify(|e| *e += tile_indices_in_current_area.len() as u32);

                    tile_indices_in_current_area.iter().for_each(|&tile_index| {
                        self.area_id_query[*tile_index] = new_area_id;
                    })
                }
            }
        });
    }
}
