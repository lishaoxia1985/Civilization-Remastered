use bevy::{
    prelude::{Commands, Entity, Query, Res, ResMut},
    utils::{HashMap, HashSet},
};
use rand::Rng;

use crate::{
    map::{
        base_terrain::BaseTerrain,
        feature::Feature,
        terrain_type::TerrainType,
        tile_query::{TileQuery, TileQueryItem},
        AreaIdAndSize, TileStorage,
    },
    tile_map::MapParameters,
    RandomNumberGenerator, River,
};

/// This function generates lakes on the map.
///
/// This fun is used because when we create the world map by System `spawn_tile_type`, some water areas will be created surrounded by land.
/// If these water areas are small enough, they will be considered as lakes and will be replaced by the `TerrainType::Lake` terrain.
pub fn generate_lake(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    area_id_and_size: Res<AreaIdAndSize>,
    query_tile: Query<TileQuery>,
) {
    query_tile.iter().for_each(|tile| {
        if tile.terrain_type == &TerrainType::Water
            && area_id_and_size.0[&tile.area_id.0] <= map_parameters.lake_max_area_size
        {
            commands.entity(tile.entity).insert(BaseTerrain::Lake);
        }
    });
}

pub fn add_lakes(
    mut commands: Commands,
    tile_storage: Res<TileStorage>,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    river: Res<River>,
    query_tile: Query<TileQuery>,
) {
    let large_lake_num = map_parameters.large_lake_num;

    // Create a HashSet to store the entities of the tiles that have already been added as lakes
    let mut added_lake_entities = HashSet::new();
    let mut num_large_lakes_added = 0;
    let lake_plot_rand = 25;

    query_tile
        .iter()
        .sort_unstable::<Entity>()
        .for_each(|tile| {
            if can_add_lake(
                &tile,
                &added_lake_entities,
                &river,
                &tile_storage,
                &map_parameters,
                &query_tile,
            ) && random_number_generator.rng.gen_range(0..lake_plot_rand) == 0
            {
                if num_large_lakes_added < large_lake_num {
                    let add_more_lakes = add_more_lake(
                        &mut commands,
                        tile.entity,
                        &mut added_lake_entities,
                        &mut random_number_generator,
                        &river,
                        &tile_storage,
                        &map_parameters,
                        &query_tile,
                    );

                    if add_more_lakes {
                        num_large_lakes_added += 1;
                    }
                }
                let mut entity_commands = commands.entity(tile.entity);
                entity_commands.insert(TerrainType::Water);
                entity_commands.insert(BaseTerrain::Lake);
                entity_commands.remove::<Feature>();
                added_lake_entities.insert(tile.entity);
            }
        });
}

fn add_more_lake(
    commands: &mut Commands,
    entity: Entity,
    added_lake_entities: &mut HashSet<Entity>,
    random_number_generator: &mut RandomNumberGenerator,
    river: &River,
    tile_storage: &TileStorage,
    map_parameters: &MapParameters,
    query_tile: &Query<TileQuery>,
) -> bool {
    let mut large_lake = 0;

    let edge_direction_array = map_parameters.edge_direction_array();

    let tile = query_tile.get(entity).unwrap();

    for &direction in edge_direction_array.iter() {
        let neighbor_entity =
            tile.hex_position
                .entity_neighbor(tile_storage, map_parameters, direction);
        if let Some(neighbor_entity) = neighbor_entity {
            let neighbor_tile = query_tile.get(neighbor_entity).unwrap();

            if can_add_lake(
                &neighbor_tile,
                added_lake_entities,
                river,
                tile_storage,
                map_parameters,
                query_tile,
            ) && random_number_generator.rng.gen_range(0..(large_lake + 4)) < 3
            {
                let mut entity_commands = commands.entity(neighbor_entity);
                entity_commands.insert(TerrainType::Water);
                entity_commands.insert(BaseTerrain::Lake);
                entity_commands.remove::<Feature>();
                added_lake_entities.insert(neighbor_entity);
                large_lake += 1;
            }
        }
    }

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
fn can_add_lake(
    tile: &TileQueryItem,
    added_lake_entities: &HashSet<Entity>,
    river: &River,
    tile_storage: &TileStorage,
    map_parameters: &MapParameters,
    query_tile: &Query<TileQuery>,
) -> bool {
    let edge_direction_array = map_parameters.edge_direction_array();

    // Check if the current tile is suitable for a lake
    if tile.terrain_type == &TerrainType::Water
        || tile.natural_wonder.is_some()
        || edge_direction_array.iter().any(|&direction| {
            tile.has_river(direction, tile_storage, map_parameters, river, query_tile)
        })
    {
        return false;
    }

    let neighbor_entities = tile
        .hex_position
        .entity_neighbors(&tile_storage, &map_parameters);

    // Check if all neighbor tiles are also suitable
    neighbor_entities.iter().all(|&entity| {
        let tile = query_tile.get(entity).unwrap();
        tile.terrain_type != &TerrainType::Water
            && !added_lake_entities.contains(&entity)
            && tile.natural_wonder.is_none()
    })
}
