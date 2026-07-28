//! 游戏资源模块
//!
//! 管理游戏中的所有资源加载、自定义材质和网格。

mod material;
mod mesh;

pub use material::*;
pub use mesh::*;

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_asset_loader::{asset_collection::AssetCollection, mapped::AssetFileStem};

/// 游戏资源集合
#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(path = "Images", collection(typed, mapped))]
    textures: HashMap<AssetFileStem, Handle<Image>>,
}

impl GameAssets {
    /// 获取纹理句柄
    pub fn texture_handle(&self, name: &str) -> Handle<Image> {
        self.textures
            .get(name)
            .unwrap_or_else(|| panic!("找不到图片资源: {}", name))
            .clone()
    }
}
