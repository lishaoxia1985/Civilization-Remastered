mod assets;

mod map;
mod ruleset;
mod tile_map;

use std::time::{SystemTime, UNIX_EPOCH};

use assets::{check_textures, load_textures, setup, AppState, MaterialResource};
use bevy_prototype_lyon::prelude::*;
use map::{
    add_features, add_lakes, add_rivers, expand_coast, generate_coast_and_ocean,
    generate_empty_map, generate_lake, generate_natural_wonder, generate_terrain,
    generate_terrain_type_for_fractal, reassign_area_id, recalculate_areas, regenerate_coast,
    RandomNumberGenerator, River, TileQuery,
};
use rand::{rngs::StdRng, SeedableRng};
use ruleset::Ruleset;
use tile_map::{
    hex::{Direction, HexOrientation, Offset},
    HexLayout, MapParameters, MapSize, TerrainType,
};

use bevy::{math::DVec2, prelude::*, utils::HashMap};

use crate::ruleset::Unique;

fn main() {
    /* let ruleset = Ruleset::new();
    dbg!(&ruleset.natural_wonders);
    let mut unique_list = Vec::new();
    for terrains in ruleset.natural_wonders.values() {
        for unique in terrains.uniques.iter() {
            if !unique_list.contains(unique) {
                unique_list.push(unique.to_owned())
            }
        }
    }
    dbg!(unique_list);
    let unique_objects =
        Unique::new("[-33]% Strength <for [All] units> <when below [-10] Happiness>");
    dbg!(unique_objects); */
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Civilization-Remastered".to_owned(),
                resolution: (800., 600.).into(),
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .init_resource::<MaterialResource>()
        .insert_resource(Ruleset::new())
        .insert_resource(TileStorage { tiles: Vec::new() })
        .insert_resource(River(HashMap::new()))
        .insert_resource(RandomNumberGenerator {
            rng: StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    .try_into()
                    .unwrap(),
            ),
        })
        .insert_resource({
            let mut map_parameters = MapParameters {
                map_size: MapSize {
                    width: 80,
                    height: 40,
                },
                hex_layout: HexLayout {
                    orientation: HexOrientation::Pointy,
                    size: DVec2::new(8., 8.),
                    origin: DVec2::new(0., 0.),
                },
                offset: Offset::Odd,
                ..Default::default()
            };
            map_parameters.update_origin();
            map_parameters
        })
        .add_plugins(ShapePlugin)
        .add_systems(OnEnter(AppState::Setup), (load_textures, camera_setup))
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnEnter(AppState::Finished), setup)
        .add_systems(
            OnEnter(AppState::GameStart),
            (
                close_on_esc,
                generate_empty_map,
                generate_terrain_type_for_fractal,
                ((generate_coast_and_ocean, expand_coast).chain()),
                (recalculate_areas, reassign_area_id).chain(),
                (generate_lake, generate_terrain),
                add_rivers,
                add_lakes,
                (recalculate_areas, reassign_area_id).chain(),
                add_features,
                generate_natural_wonder,
                regenerate_coast,
                (recalculate_areas, reassign_area_id).chain(),
                show_tiles_system,
            )
                .chain(),
        )
        .run();
}

pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

fn camera_setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Resource)]
pub struct TileStorage {
    tiles: Vec<Entity>,
}

fn show_tiles_system(
    mut commands: Commands,
    materials: Res<MaterialResource>,
    map_parameters: Res<MapParameters>,
    river: Res<River>,
    query_tile: Query<TileQuery>,
) {
    river.0.values().for_each(|river| {
        let mut path_builder = PathBuilder::new();
        river
            .iter()
            .enumerate()
            .for_each(|(index, (tile_index, flow_direction))| {
                let hex_position = query_tile.get(*tile_index).unwrap().hex_position;

                let (first_point, second_point) = match map_parameters.hex_layout.orientation {
                    HexOrientation::Pointy => match *flow_direction {
                        Direction::North => (Direction::SouthEast, Direction::NorthEast),
                        Direction::NorthEast => (Direction::South, Direction::SouthEast),
                        Direction::East => panic!(),
                        Direction::SouthEast => (Direction::SouthWest, Direction::South),
                        Direction::South => (Direction::NorthEast, Direction::SouthEast),
                        Direction::SouthWest => (Direction::SouthEast, Direction::South),
                        Direction::West => panic!(),
                        Direction::NorthWest => (Direction::South, Direction::SouthWest),
                        Direction::None => panic!(),
                    },
                    HexOrientation::Flat => match *flow_direction {
                        Direction::North => panic!(),
                        Direction::NorthEast => (Direction::SouthEast, Direction::East),
                        Direction::East => (Direction::SouthWest, Direction::SouthEast),
                        Direction::SouthEast => (Direction::NorthEast, Direction::East),
                        Direction::South => panic!(),
                        Direction::SouthWest => (Direction::East, Direction::SouthEast),
                        Direction::West => (Direction::SouthEast, Direction::SouthWest),
                        Direction::NorthWest => (Direction::East, Direction::NorthEast),
                        Direction::None => panic!(),
                    },
                };

                if index == 0 {
                    let first_point_position =
                        hex_position.corner_position(first_point, &map_parameters);
                    let second_point_position =
                        hex_position.corner_position(second_point, &map_parameters);
                    path_builder.move_to(first_point_position.as_vec2());
                    path_builder.line_to(second_point_position.as_vec2());
                } else {
                    let second_point_position =
                        hex_position.corner_position(second_point, &map_parameters);
                    path_builder.line_to(second_point_position.as_vec2());
                }
            });

        let path = path_builder.build();

        commands.spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&path),
                spatial: SpatialBundle {
                    transform: Transform::from_xyz(0., 0., 10.),
                    ..default()
                },
                ..default()
            },
            Stroke::new(Color::BLACK, 2.0),
        ));
    });

    let tile_pixel_size = map_parameters.hex_layout.size * DVec2::new(2.0, 3_f64.sqrt());

    let (sprite_rotation, text_rotation) = match map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => (
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * 3.),
        ),
        HexOrientation::Flat => (Quat::default(), Quat::default()),
    };

    for tile in query_tile.iter() {
        let mut terrain = match tile.terrain_type {
            TerrainType::Water => "Ocean".to_owned(),
            TerrainType::Flatland => "Grassland".to_owned(),
            TerrainType::Mountain => "Mountain".to_owned(),
            TerrainType::Hill => "Hill".to_owned(),
        };

        let terrain = if tile.terrain_type == &TerrainType::Mountain {
            "Mountain".to_owned()
        } else if tile.terrain_type == &TerrainType::Hill {
            format!("{}+Hill", &tile.base_terrain.name())
        } else {
            tile.base_terrain.name().to_owned()
        };

        let pixel_position = tile.hex_position.pixel_position(&map_parameters);
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(tile_pixel_size.as_vec2()),
                    ..Default::default()
                },
                texture: materials.texture_handle(&terrain),
                transform: Transform {
                    translation: Vec3::from((pixel_position.as_vec2(), 1.)),
                    rotation: sprite_rotation,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                // Draw the feature
                if let Some(feature) = tile.feature {
                    let feature_name = match feature.name() {
                        "Forest" => "ForestG",
                        "Jungle" => "JungleG",
                        _ => &feature.name(),
                    };

                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(feature_name),
                        transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
                        ..Default::default()
                    });
                }

                // Draw the natural wonder
                if let Some(natural_wonder) = tile.natural_wonder {
                    let natural_wonder_name = natural_wonder.name();

                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(natural_wonder_name),
                        transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
                        ..Default::default()
                    });
                }
            });
    }
}
