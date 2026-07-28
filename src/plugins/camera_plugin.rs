//! 相机插件
//!
//! 管理主相机的初始化、移动、缩放和边界限制。

use std::collections::HashMap;

use bevy::{camera::visibility::RenderLayers, input::mouse::MouseWheel, prelude::*};
use civ_map_generator::{
    grid::{Grid, OffsetCoordinate, WrapFlags},
    tile::Tile,
};

use crate::{
    AppState, ScreenState,
    components::{MainCamera, WorldTile},
    resources::{MapParametersRes, TileEntityMap, TileMapRes},
};

/// 相机插件
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::AssetLoading), setup_main_camera)
            .add_systems(
                Update,
                (
                    main_camera_movement,
                    cursor_drag_system,
                    zoom_main_camera_system,
                    show_main_camera_area,
                )
                    .run_if(in_state(ScreenState::WorldMap)),
            )
            .add_systems(OnEnter(AppState::GameStart), move_camera_to_player_center);
    }
}

/// 设置主相机
fn setup_main_camera(mut commands: Commands, map_params: Res<MapParametersRes>) {
    let map_parameters = &map_params.0;
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

/// 主相机移动
fn main_camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Single<&mut Transform, With<MainCamera>>,
    map_params: Res<MapParametersRes>,
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

    limit_main_camera_within_map_bounds(&mut transform, &map_params);
}

/// 鼠标拖拽相机系统
fn cursor_drag_system(
    window: Single<&Window>,
    cameras: Single<(&mut Transform, &Camera, &GlobalTransform), With<MainCamera>>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    input: Res<ButtonInput<MouseButton>>,
    map_params: Res<MapParametersRes>,
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

    limit_main_camera_within_map_bounds(&mut transform, &map_params);
}

/// 主相机缩放系统
fn zoom_main_camera_system(
    mut scroll_evr: MessageReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    projection: Single<&mut Projection, With<MainCamera>>,
) {
    let mut projection = projection.into_inner();
    if let Projection::Orthographic(ref mut orthographic) = *projection {
        for event in scroll_evr.read() {
            let zoom_factor = 1.0 + event.y * 0.1;
            orthographic.scale *= zoom_factor;
        }

        if keyboard_input.pressed(KeyCode::KeyQ) {
            orthographic.scale *= 1.01;
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            orthographic.scale *= 0.99;
        }

        orthographic.scale = orthographic.scale.clamp(0.3, 1.67);
    }
}

/// 将相机移动到玩家起始位置
fn move_camera_to_player_center(
    mut query: Query<&mut Transform, With<MainCamera>>,
    tile_map: Res<TileMapRes>,
    civ_manager: Res<crate::resources::CivilizationManager>,
) {
    let grid = tile_map.0.world_grid.grid;
    let player = civ_manager.player_nation;
    let tile_and_civ = &tile_map.0.starting_tile_and_civilization;
    let tile = tile_and_civ
        .iter()
        .find(|&(_, &c)| c == player)
        .map(|(&tile, _)| tile)
        .unwrap();

    let offset_coordinate = tile.to_offset(grid);
    let player_position = grid.offset_to_pixel(offset_coordinate);

    for mut transform in query.iter_mut() {
        [transform.translation.x, transform.translation.y] = player_position;
    }
}

/// 限制主相机在地图范围内
fn limit_main_camera_within_map_bounds(transform: &mut Transform, map_params: &MapParametersRes) {
    let map_parameters = &map_params.0;
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

/// 根据主相机的可视区域，处理当地图WrapX和WrapY时的显示边界
///
/// Notes: 在小地图的瓦片上没有WorldTile组件，因此`query_world_tile`不会查询到小地图上的瓦片。
fn show_main_camera_area(
    query: Single<&mut Transform, With<MainCamera>>,
    tilemap: Option<Res<TileMapRes>>,
    tile_entity_map: Res<TileEntityMap>,
    mut query_world_tile: Query<&mut Transform, (With<WorldTile>, Without<MainCamera>)>,
) {
    let Some(tile_map) = tilemap else {
        return;
    };

    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;

    // If the grid is not wrapped both in x and y, we don't need to do anything
    if !grid.wrap_x() && !grid.wrap_y() {
        return;
    }

    const WIDTH_OF_VISIBLE_AREA: i32 = 37;
    const HEIGHT_OF_VISIBLE_AREA: i32 = 21;

    if grid.wrap_x() {
        assert!(WIDTH_OF_VISIBLE_AREA < grid.width() as i32,
        "In horizontal wrap mode, the visible area width MUST be strictly less than the grid width.\n
        IF visible area width >= grid width, unrendered black borders will appear at window edges,\n
        causing horizontal coordinate wrapping to fail (disconnected visual continuity).");
    }

    if grid.wrap_y() {
        assert!(HEIGHT_OF_VISIBLE_AREA < grid.height() as i32,
        "In vertical wrap mode, visible area height MUST be strictly less than the grid height.\n
        IF visible area height >= grid height, unrendered black borders will appear at top/bottom edges,\n
        causing horizontal coordinate wrapping to fail (disconnected visual continuity).");
    }

    let camera_position = query.into_inner().translation.truncate().to_array();
    let camera_offset_coordinate = grid.pixel_to_offset(camera_position).to_array();

    let (left_x, right_x) = if grid.wrap_x() {
        (
            camera_offset_coordinate[0] - WIDTH_OF_VISIBLE_AREA / 2,
            camera_offset_coordinate[0] + WIDTH_OF_VISIBLE_AREA / 2,
        )
    } else {
        (0, grid.width() as i32 - 1)
    };

    let (bottom_y, top_y) = if grid.wrap_y() {
        (
            camera_offset_coordinate[1] - HEIGHT_OF_VISIBLE_AREA / 2,
            camera_offset_coordinate[1] + HEIGHT_OF_VISIBLE_AREA / 2,
        )
    } else {
        (0, grid.height() as i32 - 1)
    };

    (left_x..=right_x)
        .flat_map(|x| (bottom_y..=top_y).map(move |y| OffsetCoordinate::new(x, y)))
        .for_each(|offset_coordinate| {
            let tile = Tile::from_offset(offset_coordinate, grid);
            let tile_entity = tile_entity_map.get(tile).expect("Can't find tile entity");
            if let Ok(mut transform) = query_world_tile.get_mut(tile_entity) {
                let pixel_position = grid.offset_to_pixel(offset_coordinate);
                transform.translation = Vec3::from((pixel_position[0], pixel_position[1], 0.));
            }
        });
}
