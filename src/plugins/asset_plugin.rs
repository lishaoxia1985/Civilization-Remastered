//! 资源加载插件
//!
//! 处理游戏资源的异步加载。

use bevy::prelude::*;
use bevy_asset_loader::loading_state::{
    LoadingState, LoadingStateAppExt, config::ConfigureLoadingState,
};

use crate::{AppState, assets::GameAssets};

/// 资源加载插件
pub struct AssetLoadingPlugin;

impl Plugin for AssetLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(
            LoadingState::new(AppState::AssetLoading)
                .continue_to_state(AppState::MapGenerating)
                .load_collection::<GameAssets>(),
        );
    }
}
