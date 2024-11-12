use bevy::{asset::LoadedFolder, prelude::*, utils::HashMap};

#[derive(Resource)]
pub struct MaterialResource {
    pub textures: HashMap<String, Handle<Image>>,
}

impl MaterialResource {
    pub fn texture_handle(&self, name: &str) -> Handle<Image> {
        self.textures
            .get(name)
            .expect(&format!("Can't find Image: {}", name))
            .clone()
    }
}

impl FromWorld for MaterialResource {
    fn from_world(world: &mut World) -> Self {
        let textures: HashMap<String, Handle<Image>> = HashMap::new();
        MaterialResource { textures }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum AppState {
    #[default]
    Setup,
    Finished,
    GameStart,
}

#[derive(Resource, Default)]
pub struct SpriteFolder(Handle<LoadedFolder>);

pub fn load_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    // load multiple, individual sprites from a folder
    commands.insert_resource(SpriteFolder(
        asset_server.load_folder("./"),
        // asset_server.load_folder("Images/TileSets/HexaRealm/Tiles"),
    ));
}

pub fn check_textures(
    mut next_state: ResMut<NextState<AppState>>,
    rpg_sprite_folder: Res<SpriteFolder>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
) {
    // Advance the `AppState` once all sprite handles have been loaded by the `AssetServer`
    for event in events.read() {
        if event.is_loaded_with_dependencies(&rpg_sprite_folder.0) {
            next_state.set(AppState::Finished);
        }
    }
}

pub fn setup(
    mut next_state: ResMut<NextState<AppState>>,
    rpg_sprite_handles: Res<SpriteFolder>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut material_resource: ResMut<MaterialResource>,
) {
    let loaded_folder = loaded_folders.get(&rpg_sprite_handles.0).unwrap();

    loaded_folder.handles.iter().for_each(|handle| {
        let texture_name = handle
            .path()
            .unwrap()
            .path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        dbg!(&texture_name);
        material_resource
            .textures
            .insert(texture_name, handle.clone().typed());
    });
    next_state.set(AppState::GameStart);
}
