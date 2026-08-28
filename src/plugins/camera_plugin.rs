//! 相机插件
//!
//! 管理主相机的初始化、移动、缩放和边界限制。

use bevy::{camera::visibility::RenderLayers, input::mouse::MouseWheel, prelude::*};
use civ_map_generator::{
    grid::{Grid, OffsetCoordinate, WrapFlags},
    tile::Tile,
};

use crate::{
    AppState, NationComponent, Player, ScreenState, TurnManager, TurnPhase,
    components::{MainCamera, WorldTile},
    resources::{MapParametersRes, TileEntityMap, TileMapRes},
};

/// 相机角度状态资源
#[derive(Resource, Default)]
pub struct CameraAngle {
    /// 是否处于45度角视图
    pub is_angled: bool,
}

/// 相机插件
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::AssetLoading), setup_main_camera)
            .insert_resource(CameraAngle::default())
            .add_systems(
                Update,
                (
                    main_camera_movement,
                    zoom_main_camera_system,
                    show_main_camera_area,
                    toggle_camera_angle,
                )
                    .run_if(in_state(ScreenState::WorldMap)),
            )
            .add_systems(Update, cursor_drag_system)
            .add_systems(OnEnter(TurnPhase::Player), move_camera_to_player_center);
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

/// 切换相机角度（空格键切换45度角俯视/正俯视图）
///
/// 在切换角度时，会调整相机的Y坐标以保证屏幕中心的地图位置保持不变。
fn toggle_camera_angle(
    input: Res<ButtonInput<KeyCode>>,
    mut camera_angle: ResMut<CameraAngle>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    if input.just_pressed(KeyCode::Space) {
        camera_angle.is_angled = !camera_angle.is_angled;

        const CAMERA_HEIGHT: f32 = 300.0;

        for mut transform in query.iter_mut() {
            if camera_angle.is_angled {
                // 切换到45度角俯视视图（绕X轴旋转45度）
                let angle = 45.0f32.to_radians();
                transform.rotation = Quat::from_rotation_x(angle);
                // 提升相机高度，以便在倾斜后仍能看到地图
                // 并向后移动Y，以保持屏幕中心的地图位置不变
                // 旋转后屏幕中心对应（x, y + z, 0），因此需要减去z来补偿
                transform.translation.y -= CAMERA_HEIGHT;
                transform.translation.z = CAMERA_HEIGHT;
            } else {
                // 重置为正俯视图（无旋转）
                transform.rotation = Quat::IDENTITY;
                // 恢复Y坐标补偿
                transform.translation.y += CAMERA_HEIGHT;
                // 恢复Z坐标为0
                transform.translation.z = 0.0;
            }
        }
    }
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
    screen_state: Option<Res<State<ScreenState>>>,
    map_params: Res<MapParametersRes>,
) {
    if let Some(screen_state) = screen_state {
        if screen_state.get() != &ScreenState::WorldMap {
            *last_cursor_pos = None;
            return;
        }
    }

    let (mut transform, camera, camera_transform) = cameras.into_inner();

    if input.pressed(MouseButton::Left) {
        if let Some(cursor_position) = window.cursor_position() {
            if let Some(last_screen_pos) = *last_cursor_pos {
                // 使用同一个 GlobalTransform 转换两个屏幕坐标，
                // 这样世界空间增量只由鼠标移动决定。
                if let Ok(current_world) =
                    camera.viewport_to_world_2d(camera_transform, cursor_position)
                    && let Ok(last_world) =
                        camera.viewport_to_world_2d(camera_transform, last_screen_pos)
                {
                    let delta = current_world - last_world;
                    transform.translation -= delta.extend(0.);
                }
            }

            // 每帧都更新为当前屏幕坐标
            *last_cursor_pos = Some(cursor_position);
        } else {
            // 光标移出窗口时清除，防止重新进入窗口时跳变
            *last_cursor_pos = None;
        }
    } else {
        *last_cursor_pos = None;
    }

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

/// 将主相机移动到玩家起始位置
///
/// TODO: 将主相机移动到玩家起始位置,后续可能修改为玩家的首都位置
///       另外如果游戏中只有一个玩家时，可能不需要移动主镜头
fn move_camera_to_player_center(
    mut query: Query<&mut Transform, With<MainCamera>>,
    tile_map: Res<TileMapRes>,
    player_query: Query<&NationComponent, With<Player>>,
    turn_manager: Option<Res<TurnManager>>,
) {
    let Some(turn_manager) = turn_manager else {
        return;
    };

    // 只在实际玩家回合时移动相机
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    // 如果当前回合的nation实体是玩家，获得玩家国家；否则退出
    let Ok(player) = player_query.get(current_entity) else {
        return;
    };

    let player_nation = player.0;

    let grid = tile_map.0.world_grid.grid;

    // TODO: 获取玩家起始位置,后续可能修改为玩家的首都位置
    //       另外如果游戏中只有一个玩家时，可能不需要移动主镜头
    let tile = tile_map
        .0
        .starting_tile_and_civilization
        .iter()
        .find(|&(_, &c)| c == player_nation)
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
    mut set: ParamSet<(
        Single<&mut Transform, (With<MainCamera>, Changed<Transform>)>,
        Query<&mut Transform, With<WorldTile>>,
    )>,
    tilemap: Option<Res<TileMapRes>>,
    tile_entity_map: Res<TileEntityMap>,
    mut previous_bounds: Local<Option<((i32, i32), (i32, i32))>>,
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
        causing vertical coordinate wrapping to fail (disconnected visual continuity).");
    }

    let camera_position = set.p0().into_inner().translation.truncate().to_array();
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

    let new_bounds = ((left_x, right_x), (bottom_y, top_y));

    // Only update tiles that are newly visible (outside previous bounds)
    let tiles_to_update = (left_x..=right_x)
        .flat_map(|x| (bottom_y..=top_y).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            if let Some(((old_left_x, old_right_x), (old_bottom_y, old_top_y))) = *previous_bounds {
                !(x >= old_left_x && x <= old_right_x && y >= old_bottom_y && y <= old_top_y)
            } else {
                true
            }
        })
        .map(|(x, y)| {
            let offset_coordinate = OffsetCoordinate::new(x, y);
            let tile = Tile::from_offset(offset_coordinate, grid);
            (offset_coordinate, tile)
        });

    for (offset_coordinate, tile) in tiles_to_update {
        let tile_entity = tile_entity_map.get(tile);
        if let Ok(mut transform) = set.p1().get_mut(tile_entity) {
            let pixel_position = grid.offset_to_pixel(offset_coordinate);
            transform.translation = Vec3::from((pixel_position[0], pixel_position[1], 0.));
        }
    }

    // Update bounds cache
    *previous_bounds = Some(new_bounds);
}
