//! 文明重制版 - 主入口
//!
//! 使用 Bevy 引擎实现的文明游戏重制版。
//! 所有功能通过插件系统组织。

use std::sync::Arc;

use bevy::{
    input_focus::InputFocus, prelude::*, sprite_render::Material2dPlugin, window::WindowResolution,
};

use civ_map_generator::{
    grid::*,
    map_parameters::{MapParametersBuilder, WorldGrid},
    ruleset::enums::Nation,
};

use crate::{
    assets::ColorReplaceMaterial,
    plugins::{
        AssetLoadingPlugin, CameraPlugin, CombatPlugin, GameStatePlugin, MapPlugin, MinimapPlugin,
        TechPlugin,
    },
    resources::{GameSettings, MapParametersRes, TechManager},
};

mod assets;
mod components;
mod plugins;
mod resources;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum AppState {
    #[default]
    AssetLoading,
    MapGenerating,
    GameStart,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
// And we need to add an attribute to let us know what the source state is
// and what value it needs to have. This will ensure that unless we're
// in [`AppState::GameStart`], the [`ScreenState`] state resource
// will not exist.
#[source(AppState = AppState::GameStart)]
#[states(scoped_entities)]
pub enum ScreenState {
    #[default]
    WorldMap,
    TechTree,
}

fn main() {
    // 创建地图参数
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
    let map_params = MapParametersRes(Arc::new(map_parameters));

    // 应用设置
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
        // 注册游戏插件
        .add_plugins(AssetLoadingPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(CombatPlugin)
        .add_plugins(GameStatePlugin)
        .add_plugins(MapPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(TechPlugin)
        // 初始化资源
        .init_resource::<InputFocus>()
        .insert_resource(map_params)
        .insert_resource(GameSettings::default())
        // 初始化状态
        .init_state::<AppState>()
        .add_sub_state::<ScreenState>() // We set the substate up here.
        // 初始化文明
        .add_systems(OnExit(AppState::MapGenerating), insert_civilizations)
        .run();
}

/// 插入文明资源
fn insert_civilizations(mut commands: Commands, map_params: Res<MapParametersRes>) {
    let civ_list = &map_params.0.civilization_list;
    if civ_list.len() < 2 {
        panic!("至少需要2个文明");
    }

    // 随机选择玩家文明
    let player_idx = (map_params.0.seed % civ_list.len() as u64) as usize;
    let player_nation = civ_list[player_idx];

    commands.spawn((NationComponent(player_nation), Player, SciencePerTurn(3)));

    // 其余为敌方文明
    let enemy_nations: Vec<Nation> = civ_list
        .iter()
        .filter(|&&c| c != player_nation)
        .copied()
        .collect();

    for &nation in enemy_nations.iter() {
        commands.spawn((NationComponent(nation), Enemy, SciencePerTurn(3)));
    }
}

#[derive(Component)]
pub struct NationComponent(pub Nation);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct SciencePerTurn(pub i32);
