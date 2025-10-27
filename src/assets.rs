use bevy::{platform::collections::HashMap, prelude::*};
use bevy_asset_loader::{asset_collection::AssetCollection, mapped::AssetFileStem};

#[derive(AssetCollection, Resource)]
pub struct MaterialResource {
    #[asset(path = "./", collection(typed, mapped))]
    textures: HashMap<AssetFileStem, Handle<Image>>,
}

impl MaterialResource {
    pub fn texture_handle(&self, name: &str) -> Handle<Image> {
        self.textures
            .get(name)
            .unwrap_or_else(|| panic!("Can't find Image: {}", name))
            .clone()
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum AppState {
    #[default]
    AssetLoading,
    GeneratingMap,
    GameStart,
}
