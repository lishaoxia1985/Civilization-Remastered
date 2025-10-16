use std::{collections::HashMap, f32::consts::FRAC_PI_2, sync::Arc};

use bevy_asset_loader::loading_state::{
    config::ConfigureLoadingState, LoadingState, LoadingStateAppExt,
};

use enum_map::{enum_map, EnumMap};

use civ_map_generator::{
    generate_map,
    grid::{
        hex_grid::{Hex, HexGrid, HexLayout, HexOrientation, Offset},
        offset_coordinate::OffsetCoordinate,
        Grid, GridSize, WorldSizeType, WrapFlags,
    },
    map_parameters::{MapParameters, MapType, WorldGrid},
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{BaseTerrain, Feature, NaturalWonder, TerrainType},
    tile_map::{RiverEdge, TileMap},
};

use assets::{AppState, MaterialResource};

use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
    utils::HashSet,
};

mod assets;

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
                    size: [50., 50.],
                    origin: [0., 0.],
                },
                wrap_flags: WrapFlags::WrapX,
                offset: Offset::Odd,
            };
            let world_grid = WorldGrid::from_grid(grid);
            let map_parameters = MapParameters {
                world_grid,
                map_type: MapType::Fractal,
                ..Default::default()
            };
            MapSetting(Arc::new(map_parameters))
        })
        .add_systems(OnEnter(AppState::AssetLoading), camera_setup)
        .add_systems(
            Update,
            (
                camera_movement,
                cursor_drag_system,
                zoom_camera_system,
                wrap_tile_map.run_if(in_state(AppState::GameStart)),
                check_map_generate_status.run_if(in_state(AppState::GameStart)),
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
    map_setting: Res<MapSetting>,
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

    // limit the camera movement within the map boundary
    let map_parameters = &map_setting.0;
    let grid = &map_parameters.world_grid.grid;
    let left_bottom = grid.left_bottom();
    let right_top = grid.right_top();

    if !grid.wrap_flags.contains(WrapFlags::WrapX) {
        transform.translation.x = transform.translation.x.clamp(left_bottom[0], right_top[0]);
    }

    if !grid.wrap_flags.contains(WrapFlags::WrapY) {
        transform.translation.y = transform.translation.y.clamp(left_bottom[1], right_top[1]);
    }
}

fn cursor_drag_system(
    windows: Query<&Window>,
    mut cameras: Query<(&mut Transform, &Camera, &GlobalTransform)>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    input: Res<ButtonInput<MouseButton>>,
    map_setting: Res<MapSetting>,
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

    // limit the camera movement within the map boundary
    let map_parameters = &map_setting.0;
    let grid = &map_parameters.world_grid.grid;
    let left_bottom = grid.left_bottom();
    let right_top = grid.right_top();

    if !grid.wrap_flags.contains(WrapFlags::WrapX) {
        transform.translation.x = transform.translation.x.clamp(left_bottom[0], right_top[0]);
    }

    if !grid.wrap_flags.contains(WrapFlags::WrapY) {
        transform.translation.y = transform.translation.y.clamp(left_bottom[1], right_top[1]);
    }
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
    projection.scale = projection.scale.clamp(0.3, 1.67);
}

#[derive(Resource)]
struct MapGenerator(Task<TileMap>);

#[derive(Resource)]
struct TileMapResource(TileMap);

#[allow(dead_code)]
#[derive(Component)]
struct MapTile(Tile);

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

fn check_map_generate_status(mut commands: Commands, task: Option<ResMut<MapGenerator>>) {
    let Some(mut task) = task else {
        return;
    };

    if let Some(tile_map) = block_on(future::poll_once(&mut task.0)) {
        let map = TileMapResource(tile_map);
        commands.insert_resource(map);
        commands.remove_resource::<MapGenerator>();
    } else {
        return;
    }
}

fn wrap_tile_map(
    mut commands: Commands,
    mut query: Query<&mut Transform, With<Camera>>,
    map: Option<Res<TileMapResource>>,
    materials: Res<MaterialResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut exist_entity_and_offset_coordinates: Local<Vec<(Entity, OffsetCoordinate)>>,
) {
    if map.is_none() {
        return;
    };

    let tile_map = &map.unwrap().0;

    let grid = tile_map.world_grid.grid;

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

    let mut tile_and_river_flow_direction = HashMap::new();

    tile_map.river_list.iter().flatten().for_each(|river_edge| {
        tile_and_river_flow_direction
            .entry(river_edge.tile)
            .or_insert_with(Vec::new)
            .push(river_edge.flow_direction);
    });

    let all_possible_river_edge_mesh: Vec<_> = grid
        .corner_direction_array()
        .iter()
        .map(|&flow_direction| {
            let river_edge = RiverEdge {
                tile: Tile::new(0),
                flow_direction,
            };

            let [start_corner_direction, end_corner_direction] =
                river_edge.start_and_end_corner_directions(grid);
            let start_corner_position = grid.layout.corner(Hex::new(0, 0), start_corner_direction);
            let end_corner_position = grid.layout.corner(Hex::new(0, 0), end_corner_direction);

            let start = [start_corner_position[0], start_corner_position[1], 0.0];
            let end = [end_corner_position[0], end_corner_position[1], 0.0];
            let line_mesh = line_mesh(start.into(), end.into(), 1.5);
            (flow_direction, line_mesh)
        })
        .collect();

    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);

    // We only need to rotate the sprite for `Feature::Ice` because it was originally designed exclusively for Pointy-oriented hexagons.
    // Other terrain sprites were created to work seamlessly with both Pointy and Flat hexagon orientations.
    let sprite_rotation = match grid.layout.orientation {
        HexOrientation::Pointy => Quat::default(),
        HexOrientation::Flat => Quat::from_rotation_z(FRAC_PI_2 * 3.),
    };

    let layout = &grid.layout;
    let orientation = layout.orientation;
    let offset = grid.offset;

    // (1 + offset_x * 2) should < grid's width
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    let offset_x = 18;
    assert!(1 + offset_x * 2 < grid.width() as i32);
    // (1 + offset_y * 2) should < grid's height
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    let offset_y = 10;
    assert!(1 + offset_y * 2 < grid.height() as i32);

    let transform = query.single_mut();
    let camera_position = transform.translation.truncate().to_array();
    let offset_coordinate = layout
        .pixel_to_hex(camera_position)
        .to_offset(orientation, offset)
        .to_array();
    let mut left_x = offset_coordinate[0] - offset_x;
    let mut right_x = offset_coordinate[0] + offset_x;
    if !grid.wrap_x() {
        left_x = left_x.max(0);
        right_x = right_x.min(grid.width() as i32 - 1);
    }
    let mut bottom_y = offset_coordinate[1] - offset_y;
    let mut top_y = offset_coordinate[1] + offset_y;
    if !grid.wrap_y() {
        bottom_y = bottom_y.max(0);
        top_y = top_y.min(grid.height() as i32 - 1);
    }

    let mut offset_list = (left_x..=right_x)
        .flat_map(move |x| (bottom_y..=top_y).map(move |y| OffsetCoordinate::new(x, y)))
        .collect::<HashSet<_>>();

    // Despawn the tiles that are out of the current viewport
    // And remove the tiles that are still in the current viewport from offset_list,
    // we only need to spawn the tiles that can't be found in the current viewport later.
    exist_entity_and_offset_coordinates.retain(|(entity, map_offset)| {
        if !offset_list.contains(map_offset) {
            commands.entity(*entity).despawn_recursive();
            false
        } else {
            offset_list.remove(map_offset);
            true
        }
    });

    for &offset_coordinate in offset_list.iter() {
        let hex = Hex::from_offset(offset_coordinate, orientation, offset);
        let pixel_position = layout.hex_to_pixel(hex);
        let tile = Tile::from_offset(offset_coordinate, grid);
        // Spawn the tile with base terrain
        let parent = commands
            .spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(hex_mesh(&grid))),
                transform: Transform {
                    translation: Vec3::from((pixel_position[0], pixel_position[1], 0.)),
                    ..Default::default()
                },
                material: base_terrain_and_material[tile.base_terrain(&tile_map)].clone(),
                ..default()
            })
            .insert(MapTile(tile))
            .id();

        exist_entity_and_offset_coordinates.push((parent, offset_coordinate));

        commands.entity(parent).with_children(|parent| {
            // Draw river edges
            if let Some(flow_direction_list) = tile_and_river_flow_direction.get(&tile) {
                flow_direction_list.iter().for_each(|direction| {
                    let (_, line_mesh) = all_possible_river_edge_mesh
                        .iter()
                        .find(|(d, _)| d == direction)
                        .unwrap();
                    parent.spawn(MaterialMesh2dBundle {
                        mesh: Mesh2dHandle(meshes.add(line_mesh.clone())),
                        material: color_materials
                            .add(ColorMaterial::from_color(Color::srgb_u8(140, 215, 215))),
                        transform: Transform {
                            translation: Vec3::new(0., 0., 5.),
                            ..Default::default()
                        },
                        ..default()
                    });
                })
            };

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
                        rotation: if feature == Feature::Ice {
                            sprite_rotation
                        } else {
                            Quat::default()
                        },
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
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                    }
                });
        });
    }
}

fn line_mesh(start: Vec3, end: Vec3, width: f32) -> Mesh {
    // Calculate direction vector from start to end points
    let direction = end - start;
    let _length = direction.length();
    let normalized_direction = direction.normalize();

    // Compute perpendicular vector to create the line width
    let perpendicular =
        Vec3::new(-normalized_direction.y, normalized_direction.x, 0.0).normalize() * width / 2.0;

    // Create four vertices for the rectangle representing the line
    let vertices = vec![
        start + perpendicular,
        start - perpendicular,
        end + perpendicular,
        end - perpendicular,
    ];

    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

    let indices = Indices::U32(vec![0, 1, 2, 2, 1, 3]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.with_inserted_indices(indices)
}

fn hex_mesh(grid: &HexGrid) -> Mesh {
    let hex_layout = &grid.layout;
    let vertices: Vec<[f32; 3]> = hex_layout
        .all_corners(Hex::new(0, 0))
        .map(|corner| [corner[0], corner[1], 0.0])
        .to_vec();

    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.with_inserted_indices(indices)
}
