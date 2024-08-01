mod assets;
mod ruleset;
mod tile_map;

use assets::{check_textures, load_textures, setup, AppState, MaterialResource};
use bevy_prototype_lyon::prelude::*;
use ruleset::Ruleset;
use tile_map::{
    hex::{Direction, HexOrientation, Offset},
    HexLayout, MapParameters, MapSize, TerrainType, TileMap,
};

use bevy::{math::DVec2, prelude::*};

use crate::ruleset::Unique;

fn main() {
    /* let ruleset = Ruleset::new();
    dbg!(&ruleset.terrains);
    let mut unique_list = Vec::new();
    for terrains in ruleset.terrains.values() {
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
        .add_plugins(ShapePlugin)
        .add_systems(OnEnter(AppState::Setup), (load_textures, camera_setup))
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnEnter(AppState::Finished), setup)
        .add_systems(
            OnEnter(AppState::GameStart),
            (close_on_esc, start_up_system),
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

fn start_up_system(
    mut commands: Commands,
    materials: Res<MaterialResource>,
    ruleset: Res<Ruleset>,
) {
    let mut tile_map = TileMap::new(
        MapParameters {
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
        },
        &ruleset,
    );
    let tile_pixel_size = tile_map.map_parameters.hex_layout.size * DVec2::new(2.0, 3_f64.sqrt());
    tile_map.spawn_tile_type_for_pangaea(&ruleset);
    //tile_map.spawn_tile_type_for_fractal(&ruleset);
    tile_map.generate_terrain(&ruleset);
    tile_map.generate_coasts(&ruleset);
    tile_map.generate_lakes(&ruleset);
    tile_map.recalculate_areas();
    tile_map.add_rivers();
    tile_map.add_lakes(&ruleset);
    tile_map.add_features(&ruleset);
    tile_map.natural_wonder_generator(&ruleset);
    tile_map.recalculate_areas();

    let width = tile_map.map_parameters.map_size.width;
    let height = tile_map.map_parameters.map_size.height;

    let (min_offset_x, min_offset_y) = [0, 1, width].into_iter().fold(
        (0.0_f64, 0.0_f64),
        |(min_offset_x, min_offset_y), index| {
            let tile = &tile_map.tile_list[index as usize];
            let [offset_x, offset_y] = tile
                .pixel_position(tile_map.map_parameters.hex_layout)
                .to_array();
            (min_offset_x.min(offset_x), min_offset_y.min(offset_y))
        },
    );

    let (max_offset_x, max_offset_y) = [
        width * (height - 1) - 1,
        width * height - 2,
        width * height - 1,
    ]
    .into_iter()
    .fold((0.0_f64, 0.0_f64), |(max_offset_x, max_offset_y), index| {
        let tile = &tile_map.tile_list[index as usize];
        let [offset_x, offset_y] = tile
            .pixel_position(tile_map.map_parameters.hex_layout)
            .to_array();
        (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
    });

    tile_map.map_parameters.hex_layout.origin =
        -(DVec2::new(min_offset_x, min_offset_y) + DVec2::new(max_offset_x, max_offset_y)) / 2.;

    tile_map.river_list.values().for_each(|river| {
        let mut path_builder = PathBuilder::new();
        river
            .iter()
            .enumerate()
            .for_each(|(index, (tile_index, flow_direction))| {
                let tile = &tile_map.tile_list[*tile_index];
                let (first_point, second_point) =
                    match tile_map.map_parameters.hex_layout.orientation {
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
                    let first_point_position = tile.tile_corner_position(first_point, &tile_map);
                    let second_point_position = tile.tile_corner_position(second_point, &tile_map);
                    path_builder.move_to(first_point_position.as_vec2());
                    path_builder.line_to(second_point_position.as_vec2());
                } else {
                    let second_point_position = tile.tile_corner_position(second_point, &tile_map);
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

    let (sprite_rotation, text_rotation) = match tile_map.map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => (
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * 3.),
        ),
        HexOrientation::Flat => (Quat::default(), Quat::default()),
    };

    for tile in tile_map.tile_list.iter() {
        let pixel_position = tile.pixel_position(tile_map.map_parameters.hex_layout);

        let terrain = if tile.terrain_type == TerrainType::Mountain {
            "Mountain".to_owned()
        } else if tile.terrain_type == TerrainType::Hill {
            format!("{}+Hill", &tile.base_terrain.name)
        } else {
            tile.base_terrain.name.to_owned()
        };

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
                if let Some(terrain_feature) = &tile.terrain_feature {
                    let terrain_feature_name = match terrain_feature.name.as_str() {
                        "Forest" => "ForestG",
                        "Jungle" => "JungleG",
                        _ => &terrain_feature.name,
                    };

                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(terrain_feature_name),
                        transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
                        ..Default::default()
                    });
                }
            });
    }
}
