use bevy::prelude::{Entity, Query, Res, ResMut};
use rand::{rngs::StdRng, Rng};

use crate::{
    grid::{hex::HexOrientation, Direction},
    map::{
        base_terrain::BaseTerrain, terrain_type::TerrainType, AreaId, AreaIdAndSize, TileQuery,
        TileQueryItem, TileStorage,
    },
    tile_map::MapParameters,
    RandomNumberGenerator, River,
};

pub fn add_rivers(
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
    area_id_and_size: Res<AreaIdAndSize>,
    mut river: ResMut<River>,
    query_tile: Query<TileQuery>,
) {
    let river_source_range_default = 4;
    let sea_water_range_default = 3;
    const plots_per_river_edge: u32 = 12;

    fn pass_conditions(
        tile: &TileQueryItem,
        tile_storage: &TileStorage,
        random_number_generator: &mut StdRng,
        map_parameters: &MapParameters,
        area_id_and_size: &AreaIdAndSize,
        river: &River,
        query_tile: &Query<TileQuery>,
    ) -> [bool; 4] {
        let num_tiles = area_id_and_size.0[&tile.area_id.0];

        let is_coastal_land = tile.is_coastal_land(tile_storage, map_parameters, query_tile);

        let num_river_edges = num_river_edges(tile.area_id, &river, query_tile);
        [
            tile.terrain_type == &TerrainType::Hill || tile.terrain_type == &TerrainType::Mountain,
            !is_coastal_land && random_number_generator.gen_range(0..8) == 0,
            (tile.terrain_type == &TerrainType::Hill
                || tile.terrain_type == &TerrainType::Mountain)
                && (num_river_edges < num_tiles / plots_per_river_edge + 1),
            num_river_edges < num_tiles / plots_per_river_edge + 1,
        ]
    }

    // Returns the number of river edges in the area where the tile is
    // 1. Get the area where the tile is
    // 2. Get the number of rivers edge which the area (where the tile is) own
    fn num_river_edges(area_id: &AreaId, river: &River, query_tile: &Query<TileQuery>) -> u32 {
        let entities_in_area = query_tile
            .iter()
            .filter(|tile| tile.area_id.0 == area_id.0)
            .map(|tile| tile.entity)
            .collect::<Vec<_>>();

        let mut num_river_edges = 0;
        entities_in_area.iter().for_each(|entity_in_area| {
            river.0.values().for_each(|river_plot| {
                num_river_edges = river_plot
                    .iter()
                    .filter(|(entity_has_river, _)| entity_has_river == entity_in_area)
                    .count();
            });
        });
        num_river_edges as u32
    }

    // The tile where the river will start shoult meet these conditions:
    // 1. It should be not a water tile
    // 2. It should be not a natural wonder
    // 3. It should be not a tile which is neighbor to a natural wonder
    // 4. Its edge directions in [0..3] should be not water because the river edge uses (tile_entity, river_flow_direction) for storage.
    //    tile_entity is current tile index and river_flow_direction should be one of the edge directions in [0..3].
    let candidate_start_tiles: Vec<_> = query_tile
        .iter()
        .sort_unstable::<Entity>()
        .filter_map(|tile| {
            {
                if tile.natural_wonder.is_none() && tile.terrain_type != &TerrainType::Water {
                    let entity_neighbors = tile
                        .hex_position
                        .entity_neighbors(&tile_storage, &map_parameters);
                    if entity_neighbors.iter().all(|entity_neighbor| {
                        let neighbor_tile = query_tile.get(*entity_neighbor).unwrap();
                        neighbor_tile.natural_wonder.is_none()
                    }) {
                        map_parameters.edge_direction_array()[0..3]
                            .iter()
                            .all(|&direction| {
                                if let Some(entity_neighbor) = tile.hex_position.entity_neighbor(
                                    &tile_storage,
                                    &map_parameters,
                                    direction,
                                ) {
                                    let neighbor_tile = query_tile.get(entity_neighbor).unwrap();

                                    neighbor_tile.natural_wonder.is_none()
                                        && neighbor_tile.terrain_type != &TerrainType::Water
                                } else {
                                    false
                                }
                            })
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            .then_some(tile)
        })
        .collect();
    let mut river_id = 0;

    (0..4).for_each(|index| {
        let (river_source_range, sea_water_range) = if index <= 1 {
            (river_source_range_default, sea_water_range_default)
        } else {
            (
                (river_source_range_default / 2),
                (sea_water_range_default / 2),
            )
        };

        for tile in candidate_start_tiles.iter() {
            if pass_conditions(
                tile,
                &tile_storage,
                &mut random_number_generator.rng,
                &map_parameters,
                &area_id_and_size,
                &river,
                &query_tile,
            )[index]
                && !tile
                    .hex_position
                    .entities_at_distance(river_source_range, &tile_storage, &map_parameters)
                    .iter()
                    .any(|&entity| {
                        let tile = query_tile.get(entity).unwrap();

                        tile.is_freshwater(&tile_storage, &map_parameters, &river, &query_tile)
                    })
                && !tile
                    .hex_position
                    .entities_at_distance(sea_water_range, &tile_storage, &map_parameters)
                    .iter()
                    .any(|&entity| {
                        query_tile.get(entity).unwrap().terrain_type == &TerrainType::Water
                    })
            {
                do_river(
                    &mut river,
                    &mut random_number_generator,
                    tile.entity,
                    Direction::None,
                    Direction::None,
                    river_id,
                    &map_parameters,
                    &tile_storage,
                    &query_tile,
                );
                river_id += 1;
            }
        }
    });
}

fn do_river(
    river: &mut River,
    random_number_generator: &mut RandomNumberGenerator,
    start_plot_entity: Entity,
    this_flow_direction: Direction,
    original_flow_direction: Direction,
    river_id: i32,
    map_parameters: &MapParameters,
    tile_storage: &TileStorage,
    query_tile: &Query<TileQuery>,
) {
    // If the start plot have a river, exit the function
    // That will also prevent the river from forming a loop
    if river.0.values().any(|river| {
        river
            .iter()
            .any(|&(tile_entity, _)| tile_entity == start_plot_entity)
    }) {
        return;
    }

    let mut start_plot_entity = start_plot_entity;
    let mut this_flow_direction = this_flow_direction;
    let mut original_flow_direction = original_flow_direction;
    loop {
        let mut river_plot_entity;
        let mut best_flow_direction = Direction::None;
        match map_parameters.hex_layout.orientation {
            HexOrientation::Pointy => match this_flow_direction {
                Direction::East | Direction::West => unreachable!(),
                Direction::North => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::NorthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || neighbor_tile.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::SouthWest,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        } else {
                            river_plot_entity = neighbor_entity;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthEast => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::East,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::East,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::SouthWest,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthEast => {
                    let start_tile = query_tile.get(start_plot_entity).unwrap();
                    if let Some(neighbor_entity) = start_tile.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::East,
                    ) {
                        river_plot_entity = neighbor_entity
                    } else {
                        return;
                    };
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_entity2) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthWest,
                    ) {
                        let neighbor_tile2 = query_tile.get(neighbor_entity2).unwrap();
                        if neighbor_tile2.terrain_type == &TerrainType::Water
                            || neighbor_tile2.has_river(
                                Direction::East,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::South => {
                    let start_tile = query_tile.get(start_plot_entity).unwrap();
                    if let Some(neighbor_entity) = start_tile.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthWest,
                    ) {
                        river_plot_entity = neighbor_entity
                    } else {
                        return;
                    };
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_entity2) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::East,
                    ) {
                        let neighbor_tile2 = query_tile.get(neighbor_entity2).unwrap();
                        if neighbor_tile2.has_river(
                            Direction::SouthWest,
                            tile_storage,
                            map_parameters,
                            &river,
                            query_tile,
                        ) {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthWest => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthWest,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || neighbor_tile.has_river(
                                Direction::East,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || river_plot.has_river(
                                Direction::SouthWest,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthWest => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::West,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || neighbor_tile.has_river(
                                Direction::East,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        } else {
                            river_plot_entity = neighbor_entity;
                        }
                    } else {
                        return;
                    }
                }
                Direction::None => {
                    river_plot_entity = start_plot_entity;
                }
            },
            HexOrientation::Flat => match this_flow_direction {
                Direction::North | Direction::South => unreachable!(),
                Direction::NorthEast => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::NorthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::NorthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::South,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::East => {
                    let start_tile = query_tile.get(start_plot_entity).unwrap();
                    if let Some(neighbor_entity) = start_tile.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::NorthEast,
                    ) {
                        river_plot_entity = neighbor_entity
                    } else {
                        return;
                    };
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_entity2) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::South,
                    ) {
                        let neighbor_tile2 = query_tile.get(neighbor_entity2).unwrap();
                        if neighbor_tile2.terrain_type == &TerrainType::Water
                            || neighbor_tile2.has_river(
                                Direction::NorthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthEast => {
                    let start_tile = query_tile.get(start_plot_entity).unwrap();
                    if let Some(neighbor_entity) = start_tile.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::South,
                    ) {
                        river_plot_entity = neighbor_entity
                    } else {
                        return;
                    };
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthEast,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_entity2) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::NorthEast,
                    ) {
                        let neighbor_tile2 = query_tile.get(neighbor_entity2).unwrap();
                        if neighbor_tile2.terrain_type == &TerrainType::Water
                            || neighbor_tile2.has_river(
                                Direction::South,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthWest => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::South,
                    ) {
                        let neigbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neigbor_tile.terrain_type == &TerrainType::Water
                            || river_plot.has_river(
                                Direction::South,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neigbor_tile.has_river(
                                Direction::NorthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::West => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::SouthWest,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || neighbor_tile.has_river(
                                Direction::NorthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        } else {
                            river_plot_entity = neighbor_entity;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthWest => {
                    river_plot_entity = start_plot_entity;
                    river
                        .0
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_entity, this_flow_direction));
                    let river_plot = query_tile.get(river_plot_entity).unwrap();
                    if let Some(neighbor_entity) = river_plot.hex_position.entity_neighbor(
                        &tile_storage,
                        map_parameters,
                        Direction::North,
                    ) {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        if neighbor_tile.terrain_type == &TerrainType::Water
                            || neighbor_tile.has_river(
                                Direction::South,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                            || neighbor_tile.has_river(
                                Direction::SouthEast,
                                tile_storage,
                                map_parameters,
                                &river,
                                query_tile,
                            )
                        {
                            return;
                        } else {
                            river_plot_entity = neighbor_entity;
                        }
                    } else {
                        return;
                    }
                }
                Direction::None => {
                    river_plot_entity = start_plot_entity;
                }
            },
        }

        let river_plot = query_tile.get(river_plot_entity).unwrap();

        if river_plot.terrain_type == &TerrainType::Water {
            return;
        }

        // In this tuple, The first element is next possible flow, the second element is the direction of the special plot relative to current plot
        // We evaluate the weight value of the special plot using a certain algorithm and select the minimum one to determine the next direction of the river flow
        let adjacent_plot_directions = match map_parameters.hex_layout.orientation {
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

        fn next_flow_directions(
            flow_direction: Direction,
            map_parameters: &MapParameters,
        ) -> [Direction; 2] {
            let direction_array = map_parameters.corner_direction_array();
            let flow_direction_entity = map_parameters
                .hex_layout
                .orientation
                .corner_index(flow_direction);
            [
                direction_array[(flow_direction_entity + 1) % 6], // turn_right_flow_direction
                direction_array[(flow_direction_entity + 5) % 6], // turn_left_flow_direction
            ]
        }

        fn river_value_at_plot(
            plot_entity: Entity,
            random_number_generator: &mut RandomNumberGenerator,
            tile_storage: &TileStorage,
            map_parameters: &MapParameters,
            query_tile: &Query<TileQuery>,
        ) -> i32 {
            let tile = query_tile.get(plot_entity).unwrap();

            fn plot_elevation(entity: Entity, query_tile: &Query<TileQuery>) -> i32 {
                let tile = query_tile.get(entity).unwrap();
                match tile.terrain_type {
                    TerrainType::Water => 2,
                    TerrainType::Flatland => 1,
                    TerrainType::Mountain => 4,
                    TerrainType::Hill => 3,
                }
            }

            if tile.natural_wonder.is_some()
                || tile
                    .hex_position
                    .entity_neighbors(tile_storage, map_parameters)
                    .iter()
                    .any(|&neighbor_entity| {
                        let neighbor_tile = query_tile.get(neighbor_entity).unwrap();
                        neighbor_tile.natural_wonder.is_some()
                    })
            {
                return -1;
            }

            let mut sum = plot_elevation(plot_entity, query_tile) * 20;
            let direction_array = map_parameters.edge_direction_array();
            direction_array.iter().for_each(|&direction| {
                if let Some(adjacent_entity) =
                    tile.hex_position
                        .entity_neighbor(tile_storage, map_parameters, direction)
                {
                    sum += plot_elevation(adjacent_entity, query_tile);

                    let adjacent_tile = query_tile.get(adjacent_entity).unwrap();

                    if adjacent_tile.base_terrain == &BaseTerrain::Desert {
                        sum += 4;
                    }
                } else {
                    sum += 40;
                }
            });
            sum += random_number_generator.rng.gen_range(0..10);
            sum
        }

        let adjacent_plot_list = adjacent_plot_directions
            .into_iter()
            .filter_map(|(flow_direction, direction)| {
                river_plot
                    .hex_position
                    .entity_neighbor(&tile_storage, map_parameters, direction)
                    .map(|neighbor_tile| (flow_direction, neighbor_tile))
            })
            .collect::<Vec<_>>();

        if best_flow_direction == Direction::None {
            let mut best_value = i32::MAX;
            for (flow_direction, adjacent_plot) in adjacent_plot_list.into_iter() {
                if flow_direction.opposite_direction() != original_flow_direction
                    && (this_flow_direction == Direction::None
                        || next_flow_directions(this_flow_direction, map_parameters)
                            .contains(&flow_direction))
                {
                    let mut value = river_value_at_plot(
                        adjacent_plot,
                        random_number_generator,
                        tile_storage,
                        map_parameters,
                        query_tile,
                    );
                    if flow_direction == original_flow_direction {
                        value = (value * 3) / 4;
                    }
                    if value < best_value {
                        best_value = value;
                        best_flow_direction = flow_direction;
                    }
                }
            }
        }

        if best_flow_direction != Direction::None {
            if original_flow_direction == Direction::None {
                original_flow_direction = best_flow_direction;
            }
            start_plot_entity = river_plot_entity;
            this_flow_direction = best_flow_direction;
        } else {
            return;
        }
    }
}
