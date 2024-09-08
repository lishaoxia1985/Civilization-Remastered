use bevy::{
    prelude::{Commands, Entity, Query, Res, ResMut},
    utils::{HashMap, HashSet},
};
use rand::prelude::SliceRandom;
use rand::Rng;

use crate::{
    grid::hex::Hex,
    map::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        terrain_type::TerrainType, tile_query::TileQuery, TileStorage,
    },
    ruleset::{Ruleset, Unique},
    tile_map::MapParameters,
    RandomNumberGenerator, River,
};

pub fn generate_natural_wonder(
    mut commands: Commands,
    ruleset: Res<Ruleset>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
    map_parameters: Res<MapParameters>,
    river: Res<River>,
    query_tile: Query<TileQuery>,
) {
    let natural_wonder_list: Vec<_> = ruleset.natural_wonders.keys().collect();

    let mut natural_wonder_and_entity_and_score = HashMap::new();

    let mut land_id_and_area_size: Vec<_> = query_tile
        .iter()
        .filter(|tile| {
            tile.terrain_type == &TerrainType::Hill || tile.terrain_type == &TerrainType::Flatland
        })
        .fold(HashMap::new(), |mut map, tile| {
            *map.entry(tile.area_id.0).or_insert(0) += 1;
            map
        })
        .into_iter()
        .collect();

    // First, sort by area_size in descending order using std::cmp::Reverse
    // If area_size is the same, sort by land_id in ascending order
    land_id_and_area_size
        .sort_unstable_by_key(|&(land_id, area_size)| (std::cmp::Reverse(area_size), land_id));

    fn matches_wonder_filter(entity: Entity, filter: &str, query_tile: &Query<TileQuery>) -> bool {
        let tile = query_tile.get(entity).unwrap();

        match filter {
            "Elevated" => {
                tile.terrain_type == &TerrainType::Mountain
                    || tile.terrain_type == &TerrainType::Hill
            }
            "Land" => tile.terrain_type != &TerrainType::Water,
            _ => {
                tile.terrain_type.name() == filter
                    || tile.base_terrain.name() == filter
                    || tile.feature.map_or(false, |f| f.name() == filter)
            }
        }
    }

    for tile in query_tile.iter().sort_unstable::<Entity>() {
        for &natural_wonder_name in &natural_wonder_list {
            let possible_natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

            match natural_wonder_name.as_str() {
                "Great Barrier Reef" => {
                    if let Some(neighbor_entity) = tile.hex_position.entity_neighbor(
                        &tile_storage,
                        &map_parameters,
                        map_parameters.edge_direction_array()[1],
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        let mut all_neigbor_entities = HashSet::new();

                        all_neigbor_entities.extend(
                            tile.hex_position
                                .entity_neighbors(&tile_storage, &map_parameters)
                                .iter()
                                .map(|&entity| entity),
                        );
                        all_neigbor_entities.extend(
                            neighbor_tile
                                .hex_position
                                .entity_neighbors(&tile_storage, &map_parameters)
                                .iter()
                                .map(|&entity| entity),
                        );

                        all_neigbor_entities.remove(&tile.entity);
                        all_neigbor_entities.remove(&neighbor_entity);

                        if all_neigbor_entities.len() == 8
                            && all_neigbor_entities.iter().all(|&entity| {
                                let tile = query_tile.get(entity).unwrap();

                                tile.terrain_type == &TerrainType::Water
                                    && tile.base_terrain != &BaseTerrain::Lake
                                    && tile.feature != Some(&Feature::Ice)
                            })
                            && all_neigbor_entities
                                .iter()
                                .filter(|&entity| {
                                    let tile = query_tile.get(*entity).unwrap();
                                    tile.base_terrain == &BaseTerrain::Coast
                                })
                                .count()
                                >= 4
                        {
                            natural_wonder_and_entity_and_score
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push((tile.entity, 1));
                        }
                    }
                }
                _ => {
                    if tile.is_freshwater(&tile_storage, &map_parameters, &river, &query_tile)
                        != possible_natural_wonder.is_fresh_water
                    {
                        continue;
                    };

                    if !possible_natural_wonder
                        .occurs_on_type
                        .contains(&tile.terrain_type)
                        || !possible_natural_wonder
                            .occurs_on_base
                            .contains(&tile.base_terrain)
                    {
                        continue;
                    }

                    let check_unique_conditions =
                        possible_natural_wonder.uniques.iter().all(|unique| {
                            let unique = Unique::new(unique);
                            match unique.placeholder_text.as_str() {
                                "Must be adjacent to [] [] tiles" => {
                                    let count = tile
                                        .hex_position
                                        .entity_neighbors(&tile_storage, &map_parameters)
                                        .iter()
                                        .filter(|entity| {
                                            matches_wonder_filter(
                                                **entity,
                                                unique.params[1].as_str(),
                                                &query_tile,
                                            )
                                        })
                                        .count();
                                    count == unique.params[0].parse::<usize>().unwrap()
                                }
                                "Must be adjacent to [] to [] [] tiles" => {
                                    let count = tile
                                        .hex_position
                                        .entity_neighbors(&tile_storage, &map_parameters)
                                        .iter()
                                        .filter(|entity| {
                                            matches_wonder_filter(
                                                **entity,
                                                unique.params[2].as_str(),
                                                &query_tile,
                                            )
                                        })
                                        .count();
                                    count >= unique.params[0].parse::<usize>().unwrap()
                                        && count <= unique.params[1].parse::<usize>().unwrap()
                                }
                                "Must not be on [] largest landmasses" => {
                                    let index = unique.params[0].parse::<usize>().unwrap();
                                    !land_id_and_area_size
                                        .iter()
                                        .take(index)
                                        .any(|(id, _)| tile.area_id.0 == *id)
                                }
                                "Must be on [] largest landmasses" => {
                                    let index = unique.params[0].parse::<usize>().unwrap();
                                    land_id_and_area_size
                                        .iter()
                                        .take(index)
                                        .any(|(id, _)| tile.area_id.0 == *id)
                                }
                                _ => true,
                            }
                        });
                    // end check unique conditions

                    if check_unique_conditions {
                        natural_wonder_and_entity_and_score
                            .entry(natural_wonder_name)
                            .or_insert_with(Vec::new)
                            .push((tile.entity, 1));
                    }
                }
            }
        }
    }

    // Get the natural wonders that can be placed
    let mut selected_natural_wonder_list: Vec<_> = natural_wonder_and_entity_and_score
        .keys()
        .cloned()
        .collect();
    /* The order of selected_natural_wonder_list is random, so we should arrange this list in order
    to ensure that the obtained Vec is the same every time. */
    selected_natural_wonder_list.sort_unstable();
    // Shuffle the list that we can choose natural wonder randomly
    selected_natural_wonder_list.shuffle(&mut random_number_generator.rng);

    // Store current how many natural wonders have been placed
    let mut j = 0;
    // Store the index of the tile where the natural wonder has been placed
    let mut placed_natural_wonder_entities = Vec::new();

    // start to place wonder
    for &natural_wonder_name in &selected_natural_wonder_list {
        if j <= map_parameters.natural_wonder_num {
            // For every natural wonder, give a score to the position where the natural wonder can place.
            // The score is related to the min value of the distance from the position to all the placed natural wonders
            // If no natural wonder has placed, we choose the random place where the current natural wonder can place for the current natural wonder

            // the score method start
            let tile_index_and_score = natural_wonder_and_entity_and_score
                .get_mut(natural_wonder_name)
                .unwrap();
            for (position_x_entity, score) in tile_index_and_score.iter_mut() {
                let closest_natural_wonder_dist = placed_natural_wonder_entities
                    .iter()
                    .map(|position_y_entity| {
                        let tile_x = query_tile.get(*position_x_entity).unwrap();
                        let tile_y = query_tile.get(*position_y_entity).unwrap();

                        let position_x_hex = tile_x.hex_position;
                        let position_y_hex = tile_y.hex_position;
                        Hex::hex_distance(Hex::from(position_x_hex.0), Hex::from(position_y_hex.0))
                    })
                    .min()
                    .unwrap_or(1000000);
                *score = if closest_natural_wonder_dist <= 10 {
                    100 * closest_natural_wonder_dist
                } else {
                    1000 + (closest_natural_wonder_dist - 10)
                } + random_number_generator.rng.gen_range(0..100);
            }
            // the score method end

            // choose the max score position as the candidate position for the current natural wonder
            let max_score_position_entity = tile_index_and_score
                .iter()
                .max_by_key(|&(_, score)| score)
                .map(|&(index, _)| index)
                .unwrap();

            if !placed_natural_wonder_entities.contains(&max_score_position_entity) {
                let natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                let mut entity_commands = commands.entity(max_score_position_entity);
                // At first, we should remove feature from the tile
                entity_commands.remove::<Feature>();

                match natural_wonder_name.as_str() {
                    "Great Barrier Reef" => {
                        let tile = query_tile.get(max_score_position_entity).unwrap();
                        let neighbor_entity = tile
                            .hex_position
                            .entity_neighbor(
                                &tile_storage,
                                &map_parameters,
                                map_parameters.edge_direction_array()[1],
                            )
                            .unwrap();

                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();

                        let neighbor_entities: Vec<_> = tile
                            .hex_position
                            .entity_neighbors(&tile_storage, &map_parameters)
                            .iter()
                            .map(|&entity| entity)
                            .collect();
                        let neighbor_neighbor_entities: Vec<_> = neighbor_tile
                            .hex_position
                            .entity_neighbors(&tile_storage, &map_parameters)
                            .iter()
                            .map(|&entity| entity)
                            .collect();

                        neighbor_entities.into_iter().for_each(|entity| {
                            let mut entity_commands = commands.entity(entity);
                            entity_commands.insert(TerrainType::Water);
                            entity_commands.insert(BaseTerrain::Coast);
                        });
                        neighbor_neighbor_entities.into_iter().for_each(|entity| {
                            let mut entity_commands = commands.entity(entity);
                            entity_commands.insert(TerrainType::Water);
                            entity_commands.insert(BaseTerrain::Coast);
                        });
                        // place the natural wonder on the candidate position and its adjacent tile
                        let mut entity_commands = commands.entity(max_score_position_entity);
                        entity_commands
                            .insert(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                        let mut adj_entity_commands = commands.entity(neighbor_entity);
                        adj_entity_commands
                            .insert(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                        // add the position of the placed natural wonder to the list of placed natural wonder positions
                        placed_natural_wonder_entities.push(max_score_position_entity);
                        placed_natural_wonder_entities.push(neighbor_entity);
                    }
                    "Rock of Gibraltar" => {
                        let tile = query_tile.get(max_score_position_entity).unwrap();
                        let neighbor_entities: Vec<_> = tile
                            .hex_position
                            .entity_neighbors(&tile_storage, &map_parameters)
                            .iter()
                            .map(|&entity| entity)
                            .collect();

                        neighbor_entities.into_iter().for_each(|neighbor_entity| {
                            let mut entity_commands = commands.entity(neighbor_entity);

                            let neighbor_tile = query_tile.get(neighbor_entity).unwrap();

                            if neighbor_tile.terrain_type == &TerrainType::Water {
                                entity_commands.insert(BaseTerrain::Coast);
                            } else {
                                entity_commands.insert(TerrainType::Mountain);
                            }
                        });
                        let mut entity_commands = commands.entity(max_score_position_entity);
                        // Edit the choice tile's terrain_type to match the natural wonder
                        entity_commands.insert(TerrainType::Flatland);
                        // Edit the choice tile's base_terrain to match the natural wonder
                        entity_commands.insert(BaseTerrain::Grassland);
                        // place the natural wonder on the candidate position
                        entity_commands
                            .insert(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                        // add the position of the placed natural wonder to the list of placed natural wonder positions
                        placed_natural_wonder_entities.push(max_score_position_entity);
                    }
                    _ => {
                        let mut entity_commands = commands.entity(max_score_position_entity);
                        // Edit the choice tile's terrain_type to match the natural wonder
                        if let Some(turn_into_terrain_type) = natural_wonder.turns_into_type {
                            entity_commands.insert(turn_into_terrain_type);
                        };
                        // Edit the choice tile's base_terrain to match the natural wonder
                        if let Some(turn_into_base_terrain) = natural_wonder.turns_into_base {
                            entity_commands.insert(turn_into_base_terrain);
                        }
                        // place the natural wonder on the candidate position
                        entity_commands
                            .insert(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                        // add the position of the placed natural wonder to the list of placed natural wonder positions
                        placed_natural_wonder_entities.push(max_score_position_entity);
                    }
                }
                j += 1;
            }
        }
    }
}

/// If the natural wonder is not water, and its neighbors have water tile, then change the water neighbor tiles to lake or coast
pub fn regenerate_coast(
    mut commands: Commands,
    tile_storage: Res<TileStorage>,
    map_parameters: Res<MapParameters>,
    query_tile: Query<TileQuery>,
) {
    let placed_natural_wonder_tile: Vec<_> = query_tile
        .iter()
        .filter(|tile| tile.natural_wonder.is_some())
        .map(|tile| tile.entity)
        .collect();

    placed_natural_wonder_tile.iter().for_each(|&entity| {
        let tile = query_tile.get(entity).unwrap();

        if tile.terrain_type != &TerrainType::Water {
            let neighbor_entities: Vec<_> = tile
                .hex_position
                .entity_neighbors(&tile_storage, &map_parameters)
                .iter()
                .map(|&entity| entity)
                .collect();

            neighbor_entities.iter().for_each(|&neighbor_entity| {
                let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                if neighbor_tile.terrain_type == &TerrainType::Water {
                    let mut entity_commands = commands.entity(neighbor_entity);

                    let neighbor_neighbor_entities = neighbor_tile
                        .hex_position
                        .entity_neighbors(&tile_storage, &map_parameters);

                    if neighbor_neighbor_entities
                        .iter()
                        .any(|&neighbor_neighbor_entity| {
                            let neighbor_neighbor_tile =
                                query_tile.get(neighbor_neighbor_entity).unwrap();
                            neighbor_neighbor_tile.base_terrain == &BaseTerrain::Lake
                        })
                    {
                        entity_commands.insert(BaseTerrain::Lake);
                    } else {
                        entity_commands.insert(BaseTerrain::Coast);
                    };
                };
            });
        }
    });
}
