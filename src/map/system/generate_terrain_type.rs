use bevy::{
    math::{DVec2, IVec2},
    prelude::{Commands, Res, ResMut},
};
use rand::Rng;

use crate::{
    ruleset::TerrainType,
    tile_map::{CvFractal, Flags, MapParameters, SeaLevel, WorldAge},
    RandomNumberGenerator, TileStorage,
};

pub fn generate_terrain_type_for_fractal(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
) {
    let continent_grain = 2;

    let sea_level_low = 65;
    let sea_level_normal = 72;
    let sea_level_high = 78;
    let world_age_old = 2;
    let world_age_normal = 3;
    let world_age_new = 5;

    let extra_mountains = 0;

    let tectonic_islands = false;

    let adjustment = match map_parameters.world_age {
        WorldAge::Old => world_age_old,
        WorldAge::Normal => world_age_normal,
        WorldAge::New => world_age_new,
    };

    let mountains = 97 - adjustment - extra_mountains;
    let hills_near_mountains = 91 - (adjustment * 2) - extra_mountains;
    let hills_bottom1 = 28 - adjustment;
    let hills_top1 = 28 + adjustment;
    let hills_bottom2 = 72 - adjustment;
    let hills_top2 = 72 + adjustment;
    let hills_clumps = 1 + adjustment;

    let water_percent = match map_parameters.sea_level {
        SeaLevel::Low => sea_level_low,
        SeaLevel::Normal => sea_level_normal,
        SeaLevel::High => sea_level_high,
        SeaLevel::Random => {
            sea_level_low
                + random_number_generator
                    .rng
                    .gen_range(0..=(sea_level_high - sea_level_low))
        }
    };

    let orientation = map_parameters.hex_layout.orientation;
    let offset = map_parameters.offset;

    let mut continents_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    continents_fractal.ridge_builder(
        &mut random_number_generator.rng,
        15,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        1,
        2,
        orientation,
        offset,
    );

    let mut mountains_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    mountains_fractal.ridge_builder(
        &mut random_number_generator.rng,
        10,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        6,
        1,
        orientation,
        offset,
    );

    let mut hills_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    hills_fractal.ridge_builder(
        &mut random_number_generator.rng,
        15,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        1,
        2,
        orientation,
        offset,
    );

    let [water_threshold] = continents_fractal.get_height_from_percents(&[water_percent])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    let [pass_threshold, hills_bottom1, hills_top1, hills_bottom2, hills_top2] = hills_fractal
        .get_height_from_percents(&[
            hills_near_mountains,
            hills_bottom1,
            hills_top1,
            hills_bottom2,
            hills_top2,
        ])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    let [mountain_threshold, hills_near_mountains, hills_clumps, mountain_100, mountain_99, mountain_98, mountain_97, mountain_95] =
        mountains_fractal.get_height_from_percents(&[
            mountains,
            hills_near_mountains,
            hills_clumps,
            100,
            99,
            98,
            97,
            95,
        ])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    tile_storage
        .tiles
        .iter()
        .enumerate()
        .for_each(|(index, tile)| {
            let [x, y] = map_parameters.index_to_offset_coordinate(index).to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);
            let mut entity_commands = commands.entity(*tile);

            if height <= water_threshold {
                entity_commands.insert(TerrainType::Water);
                if tectonic_islands {
                    if mountain_height == mountain_100 {
                        entity_commands.insert(TerrainType::Mountain);
                    } else if mountain_height == mountain_99 {
                        entity_commands.insert(TerrainType::Hill);
                    } else if (mountain_height == mountain_97) || (mountain_height == mountain_95) {
                        entity_commands.insert(TerrainType::Flatland);
                    }
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    entity_commands.insert(TerrainType::Hill);
                } else {
                    entity_commands.insert(TerrainType::Mountain);
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                entity_commands.insert(TerrainType::Hill);
            } else {
                entity_commands.insert(TerrainType::Flatland);
            };
        });
}

