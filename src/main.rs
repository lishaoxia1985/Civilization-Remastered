mod assets;

use std::{f32::consts::FRAC_PI_2, sync::Arc};

use bevy_asset_loader::loading_state::{
    config::ConfigureLoadingState, LoadingState, LoadingStateAppExt,
};

use enum_map::{enum_map, EnumMap};

use civ_map_generator::{
    generate_map,
    grid::{
        hex_grid::{
            hex::{HexLayout, HexOrientation, Offset},
            HexGrid,
        },
        Grid, GridSize, WorldSizeType, WrapFlags,
    },
    map_parameters::{MapParameters, MapType, WorldGrid},
    ruleset::Ruleset,
    tile_component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        terrain_type::TerrainType,
    },
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

fn main() {
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
                    size: [8., 8.],
                    origin: [0., 0.],
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
        transform: Transform::from_xyz(map_center[0], map_center[1], 0.0),
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
        river.iter().enumerate().for_each(|(_index, river_edge)| {
            // Transform the river flow direction into the directions of the first and second points in the tile
            let [first_point, second_point] = river_edge.start_and_end_corner_directions(grid);
            let first_point_position = river_edge.tile.corner_position(first_point, grid);
            let second_point_position = river_edge.tile.corner_position(second_point, grid);
            path_builder.move_to(first_point_position.into());
            path_builder.line_to(second_point_position.into());
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

    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);

    let (sprite_rotation, text_rotation) = match grid.layout.orientation {
        HexOrientation::Pointy => (Quat::default(), Quat::default()),
        HexOrientation::Flat => (
            Quat::from_rotation_z(FRAC_PI_2 * 3.),
            Quat::from_rotation_z(-FRAC_PI_2 * 3.),
        ),
    };

    for tile in tile_map.all_tiles() {
        let pixel_position = tile.pixel_position(grid);
        // Spawn the tile with base terrain
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(RegularPolygon::new(8.0, 6))),
                transform: Transform {
                    translation: Vec3::from((pixel_position[0], pixel_position[1], 0.)),
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
                    let feature_texture = match feature {
                        Feature::Forest => "sv_forest",
                        Feature::Jungle => "sv_jungle",
                        Feature::Marsh => "sv_marsh",
                        Feature::Floodplain => "sv_floodplains",
                        Feature::Ice => "sv_ice",
                        Feature::Oasis => "sv_oasis",
                        Feature::Atoll => "sv_atoll",
                        Feature::Fallout => "sv_fallout",
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
                    let natural_wonder_texture = match natural_wonder {
                        NaturalWonder::GreatBarrierReef => "sv_coralreef",
                        NaturalWonder::OldFaithful => "sv_geyser",
                        NaturalWonder::ElDorado => "sv_el_dorado",
                        NaturalWonder::FountainOfYouth => "sv_fountain_of_youth",
                        NaturalWonder::GrandMesa => "sv_mesa",
                        NaturalWonder::MountFuji => "sv_fuji",
                        NaturalWonder::Krakatoa => "sv_krakatoa",
                        NaturalWonder::RockOfGibraltar => "sv_gibraltar",
                        NaturalWonder::CerroDePotosi => "sv_cerro_de_patosi",
                        NaturalWonder::BarringerCrater => "sv_crater",
                        NaturalWonder::MountKailash => "sv_mount_kailash",
                        NaturalWonder::MountSinai => "sv_mount_sinai",
                        NaturalWonder::SriPada => "sv_sri_pada",
                        NaturalWonder::Uluru => "sv_uluru",
                        NaturalWonder::KingSolomonsMines => "sv_kingsolomonsmine",
                        NaturalWonder::LakeVictoria => "sv_lakevictoria",
                        NaturalWonder::MountKilimanjaro => "sv_mountkilimanjaro",
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
                tile_map.starting_tile_and_civilization.iter().for_each(
                    |(&starting_tile, civilization)| {
                        if starting_tile == tile {
                            parent.spawn(SpriteBundle {
                                sprite: Sprite {
                                    color: Color::BLACK,
                                    custom_size: Some(tile_pixel_size),
                                    ..Default::default()
                                },
                                texture: materials.texture_handle(civilization.as_str()),
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
                    .starting_tile_and_city_state
                    .iter()
                    .for_each(|(&starting_tile, _)| {
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
