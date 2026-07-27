//! 相机插件
//!
//! 管理主相机的初始化、移动、缩放和边界限制。

use bevy::{camera::visibility::RenderLayers, input::mouse::MouseWheel, prelude::*};
use civ_map_generator::grid::{Grid, WrapFlags};

use crate::{
    components::MainCamera,
    resources::{MapParametersRes, TileMapRes},
};

/// 相机插件
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::assets::AppState::AssetLoading),
            setup_main_camera,
        )
        .add_systems(
            Update,
            (
                main_camera_movement,
                cursor_drag_system,
                zoom_main_camera_system,
            )
                .in_set(crate::resources::GameSystemGroup::PlayOnWorldMap),
        )
        .add_systems(
            OnEnter(crate::assets::AppState::GameStart),
            move_camera_to_player_center,
        );
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
