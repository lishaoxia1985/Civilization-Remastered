mod assets;

mod grid;
mod map;
mod ruleset;
mod tile_map;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use assets::{check_textures, load_textures, setup, AppState, MaterialResource};
use bevy_prototype_lyon::{
    draw::Stroke, entity::ShapeBundle, path::PathBuilder, plugin::ShapePlugin,
    prelude::GeometryBuilder,
};
use grid::hex::{HexLayout, HexOrientation, Offset};
use grid::Direction;
use map::AreaIdAndSize;
use map::{
    add_features, add_lakes, add_rivers, expand_coast, generate_coast_and_ocean,
    generate_empty_map, generate_lake, generate_natural_wonder, generate_terrain,
    generate_terrain_type_for_fractal, reassign_area_id, recalculate_areas, regenerate_coast,
    terrain_type::TerrainType, RandomNumberGenerator, River, TileQuery, TileStorage,
};
use rand::{rngs::StdRng, SeedableRng};
use ruleset::Ruleset;
use tile_map::{MapParameters, MapSize};

use bevy::{
    input::mouse::MouseWheel,
    math::DVec2,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    utils::HashMap,
};

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
        .insert_resource(AreaIdAndSize(BTreeMap::new()))
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
                    size: DVec2::new(16., 16.),
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
        .add_systems(
            Update,
            (camera_movement, cursor_drag_system, zoom_camera_system),
        )
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

