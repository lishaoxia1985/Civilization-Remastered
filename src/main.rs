mod assets;

use std::{f32::consts::FRAC_PI_2, sync::Arc};

use bevy_asset_loader::loading_state::{
    config::ConfigureLoadingState, LoadingState, LoadingStateAppExt,
};

use enum_map::{enum_map, EnumMap};

use civ_map_generator::{
    component::map_component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    generate_map,
    grid::{
        direction::Direction, hex_grid::{
            hex::{HexLayout, HexOrientation, Offset},
            HexGrid,
        }, Grid, GridSize, Size, WorldSizeType, WrapFlags
    },
    map_parameters::{MapParameters, MapType, WorldGrid},
    ruleset::Ruleset,
    tile_map::TileMap,
};

use assets::{AppState, MaterialResource};
use bevy_prototype_lyon::{
    draw::Stroke, entity::ShapeBundle, path::PathBuilder, plugin::ShapePlugin,
    prelude::GeometryBuilder,
};

use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};

// use crate::ruleset::Unique;

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
        .add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::GameStart)
                .load_collection::<MaterialResource>(),
        )
        // .insert_resource(Ruleset::new())
        .insert_resource({
            let world_size_type = WorldSizeType::Huge;
            let grid = HexGrid {
                size: HexGrid::default_size(world_size_type),
                layout: HexLayout {
                    orientation: HexOrientation::Pointy,
                    size: Vec2::new(8., 8.),
                    origin: Vec2::new(0., 0.),
                },
                wrap_flags: WrapFlags::WrapX,
                offset: Offset::Odd,
            };

            //let world_grid = WorldGrid::new(grid, world_size);
            let world_grid = WorldGrid::from_grid(grid);
            let map_type = MapType::Pangaea;
            let map_parameters = MapParameters {
                world_grid,
                map_type,
                ..Default::default()
            };
            MapSetting(Arc::new(map_parameters))
        })
        .add_plugins(ShapePlugin)
        .add_systems(OnEnter(AppState::AssetLoading), camera_setup)
        .add_systems(
            Update,
            (
                camera_movement,
                cursor_drag_system,
                zoom_camera_system,
                show_tile_map.run_if(in_state(AppState::GameStart)),
            ),
        )
        .add_systems(OnEnter(AppState::GameStart), generate_tile_map)
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

fn camera_setup(mut commands: Commands, map_setting: Res<MapSetting>) {
    let map_parameters = &map_setting.0;
    let grid = map_parameters.world_grid.grid;
    let map_center = grid.center();
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(map_center.x, map_center.y, 0.0),
        ..Default::default()
    });
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let mut transform = query.single_mut();

    let mut movement = Vec3::ZERO;

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

    transform.translation += movement * time.delta_seconds() * 300.0;
}

fn cursor_drag_system(
    windows: Query<&Window>,
    mut cameras: Query<(&mut Transform, &Camera, &GlobalTransform)>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    input: Res<ButtonInput<MouseButton>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((mut transform, camera, global_transform)) = cameras.get_single_mut() else {
        return;
    };
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
    mut projection: Query<&mut OrthographicProjection, With<Camera>>,
) {
    let mut projection = projection.single_mut();
    for event in scroll_evr.read() {
        let zoom_factor = 1.0 + event.y * 0.1; // Zoom speed
        projection.scale *= zoom_factor;
    }

    // Handle keyboard zoom
    if keyboard_input.pressed(KeyCode::KeyQ) {
        projection.scale *= 1.01;
    }
    if keyboard_input.pressed(KeyCode::KeyE) {
        projection.scale *= 0.99;
    }

    // Restrict zoom range
    if projection.scale > 2.0 {
        projection.scale = 2.0;
    } else if projection.scale < 0.1 {
        projection.scale = 0.1;
    }
}

#[derive(Resource)]
struct MapGenerator(Task<TileMap>);

#[derive(Resource)]
struct MapSetting(Arc<MapParameters>);

fn generate_tile_map(mut commands: Commands, map_setting: Res<MapSetting>) {
    let map_parameters = Arc::clone(&map_setting.0);
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move {
        let ruleset = Ruleset::new();
        generate_map(&map_parameters, &ruleset)
    });
    commands.insert_resource(MapGenerator(task));
}