pub fn generate_terrain_type_for_pangaea(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    mut random_number_generator: ResMut<RandomNumberGenerator>,
    tile_storage: Res<TileStorage>,
) {
    let continent_grain = 2;

    let sea_level_low = 71;
    let sea_level_normal = 78;
    let sea_level_high = 84;
    let world_age_old = 2;
    let world_age_normal = 3;
    let world_age_new = 5;

    let extra_mountains = 0;

    let adjustment = match map_parameters.world_age {
        WorldAge::Old => world_age_old,
        WorldAge::Normal => world_age_normal,
        WorldAge::New => world_age_new,
    };

    let mountains = 97 - adjustment - extra_mountains;
    let hills_near_mountains = 91 - (adjustment * 2) - extra_mountains;
    let hills_bottom1 = 28 - adjustment;
    let hills_top1 = 28 + adjustment;
    let hills_bottom2 = 72 - adjustment;
    let hills_top2 = 72 + adjustment;
    let hills_clumps = 1 + adjustment;

    let water_percent = match map_parameters.sea_level {
        SeaLevel::Low => sea_level_low,
        SeaLevel::Normal => sea_level_normal,
        SeaLevel::High => sea_level_high,
        SeaLevel::Random => {
            sea_level_low
                + random_number_generator
                    .rng
                    .gen_range(0..=(sea_level_high - sea_level_low))
        }
    };

    let orientation = map_parameters.hex_layout.orientation;
    let offset = map_parameters.offset;

    let mut continents_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    continents_fractal.ridge_builder(
        &mut random_number_generator.rng,
        15,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        1,
        2,
        orientation,
        offset,
    );

    let mut mountains_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    mountains_fractal.ridge_builder(
        &mut random_number_generator.rng,
        10,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        6,
        1,
        orientation,
        offset,
    );

    let mut hills_fractal = CvFractal::create(
        &mut random_number_generator.rng,
        map_parameters.map_size.width,
        map_parameters.map_size.height,
        2,
        Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        7,
        6,
    );

    hills_fractal.ridge_builder(
        &mut random_number_generator.rng,
        15,
        &Flags {
            wrap_x: map_parameters.wrap_x,
            wrap_y: map_parameters.wrap_y,
            ..Default::default()
        },
        1,
        2,
        orientation,
        offset,
    );

    let [water_threshold] = continents_fractal.get_height_from_percents(&[water_percent])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    let [pass_threshold, hills_bottom1, hills_top1, hills_bottom2, hills_top2] = hills_fractal
        .get_height_from_percents(&[
            hills_near_mountains,
            hills_bottom1,
            hills_top1,
            hills_bottom2,
            hills_top2,
        ])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    let [mountain_threshold, hills_near_mountains, hills_clumps, mountain_100, mountain_99, mountain_98, mountain_97, mountain_95] =
        mountains_fractal.get_height_from_percents(&[
            mountains,
            hills_near_mountains,
            hills_clumps,
            100,
            99,
            98,
            97,
            95,
        ])[..]
    else {
        panic!("Vec length does not match the pattern")
    };

    let width = map_parameters.map_size.width;
    let height = map_parameters.map_size.height;
    let center_position = DVec2::new(width as f64 / 2., height as f64 / 2.);

    let axis = center_position * 3. / 5.;

    tile_storage
        .tiles
        .iter()
        .enumerate()
        .for_each(|(index, tile)| {
            let [x, y] = map_parameters.index_to_offset_coordinate(index).to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);

            let mut h = water_threshold as f64;

            let delta = IVec2::from([x, y]).as_dvec2() - center_position;
            let d = (delta / axis).length_squared();

            if d <= 1. {
                h = h + (h * 0.125)
            } else {
                h = h - (h * 0.125)
            }

            let height = ((height as f64 + h + h) * 0.33) as i32;

            let mut entity_commands = commands.entity(*tile);

            if height <= water_threshold {
                entity_commands.insert(TerrainType::Water);
                if height == mountain_100 {
                    entity_commands.insert(TerrainType::Mountain);
                } else if height == mountain_99 {
                    entity_commands.insert(TerrainType::Hill);
                } else if height == mountain_97 || height == mountain_95 {
                    entity_commands.insert(TerrainType::Flatland);
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    entity_commands.insert(TerrainType::Hill);
                } else {
                    entity_commands.insert(TerrainType::Mountain);
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                entity_commands.insert(TerrainType::Hill);
            } else {
                entity_commands.insert(TerrainType::Flatland);
            };
        });
}
