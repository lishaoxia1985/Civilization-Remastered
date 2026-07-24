use std::sync::Arc;

use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};

use civ_map_generator::{
    grid::*,
    map_parameters::{MapParameters, MapParametersBuilder, WorldGrid},
    ruleset::enums::{Difficulty, Era, Speed},
    tile_map::TileMap,
};

use assets::{AppState, MaterialResource};

use bevy::{
    camera::visibility::RenderLayers, input::mouse::MouseWheel, input_focus::InputFocus,
    prelude::*, sprite_render::Material2dPlugin, window::WindowResolution,
};

use crate::{
    assets::{ColorReplaceMaterial, ScreenState},
    combat::{
        advance_turn_system, ai_attack_system, handle_unit_attack, handle_unit_selection,
        setup_unit_info_panel, update_unit_info_panel,
    },
    game_state::{
        CivilizationStates, TurnPhase, setup_end_turn_button, setup_game_state_ui,
        update_game_state_ui,
    },
    generating_map::{check_map_generate_status, generate_tile_map},
    minimap::{
        DefaultFovIndicatorSize, handle_tile_click, minimap_fov_update, setup_info_panel,
        setup_minimap, spawn_tile_map_for_minimap,
    },
    tech_manage::insert_tech_manager_map,
    technology_screen::{
        ai_research_system, handle_tech_click_system, setup_tech_button, spawn_technology_screen,
    },
    world_map::{setup_tile_map, show_main_camera_area},
};

use civ_map_generator::ruleset::enums::Nation;

mod assets;
mod combat;
mod game_state;
mod generating_map;
mod minimap;
mod tech_manage;
mod technology_screen;
mod unit_component;
mod world_map;

#[derive(Resource)]
struct MapSetting(Arc<MapParameters>);

#[derive(Resource)]
struct TileMapResource(TileMap);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameSystemSet {
    PlayOnWorldMap,
    PlayOnTechScreen,
}

#[derive(Resource)]
pub struct GameSetting {
    speed: Speed,
    difficulty: Difficulty,
    start_era: Era,
}

impl Default for GameSetting {
    fn default() -> Self {
        Self {
            speed: Speed::Standard,
            difficulty: Difficulty::Chieftain,
            start_era: Era::AncientEra,
        }
    }
}

fn main() {
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

    let map_parameters = MapParametersBuilder::new(world_grid).build();

    let map_setting = MapSetting(Arc::new(map_parameters));

    // Create default fov indicator size resource
    let default_fov_indicator_size = DefaultFovIndicatorSize::default();

    // App setup
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Civilization-Remastered".to_owned(),
                resolution: WindowResolution::new(800, 600),
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(Material2dPlugin::<ColorReplaceMaterial>::default())
        .init_resource::<InputFocus>()
        .insert_resource(map_setting)
        .insert_resource(GameSetting::default())
        .insert_resource(default_fov_indicator_size)
        .init_state::<AppState>()
        .init_state::<TurnPhase>()
        .init_state::<ScreenState>()
        .configure_sets(
            Update,
            (
                GameSystemSet::PlayOnWorldMap
                    .run_if(in_state(AppState::GameStart))
                    .run_if(in_state(ScreenState::WorldMap)),
                GameSystemSet::PlayOnTechScreen
                    .run_if(in_state(AppState::GameStart))
                    .run_if(in_state(ScreenState::TechTree)),
            ),
        )
        .add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::MapGenerating)
                .load_collection::<MaterialResource>(),
        )
        .add_systems(OnEnter(AppState::AssetLoading), main_camera_setup)
        .add_systems(
            OnEnter(AppState::GameStart),
            (
                setup_minimap,
                setup_info_panel,
                setup_game_state_ui,
                setup_end_turn_button,
                setup_unit_info_panel,
            ),
        )
        .add_systems(
            Update,
            (
                check_map_generate_status.run_if(in_state(AppState::MapGenerating)),
                (
                    main_camera_movement,
                    cursor_drag_system,
                    zoom_main_camera_system,
                    minimap_fov_update,
                    handle_tile_click,
                    show_main_camera_area,
                    // Game state UI update
                    update_game_state_ui,
                    // Unit selection and attack
                    handle_unit_selection,
                    handle_unit_attack,
                    update_unit_info_panel,
                    // AI systems
                    ai_research_system,
                    ai_attack_system,
                    // Turn advancement system
                    advance_turn_system,
                )
                    .in_set(GameSystemSet::PlayOnWorldMap),
                // Technology click handling
                handle_tech_click_system.in_set(GameSystemSet::PlayOnTechScreen),
            ),
        )
        .add_systems(OnEnter(AppState::MapGenerating), generate_tile_map)
        .add_systems(OnEnter(AppState::GameStart), setup_tech_button)
        .add_systems(
            OnExit(AppState::MapGenerating),
            (setup_tile_map, spawn_tile_map_for_minimap),
        )
        .add_systems(OnExit(AppState::MapGenerating), insert_civilizations)
        .add_systems(OnEnter(AppState::GameStart), insert_tech_manager_map)
        .add_systems(OnEnter(AppState::GameStart), move_camera_to_player_center)
        .add_systems(OnEnter(ScreenState::TechTree), spawn_technology_screen)
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

fn move_camera_to_player_center(
    mut query: Query<&mut Transform, With<MainCamera>>,
    tile_map: Res<TileMapResource>,
    civilization: Res<CivilizationStates>,
) {
    let grid = tile_map.0.world_grid.grid;
    let player = civilization.player_nation;
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

fn insert_civilizations(mut commands: Commands, map_setting: Res<MapSetting>) {
    let civ_list = &map_setting.0.civilization_list;
    if civ_list.len() < 2 {
        panic!("Need at least 2 civilizations");
    }

    // Randomly select player civilization (using simple pseudo-random)
    let player_idx = (map_setting.0.seed % civ_list.len() as u64) as usize;
    let player_nation = civ_list[player_idx];

    // The rest are enemy civilizations
    let enemy_nations: Vec<Nation> = civ_list
        .iter()
        .filter(|&&c| c != player_nation)
        .copied()
        .collect();

    commands.insert_resource(CivilizationStates::new(player_nation, enemy_nations));
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