fn show_tile_map(
    mut commands: Commands,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    map_setting: Res<MapSetting>,
    task: Option<ResMut<MapGenerator>>,
) {
    let grid = map_setting.0.world_grid.grid;

    let tile_map;

    let Some(mut task) = task else {
        return;
    };

    if let Some(map) = block_on(future::poll_once(&mut task.0)) {
        tile_map = map;
        commands.remove_resource::<MapGenerator>();
    } else {
        return;
    }

    let base_terrain_and_texture_name = enum_map! {
        BaseTerrain::Ocean => "sv_terrainhexocean",
        BaseTerrain::Lake => "sv_terrainhexcoast",
        BaseTerrain::Coast => "sv_terrainhexcoast",
        BaseTerrain::Grassland => "sv_terrainhexgrasslands",
        BaseTerrain::Desert => "sv_terrainhexdesert",
        BaseTerrain::Plain => "sv_terrainhexplains",
        BaseTerrain::Tundra => "sv_terrainhextundra",
        BaseTerrain::Snow => "sv_terrainhexsnow",
    };

    let base_terrain_and_material: EnumMap<_, _> = base_terrain_and_texture_name
        .into_iter()
        .map(|(base_terrain, base_terrain_texture)| {
            (
                base_terrain,
                color_materials.add(materials.texture_handle(base_terrain_texture)),
            )
        })
        .collect();

    // Draw rivers
    tile_map.river_list.iter().for_each(|river| {
        let mut path_builder = PathBuilder::new();
        river
            .iter()
            .enumerate()
            .for_each(|(_index, (tile, flow_direction))| {
                let (first_point, second_point) = match grid.layout.orientation {
                    HexOrientation::Pointy => match *flow_direction {
                        Direction::North => (Direction::SouthEast, Direction::NorthEast),
                        Direction::NorthEast => (Direction::South, Direction::SouthEast),
                        Direction::East => panic!(),
                        Direction::SouthEast => (Direction::SouthWest, Direction::South),
                        Direction::South => (Direction::NorthEast, Direction::SouthEast),
                        Direction::SouthWest => (Direction::SouthEast, Direction::South),
                        Direction::West => panic!(),
                        Direction::NorthWest => (Direction::South, Direction::SouthWest),
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
                let first_point_position = tile.corner_position(first_point, grid);
                let second_point_position = tile.corner_position(second_point, grid);
                path_builder.move_to(first_point_position);
                path_builder.line_to(second_point_position);
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

    let tile_pixel_size = grid.layout.size * Vec2::new(2.0, 2.0);

    let (sprite_rotation, text_rotation) = match grid.layout.orientation {
        HexOrientation::Pointy => (Quat::default(), Quat::default()),
        HexOrientation::Flat => (
            Quat::from_rotation_z(FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-FRAC_PI_2 * 3.),
        ),
    };

    for tile in tile_map.iter_tiles() {
        let pixel_position = tile.pixel_position(grid);
        // Spawn the tile with base terrain
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(RegularPolygon::new(8.0, 6))),
                transform: Transform {
                    translation: Vec3::from((pixel_position, 0.)),
                    rotation: sprite_rotation,
                    ..Default::default()
                },
                material: base_terrain_and_material[tile.base_terrain(&tile_map)].clone(),
                ..default()
            })
            .with_children(|parent| {
                // Draw terrain type Mountain with no natural wonder and Hill
                // Notice terrain type Flatland and Water are not drawn in this moment because they only need to be drawn with base terrain
                if tile.terrain_type(&tile_map) == TerrainType::Mountain
                    && tile.natural_wonder(&tile_map).is_none()
                {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size),
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
                } else if tile.terrain_type(&tile_map) == TerrainType::Hill {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(tile_pixel_size),
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
                if let Some(feature) = tile.feature(&tile_map) {
                    let feature_texture = match feature.name() {
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
                            custom_size: Some(tile_pixel_size),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(feature_texture),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 2.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }

                // Draw the natural wonder
                if let Some(natural_wonder) = tile.natural_wonder(&tile_map) {
                    let natural_wonder_texture = match natural_wonder.name() {
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
                            custom_size: Some(tile_pixel_size),
                            ..Default::default()
                        },
                        texture: materials.texture_handle(natural_wonder_texture),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 2.),
                            rotation: text_rotation,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }

                // Draw the civilization
                tile_map.civilization_and_starting_tile.iter().for_each(
                    |(civilization, &starting_tile)| {
                        if starting_tile == tile {
                            parent.spawn(SpriteBundle {
                                sprite: Sprite {
                                    color: Color::BLACK,
                                    custom_size: Some(tile_pixel_size),
                                    ..Default::default()
                                },
                                texture: materials.texture_handle(civilization),
                                transform: Transform {
                                    translation: Vec3::new(0., 0., 3.),
                                    rotation: text_rotation,
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        }
                    },
                );

                // Draw the city state
                tile_map
                    .city_state_and_starting_tile
                    .iter()
                    .for_each(|(_, &starting_tile)| {
                        if starting_tile == tile {
                            parent.spawn(SpriteBundle {
                                sprite: Sprite {
                                    custom_size: Some(tile_pixel_size),
                                    ..Default::default()
                                },
                                texture: materials.texture_handle("CityState"),
                                transform: Transform {
                                    translation: Vec3::new(0., 0., 3.),
                                    rotation: text_rotation,
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        }
                    });
            });
    }
}