fn show_tiles_system(
    mut commands: Commands,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    map_parameters: Res<MapParameters>,
    river: Res<River>,
    query_tile: Query<TileQuery>,
) {
    let mut base_terrain_and_material = HashMap::new();

    base_terrain_and_material.insert(
        "Ocean",
        color_materials.add(materials.texture_handle("sv_terrainhexocean")),
    );

    // Lake use the same texture as Coast
    base_terrain_and_material.insert(
        "Lake",
        color_materials.add(materials.texture_handle("sv_terrainhexcoast")),
    );

    base_terrain_and_material.insert(
        "Coast",
        color_materials.add(materials.texture_handle("sv_terrainhexcoast")),
    );

    base_terrain_and_material.insert(
        "Grassland",
        color_materials.add(materials.texture_handle("sv_terrainhexgrasslands")),
    );

    base_terrain_and_material.insert(
        "Desert",
        color_materials.add(materials.texture_handle("sv_terrainhexdesert")),
    );

    base_terrain_and_material.insert(
        "Plain",
        color_materials.add(materials.texture_handle("sv_terrainhexplains")),
    );

    base_terrain_and_material.insert(
        "Tundra",
        color_materials.add(materials.texture_handle("sv_terrainhextundra")),
    );

    base_terrain_and_material.insert(
        "Snow",
        color_materials.add(materials.texture_handle("sv_terrainhexsnow")),
    );

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

                /* if index == 0 {
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
                } */
                let first_point_position =
                    hex_position.corner_position(first_point, &map_parameters);
                let second_point_position =
                    hex_position.corner_position(second_point, &map_parameters);
                path_builder.move_to(first_point_position.as_vec2());
                path_builder.line_to(second_point_position.as_vec2());
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
            Stroke::new(Color::srgb_u8(140, 215, 215), 2.0),
        ));
    });

    /* let tile_pixel_size = match map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => map_parameters.hex_layout.size * DVec2::new(3_f64.sqrt(), 2.0),
        HexOrientation::Flat => map_parameters.hex_layout.size * DVec2::new(2.0, 3_f64.sqrt()),
    }; */

    let tile_pixel_size = map_parameters.hex_layout.size * DVec2::new(2.0, 2.0);

    let (sprite_rotation, text_rotation) = match map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => (Quat::default(), Quat::default()),
        HexOrientation::Flat => (
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * 3.),
        ),
    };

    for tile in query_tile.iter() {
        let pixel_position = tile.hex_position.pixel_position(&map_parameters);
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(RegularPolygon::new(16.0, 6))),
                transform: Transform {
                    translation: Vec3::from((pixel_position.as_vec2(), 0.)),
                    rotation: sprite_rotation,
                    ..Default::default()
                },
                material: base_terrain_and_material
                    .get(&tile.base_terrain.name())
                    .unwrap()
                    .clone(),
                ..default()
            })
            .with_children(|parent| {
                if tile.terrain_type == &TerrainType::Mountain && tile.natural_wonder.is_none() {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle("sv_mountains"),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 3.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                } else if tile.terrain_type == &TerrainType::Hill {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle("sv_hills"),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 3.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }

                // Draw the feature
                if let Some(feature) = tile.feature {
                    let feature_name = match feature.name() {
                        "Forest" => "sv_forest",
                        "Jungle" => "sv_jungle",
                        "Marsh" => "sv_marsh",
                        "Floodplain" => "sv_floodplains",
                        "Ice" => "sv_ice",
                        "Oasis" => "sv_oasis",
                        "Atoll" => "sv_atoll",
                        "Fallout" => "sv_fallout",
                        _ => unreachable!(),
                    };

                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(feature_name),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 2.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }

                // Draw the natural wonder
                if let Some(natural_wonder) = tile.natural_wonder {
                    let natural_wonder_name = match natural_wonder.name() {
                        "Great Barrier Reef" => "sv_coralreef",
                        "Old Faithful" => "sv_geyser",
                        "El Dorado" => "sv_el_dorado",
                        "Fountain of Youth" => "sv_fountain_of_youth",
                        "Grand Mesa" => "sv_mesa",
                        "Mount Fuji" => "sv_fuji",
                        "Krakatoa" => "sv_krakatoa",
                        "Rock of Gibraltar" => "sv_gibraltar",
                        "Cerro de Potosi" => "sv_cerro_de_patosi",
                        "Barringer Crater" => "sv_crater",
                        "Mount Kailash" => "sv_mount_kailash",
                        "Mount Sinai" => "sv_mount_sinai",
                        "Sri Pada" => "sv_sri_pada",
                        "Uluru" => "sv_uluru",
                        "King Solomon's Mines" => "sv_kingsolomonsmine",
                        "Lake Victoria" => "sv_lakevictoria",
                        "Mount Kilimanjaro" => "sv_mountkilimanjaro",
                        _ => unreachable!(),
                    };

                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size.as_vec2()),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(natural_wonder_name),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 2.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
            });
    }
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in query.iter_mut() {
        let mut movement = Vec3::ZERO;

        // 相机移动
        if keyboard_input.pressed(KeyCode::KeyW) {
            movement.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement.x += 1.0;
        }

        // 处理相机移动
        transform.translation += movement * time.delta_seconds() * 300.0;
    }
}

fn cursor_drag_system(
    windows: Query<&Window>,
    mut cameras: Query<(&mut Transform, &Camera, &GlobalTransform)>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    input: Res<ButtonInput<MouseButton>>,
) {
    let window = windows.single();
    let (mut transform, camera, global_transform) = cameras.single_mut();
    if input.pressed(MouseButton::Left) {
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(global_transform, cursor))
        {
            if let Some(last_pos) = *last_cursor_pos {
                let delta = world_position - last_pos;
                transform.translation -= delta.extend(0.);
            } else {
                *last_cursor_pos = Some(world_position);
            }
        };
    } else {
        *last_cursor_pos = None;
    };
}

fn zoom_camera_system(
    mut scroll_evr: EventReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut transform = query.single_mut();
    for event in scroll_evr.read() {
        let zoom_factor = 1.0 + event.y * 0.1; // 调整缩放因子，这里0.1为缩放增量因子
        transform.scale *= zoom_factor;
    }

    // 处理缩放
    if keyboard_input.pressed(KeyCode::KeyQ) {
        transform.scale *= 1.01;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        transform.scale *= 0.99;
    }

    // 限制缩放范围
    if transform.scale.max_element() > 2.0 {
        transform.scale = Vec3::splat(2.0);
    } else if transform.scale.min_element() < 0.1 {
        transform.scale = Vec3::splat(0.1);
    }
}
