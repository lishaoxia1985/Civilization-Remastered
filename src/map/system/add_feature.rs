use bevy::{
    prelude::{Commands, Entity, Query, Res, ResMut},
    utils::HashMap,
};
use rand::Rng;

use crate::{
    map::{
        base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType,
        tile_query::TileQuery, TileStorage,
    },
    ruleset::Ruleset,
    tile_map::{MapParameters, Rainfall},
    RandomNumberGenerator, River,
};

pub fn add_features(
    mut commands: Commands,
    ruleset: Res<Ruleset>,
    river: Res<River>,
    tile_storage: Res<TileStorage>,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    query_tile: Query<TileQuery>,
) {
    let rainfall = match map_parameters.rainfall {
        Rainfall::Arid => -4,
        Rainfall::Normal => 0,
        Rainfall::Wet => 4,
        Rainfall::Random => random_number_generator.rng.gen_range(0..11) - 5,
    };

    let equator_adjustment = 0;
    let mut jungle_percent = 12;
    let mut forest_percent = 18;
    let mut marsh_percent = 3;
    let mut oasis_percent = 1;

    jungle_percent += rainfall;
    forest_percent += rainfall;
    marsh_percent += rainfall / 2;
    oasis_percent += rainfall / 4;

    let equator = equator_adjustment;

    let jungle_max_percent = jungle_percent;
    let forest_max_percent = forest_percent;
    let marsh_max_percent = marsh_percent;
    let oasis_max_percent = oasis_percent;

    let mut placed_entities_and_features: HashMap<Entity, Feature> = HashMap::new();

    let mut forest_count = 0;
    let mut jungle_count = 0;
    let mut marsh_count = 0;
    let mut oasis_count = 0;
    let mut num_land_plots = 0;
    let jungle_bottom = equator - (jungle_percent as f64 * 0.5).ceil() as i32;
    let jungle_top = equator + (jungle_percent as f64 * 0.5).ceil() as i32;

    for (index, &entity) in tile_storage.tiles.iter().enumerate() {
        let mut entity_commands = commands.entity(entity);

        let tile = query_tile.get(entity).unwrap();

        let latitude = map_parameters.latitude(index);

        let neighbor_entities = tile
            .hex_position
            .entity_neighbors(&tile_storage, &map_parameters);

        /* **********start to add ice********** */
        if tile.is_impassable(&ruleset) {
            continue;
        } else if tile.terrain_type == &TerrainType::Water {
            if !map_parameters
                .edge_direction_array()
                .iter()
                .any(|&direction| {
                    tile.has_river(
                        direction,
                        &tile_storage,
                        &map_parameters,
                        &river,
                        &query_tile,
                    )
                })
                && ruleset.features["Ice"]
                    .occurs_on_type
                    .contains(&tile.terrain_type)
                && ruleset.features["Ice"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
            {
                if latitude > 0.78 {
                    let mut score = random_number_generator.rng.gen_range(0..100) as f64;
                    score += latitude * 100.;
                    if neighbor_entities.iter().any(|&entity| {
                        let tile = query_tile.get(entity).unwrap();
                        tile.terrain_type != &TerrainType::Water
                    }) {
                        score /= 2.0;
                    }
                    let a = neighbor_entities
                        .iter()
                        .filter(|&entity| {
                            placed_entities_and_features.get(entity) == Some(&Feature::Ice)
                        })
                        .count();
                    score += 10. * a as f64;
                    if score > 130. {
                        entity_commands.insert(Feature::Ice);
                        placed_entities_and_features.insert(entity, Feature::Ice);
                    }
                }
            }
        }
        /* **********the end of add ice********** */
        else {
            /* **********start to add Floodplain********** */
            num_land_plots += 1;
            if map_parameters
                .edge_direction_array()
                .iter()
                .any(|&direction| {
                    tile.has_river(
                        direction,
                        &tile_storage,
                        &map_parameters,
                        &river,
                        &query_tile,
                    )
                })
                && ruleset.features["Floodplain"]
                    .occurs_on_type
                    .contains(&tile.terrain_type)
                && ruleset.features["Floodplain"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
            {
                entity_commands.insert(Feature::Floodplain);
                continue;
            }
            /* **********the end of add Floodplain********** */
            /* **********start to add oasis********** */
            else if ruleset.features["Oasis"]
                .occurs_on_type
                .contains(&tile.terrain_type)
                && ruleset.features["Oasis"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                && (oasis_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                    <= oasis_max_percent
                && random_number_generator.rng.gen_range(0..4) == 1
            {
                entity_commands.insert(Feature::Oasis);
                oasis_count += 1;
                continue;
            }
            /* **********the end of add oasis********** */
            /* **********start to add march********** */
            if ruleset.features["Marsh"]
                .occurs_on_type
                .contains(&tile.terrain_type)
                && ruleset.features["Marsh"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                && (marsh_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                    <= marsh_max_percent
            {
                let mut score = 300;

                let a = neighbor_entities
                    .iter()
                    .filter(|&entity| {
                        placed_entities_and_features.get(entity) == Some(&Feature::Marsh)
                    })
                    .count();
                match a {
                    0 => (),
                    1 => score += 50,
                    2 | 3 => score += 150,
                    4 => score -= 50,
                    _ => score -= 200,
                };
                if random_number_generator.rng.gen_range(0..300) <= score {
                    entity_commands.insert(Feature::Marsh);
                    placed_entities_and_features.insert(entity, Feature::Marsh);
                    marsh_count += 1;
                    continue;
                }
            };
            /* **********the end of add march********** */
            /* **********start to add jungle********** */
            if ruleset.features["Jungle"]
                .occurs_on_type
                .contains(&tile.terrain_type)
                && ruleset.features["Jungle"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                && (jungle_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                    <= jungle_max_percent
                && (latitude >= jungle_bottom as f64 / 100. && latitude <= jungle_top as f64 / 100.)
            {
                let mut score = 300;

                let a = neighbor_entities
                    .iter()
                    .filter(|&entity| {
                        placed_entities_and_features.get(entity) == Some(&Feature::Jungle)
                    })
                    .count();
                match a {
                    0 => (),
                    1 => score += 50,
                    2 | 3 => score += 150,
                    4 => score -= 50,
                    _ => score -= 200,
                };
                if random_number_generator.rng.gen_range(0..300) <= score {
                    entity_commands.insert(Feature::Jungle);
                    placed_entities_and_features.insert(entity, Feature::Jungle);

                    if tile.terrain_type == &TerrainType::Hill
                        && (tile.base_terrain == &BaseTerrain::Grassland
                            || tile.base_terrain == &BaseTerrain::Plain)
                    {
                        entity_commands.insert(BaseTerrain::Plain);
                    } else {
                        entity_commands.insert(TerrainType::Flatland);
                        entity_commands.insert(BaseTerrain::Plain);
                    }

                    jungle_count += 1;
                    continue;
                }
            }
            /* **********the end of add jungle********** */
            /* **********start to add forest********** */
            if ruleset.features["Forest"]
                .occurs_on_type
                .contains(&tile.terrain_type)
                && ruleset.features["Forest"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                && (forest_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                    <= forest_max_percent
            {
                let mut score = 300;

                let a = neighbor_entities
                    .iter()
                    .filter(|&entity| {
                        placed_entities_and_features.get(entity) == Some(&Feature::Forest)
                    })
                    .count();
                match a {
                    0 => (),
                    1 => score += 50,
                    2 | 3 => score += 150,
                    4 => score -= 50,
                    _ => score -= 200,
                };
                if random_number_generator.rng.gen_range(0..300) <= score {
                    entity_commands.insert(Feature::Forest);
                    placed_entities_and_features.insert(entity, Feature::Forest);
                    forest_count += 1;
                    continue;
                }
            }
            /* **********the end of add forest********** */
        }
    }
}
