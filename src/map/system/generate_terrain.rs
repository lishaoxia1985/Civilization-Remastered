use std::cmp::max;

use bevy::prelude::{Commands, Query, Res, ResMut};

use crate::{
    map::tile_query::TileQuery,
    ruleset::{BaseTerrain, TerrainType},
    tile_map::{CvFractal, Flags, MapParameters, Temperature},
    RandomNumberGenerator, TileStorage,
};

pub fn generate_terrain(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
    query_tile: Query<TileQuery>,
) {
    let temperature_shift = 0.1;
    let desert_shift = 16;
    let mut desert_percent = 32;
    let plains_percent = 50;
    let mut snow_latitude = 0.75;
    let mut tundra_latitude = 0.6;
    let mut grass_latitude = 0.1;
    let desert_bottom_latitude = 0.2;
    let mut desert_top_latitude = 0.5;

    match map_parameters.temperature {
        Temperature::Cool => {
            desert_percent -= desert_shift;
            tundra_latitude -= temperature_shift * 1.5;
            desert_top_latitude -= temperature_shift;
            grass_latitude -= temperature_shift * 0.5;
        }
        Temperature::Normal => {}
        Temperature::Hot => {
            desert_percent += desert_shift;
            snow_latitude += temperature_shift * 0.5;
            tundra_latitude += temperature_shift;
            desert_top_latitude += temperature_shift;
            grass_latitude -= temperature_shift * 0.5;
        }
    }

    let desert_top_percent = 100;
    let desert_bottom_percent = max(0, 100 - desert_percent);
    let plains_top_percent = 100;
    let plains_bottom_percent = max(0, 100 - plains_percent);

    //let (seed, seed2, seed3) = self.random_number_generator.gen();
    let variation_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        3,
        Flags::default(),
        -1,
        -1,
    );
    let deserts_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        3,
        Flags::default(),
        -1,
        -1,
    );
    let plains_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        3,
        Flags::default(),
        -1,
        -1,
    );

    let [desert_top, plains_top] =
        deserts_fractal.get_height_from_percents(&[desert_top_percent, plains_top_percent])[..]
    else {
        panic!("Vec length does not match the pattern")
    };
    let [desert_bottom, plains_bottom] = plains_fractal
        .get_height_from_percents(&[desert_bottom_percent, plains_bottom_percent])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    tile_storage
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            let tile = query_tile.get(**entity).unwrap();
            tile.terrain_type != &TerrainType::Water
        })
        .for_each(|(index, entity)| {
            let [x, y] = map_parameters.index_to_offset_coordinate(index).to_array();

            let mut entity_commands = commands.entity(*entity);

            entity_commands.insert(BaseTerrain::Grassland);

            let deserts_height = deserts_fractal.get_height(x, y);
            let plains_height = plains_fractal.get_height(x, y);

            let mut latitude = map_parameters.latitude(index);
            latitude += (128 - variation_fractal.get_height(x, y)) as f64 / (255.0 * 5.0);
            latitude = latitude.clamp(0., 1.);

            if latitude >= snow_latitude {
                entity_commands.insert(BaseTerrain::Snow);
            } else if latitude >= tundra_latitude {
                entity_commands.insert(BaseTerrain::Tundra);
            } else if latitude < grass_latitude {
                entity_commands.insert(BaseTerrain::Grassland);
            } else if deserts_height >= desert_bottom
                && deserts_height <= desert_top
                && latitude >= desert_bottom_latitude
                && latitude < desert_top_latitude
            {
                entity_commands.insert(BaseTerrain::Desert);
            } else if plains_height >= plains_bottom && plains_height <= plains_top {
                entity_commands.insert(BaseTerrain::Plain);
            }
        });
}
