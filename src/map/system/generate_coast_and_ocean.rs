use bevy::{
    prelude::{Commands, Entity, Query, Res, ResMut},
    utils::hashbrown::HashSet,
};
use rand::Rng;

use crate::{
    map::TileQuery,
    ruleset::{BaseTerrain, TerrainType},
    tile_map::MapParameters,
    RandomNumberGenerator, TileStorage,
};

pub fn generate_coast_and_ocean(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    tile_storage: Res<TileStorage>,
    query_tile: Query<TileQuery>,
) {
    query_tile.iter().for_each(|tile| {
        if tile.terrain_type == &TerrainType::Water {
            let entity_neighbors = tile
                .hex_position
                .entity_neighbors(&tile_storage, &map_parameters);
            if entity_neighbors
                .iter()
                .any(|&entity| query_tile.get(entity).unwrap().terrain_type != &TerrainType::Water)
            {
                commands.entity(tile.entity).insert(BaseTerrain::Coast);
            }
        }
    })
}

pub fn expand_coast(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
    query_tile: Query<TileQuery>,
) {
    // HashSet to store the entities that have been expanded to coast
    let mut expansion_entities = HashSet::new();

    // The tiles that can be expanded should meet some conditions:
    // 1. They are water and not already coast
    // 2. They have at least one neighbor that is coast
    for &chance in &map_parameters.coast_expand_chance {
        query_tile
            .iter()
            .sort_unstable::<Entity>()
            .filter(|tile| {
                tile.terrain_type == &TerrainType::Water && tile.base_terrain != &BaseTerrain::Coast
            })
            .for_each(|tile| {
                if !expansion_entities.contains(&tile.entity)
                    && tile
                        .hex_position
                        .entity_neighbors(&tile_storage, &map_parameters)
                        .iter()
                        .any(|&entity| {
                            let tile = query_tile.get(entity).unwrap();
                            tile.base_terrain == &BaseTerrain::Coast
                                || expansion_entities.contains(&entity)
                        })
                    && random_number_generator.rng.gen_bool(chance)
                {
                    expansion_entities.insert(tile.entity);
                    commands.entity(tile.entity).insert(BaseTerrain::Coast);
                }
            });
    }
}
