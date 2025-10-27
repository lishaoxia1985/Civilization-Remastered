use std::{collections::HashSet, f32::consts::FRAC_PI_2, sync::Arc};

use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};

use enum_map::{EnumMap, enum_map};

use civ_map_generator::{
    grid::{
        Grid, GridSize, WorldSizeType, WrapFlags,
        hex_grid::{Hex, HexGrid, HexLayout, HexOrientation, Offset},
        offset_coordinate::OffsetCoordinate,
    },
    map_parameters::{MapParameters, MapType, WorldGrid},
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{BaseTerrain, Feature, TerrainType},
    tile_map::{RiverEdge, TileMap},
};

use assets::{AppState, MaterialResource};

use bevy::{
    camera::visibility::RenderLayers, input::mouse::MouseWheel, platform::collections::HashMap,
    prelude::*, window::WindowResolution,
};

use crate::{
    custom_mesh::{hex_mesh, line_mesh},
    generating_map::{check_map_generate_status, generate_tile_map},
    minimap::{minimap_fov_update, setup_minimap},
};

mod assets;
mod custom_mesh;
mod generating_map;
mod minimap;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Civilization-Remastered".to_owned(),
                resolution: WindowResolution::new(1280, 720),
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .init_state::<AppState>()
        .add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::GeneratingMap)
                .load_collection::<MaterialResource>(),
        )
        .insert_resource(RulesetResource(Arc::new(Ruleset::new())))
        .insert_resource({
            let world_size_type = WorldSizeType::Standard;
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
        .add_systems(OnEnter(AppState::AssetLoading), main_camera_setup)
        .add_systems(
            Update,
            (
                main_camera_movement,
                cursor_drag_system,
                zoom_main_camera_system,
                minimap_fov_update.run_if(in_state(AppState::GameStart)),
                setup_minimap.run_if(in_state(AppState::GameStart)),
                wrap_tile_map.run_if(in_state(AppState::GameStart)),
                check_map_generate_status.run_if(in_state(AppState::GeneratingMap)),
            ),
        )
        .add_systems(OnEnter(AppState::GeneratingMap), generate_tile_map)
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

#[derive(Resource)]
pub struct RulesetResource(Arc<Ruleset>);

#[derive(Component)]
struct MainCamera;

fn main_camera_setup(mut commands: Commands, map_setting: Res<MapSetting>) {
    let map_parameters = &map_setting.0;
    let grid = map_parameters.world_grid.grid;
    let map_center = grid.center();
    commands.spawn((
        Camera2d,
        Transform::from_xyz(map_center[0], map_center[1], 0.0),
        Msaa::Sample8,
        RenderLayers::layer(0),
        MainCamera,
    ));
}

fn main_camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Single<&mut Transform, With<MainCamera>>,
    map_setting: Res<MapSetting>,
) {
    let mut transform = query.into_inner();

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

    transform.translation += movement * time.delta_secs() * 300.0;

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
    window: Single<&Window>,
    cameras: Single<(&mut Transform, &Camera, &GlobalTransform), With<MainCamera>>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    input: Res<ButtonInput<MouseButton>>,
    map_setting: Res<MapSetting>,
) {
    let (mut transform, camera, camera_transform) = cameras.into_inner();
    if input.pressed(MouseButton::Left) {
        if let Some(cursor_position) = window.cursor_position()
            && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
        {
            if let Some(last_pos) = *last_cursor_pos {
                let delta = world_pos - last_pos;
                transform.translation -= delta.extend(0.);
            } else {
                *last_cursor_pos = Some(world_pos);
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

fn zoom_main_camera_system(
    mut scroll_evr: MessageReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    projection: Single<&mut Projection, With<MainCamera>>,
) {
    let mut projection = projection.into_inner();
    if let Projection::Orthographic(ref mut orthographic) = *projection {
        for event in scroll_evr.read() {
            let zoom_factor = 1.0 + event.y * 0.1; // Zoom speed
            orthographic.scale *= zoom_factor;
        }

        // Handle keyboard zoom
        if keyboard_input.pressed(KeyCode::KeyQ) {
            orthographic.scale *= 1.01;
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            orthographic.scale *= 0.99;
        }

        // Restrict zoom range
        orthographic.scale = orthographic.scale.clamp(0.3, 1.67);
    }
}

#[derive(Resource)]
struct TileMapResource(TileMap);

#[allow(dead_code)]
#[derive(Component)]
struct MapTile(Tile);

#[derive(Resource)]
struct MapSetting(Arc<MapParameters>);

fn wrap_tile_map(
    mut commands: Commands,
    query: Single<&mut Transform, With<MainCamera>>,
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

    let base_terrain_and_material: EnumMap<BaseTerrain, Handle<ColorMaterial>> = enum_map! {
        base_terrain => color_materials.add(materials.texture_handle(base_terrain.as_str())),
    };

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

    // (1 + offset_x * 2) should < grid's width
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    const OFFSET_X: i32 = 18;
    assert!(1 + OFFSET_X * 2 < grid.width() as i32);
    // (1 + offset_y * 2) should < grid's height
    // Because if it's not, the same tile will be drawn twice due to the grid's wrapping behavior.
    const OFFSET_Y: i32 = 10;
    assert!(1 + OFFSET_Y * 2 < grid.height() as i32);

    let camera_position = query.into_inner().translation.truncate().to_array();
    let camera_offset_coordinate = grid.pixel_to_offset(camera_position).to_array();
    let mut left_x = camera_offset_coordinate[0] - OFFSET_X;
    let mut right_x = camera_offset_coordinate[0] + OFFSET_X;
    if !grid.wrap_x() {
        left_x = left_x.max(0);
        right_x = right_x.min(grid.width() as i32 - 1);
    }
    let mut bottom_y = camera_offset_coordinate[1] - OFFSET_Y;
    let mut top_y = camera_offset_coordinate[1] + OFFSET_Y;
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
            commands.entity(*entity).despawn();
            false
        } else {
            offset_list.remove(map_offset);
            true
        }
    });

    let hex_mesh = meshes.add(hex_mesh(&grid));

    for &offset_coordinate in offset_list.iter() {
        let pixel_position = grid.offset_to_pixel(offset_coordinate);
        let tile = Tile::from_offset(offset_coordinate, grid);
        // Spawn the tile with base terrain
        let parent = commands
            .spawn((
                Mesh2d(hex_mesh.clone()),
                Transform {
                    translation: Vec3::from((pixel_position[0], pixel_position[1], 0.)),
                    ..Default::default()
                },
                MeshMaterial2d(base_terrain_and_material[tile.base_terrain(tile_map)].clone()),
            ))
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
                    parent.spawn((
                        Mesh2d(meshes.add(line_mesh.clone())),
                        MeshMaterial2d(
                            color_materials
                                .add(ColorMaterial::from_color(Color::srgb_u8(140, 215, 215))),
                        ),
                        Transform {
                            translation: Vec3::new(0., 0., 5.),
                            ..Default::default()
                        },
                    ));
                })
            };

            // Draw terrain type Mountain with no natural wonder and Hill
            // Notice terrain type Flatland and Water are not drawn in this moment because they only need to be drawn with base terrain
            let terrain_type = tile.terrain_type(tile_map);
            let is_mountain_without_wonder =
                terrain_type == TerrainType::Mountain && tile.natural_wonder(tile_map).is_none();

            if is_mountain_without_wonder || terrain_type == TerrainType::Hill {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(terrain_type.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 3.),
                        ..Default::default()
                    },
                ));
            }

            // Draw the feature
            if let Some(feature) = tile.feature(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(feature.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        rotation: if feature == Feature::Ice {
                            sprite_rotation
                        } else {
                            Quat::default()
                        },
                        ..Default::default()
                    },
                ));
            }

            // Draw the natural wonder
            if let Some(natural_wonder) = tile.natural_wonder(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(natural_wonder.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        ..Default::default()
                    },
                ));
            }

            // Draw the civilization
            tile_map.starting_tile_and_civilization.iter().for_each(
                |(&starting_tile, civilization)| {
                    if starting_tile == tile {
                        parent.spawn((
                            Sprite {
                                color: Color::BLACK,
                                image: materials.texture_handle(civilization.as_str()),
                                custom_size: Some(tile_pixel_size),
                                ..Default::default()
                            },
                            Transform {
                                translation: Vec3::new(0., 0., 3.),
                                ..Default::default()
                            },
                        ));
                    }
                },
            );

            // Draw the city state
            tile_map
                .starting_tile_and_city_state
                .iter()
                .for_each(|(&starting_tile, _)| {
                    if starting_tile == tile {
                        parent.spawn((
                            Sprite {
                                custom_size: Some(tile_pixel_size),
                                image: materials.texture_handle("CityState"),
                                ..Default::default()
                            },
                            Transform {
                                translation: Vec3::new(0., 0., 3.),
                                ..Default::default()
                            },
                        ));
                    }
                });
        });
    }
}
