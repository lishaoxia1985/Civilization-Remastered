use std::sync::Arc;

use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};

use civ_map_generator::{
    grid::{
        Grid, GridSize, WorldSizeType, WrapFlags,
        hex_grid::{HexGrid, HexLayout, HexOrientation, Offset},
    },
    map_parameters::{MapParameters, MapType, WorldGrid},
    nation::Nation,
    ruleset::Ruleset,
    tile::Tile,
    tile_map::TileMap,
};

use assets::{AppState, MaterialResource};

use bevy::{
    camera::visibility::RenderLayers, input::mouse::MouseWheel, input_focus::InputFocus,
    prelude::*, sprite_render::Material2dPlugin, window::WindowResolution,
};

use crate::{
    custom_material::ColorReplaceMaterial,
    generating_map::{check_map_generate_status, generate_tile_map},
    minimap::{DefaultFovIndicatorSize, minimap_fov_update, setup_minimap},
    technology::setup_tech_button,
    world_map::setup_tile_map,
};

mod assets;
mod custom_material;
mod custom_mesh;
mod generating_map;
mod minimap;
mod technology;
mod world_map;

#[derive(Resource)]
pub struct RulesetResource(Arc<Ruleset>);

#[derive(Resource)]
struct MapSetting(Arc<MapParameters>);

#[derive(Resource)]
struct TileMapResource(TileMap);

struct MapUnit {
    name: String,
    owner: Nation,
    position: Tile,
    Attack: u32,
    Defense: u32,
    Movement: u32,
    Hp: u32,
    promotion: Vec<String>,
}
struct NationUnit {
    unit_list: Vec<MapUnit>,
}

const START_UNITS: [&str; 2] = ["Settler", "Warrior"];

fn main() {
    // Create ruleset resource
    let ruleset = Ruleset::new();
    let ruleset_resource = RulesetResource(Arc::new(ruleset));

    // Create map parameters resource
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
    let map_setting = MapSetting(Arc::new(map_parameters));

    // Create default fov indicator size resource
    let default_fov_indicator_size = DefaultFovIndicatorSize::default();

    // App setup
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
        .add_plugins(Material2dPlugin::<ColorReplaceMaterial>::default())
        .init_resource::<InputFocus>()
        .insert_resource(ruleset_resource)
        .insert_resource(map_setting)
        .insert_resource(default_fov_indicator_size)
        .init_state::<AppState>()
        .add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::MapGenerating)
                .load_collection::<MaterialResource>(),
        )
        .add_systems(OnEnter(AppState::AssetLoading), main_camera_setup)
        .add_systems(
            Update,
            (
                main_camera_movement,
                cursor_drag_system,
                zoom_main_camera_system,
                minimap_fov_update.run_if(in_state(AppState::GameStart)),
                setup_minimap.run_if(in_state(AppState::GameStart)),
                setup_tile_map.run_if(in_state(AppState::GameStart)),
                check_map_generate_status.run_if(in_state(AppState::MapGenerating)),
            ),
        )
        .add_systems(OnEnter(AppState::MapGenerating), generate_tile_map)
        .add_systems(OnEnter(AppState::GameStart), setup_tech_button)
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
    limit_main_camera_within_map_bounds(&mut transform, &map_setting);
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

    // limit the main camera movement within the map boundary
    limit_main_camera_within_map_bounds(&mut transform, &map_setting);
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

/// Limit the main camera movement within the map boundary.
///
/// TODO: In original game, when the map edge is seen, the camera is limited to the map edge.
///       When the map is not seen, the camera is limited to the civ visible area.
fn limit_main_camera_within_map_bounds(transform: &mut Transform, map_setting: &MapSetting) {
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
