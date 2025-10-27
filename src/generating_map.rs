use std::sync::Arc;

use bevy::{
    ecs::{
        resource::Resource,
        system::{Commands, Res, ResMut},
    },
    state::state::NextState,
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};
use civ_map_generator::{generate_map, tile_map::TileMap};

use crate::{MapSetting, RulesetResource, TileMapResource, assets::AppState};

#[derive(Resource)]
pub struct MapGenerator(Task<TileMap>);

pub fn generate_tile_map(
    mut commands: Commands,
    map_setting: Res<MapSetting>,
    ruleset: Res<RulesetResource>,
) {
    let map_parameters = Arc::clone(&map_setting.0);
    let ruleset = Arc::clone(&ruleset.0);
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move { generate_map(&map_parameters, &ruleset) });
    commands.insert_resource(MapGenerator(task));
}

pub fn check_map_generate_status(
    mut commands: Commands,
    task: Option<ResMut<MapGenerator>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(mut task) = task else {
        return;
    };

    if let Some(tile_map) = block_on(future::poll_once(&mut task.0)) {
        commands.insert_resource(TileMapResource(tile_map));
        commands.remove_resource::<MapGenerator>();
        next_state.set(AppState::GameStart);
    }
}
