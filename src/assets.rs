use bevy::{prelude::*, utils::HashMap};

#[derive(Resource)]
pub struct MaterialResource {
    textures: HashMap<String, Handle<Image>>,
}

impl MaterialResource {
    pub fn texture_handle(&self, name: &str) -> Handle<Image> {
        self.textures
            .get(name)
            .unwrap_or_else(|| panic!("Can't find Image: {}", name))
            .clone()
    }
}

impl FromWorld for MaterialResource {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mut textures: HashMap<String, Handle<Image>> = HashMap::new();
        let mut insert_texture_handle_to_hashmap_from_folder = |path: &str| {
            let handles = asset_server.load_folder(path).unwrap();
            handles.into_iter().for_each(|handle| {
                let texture_name = asset_server
                    .get_handle_path(&handle)
                    .unwrap()
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                textures.insert(texture_name, handle.typed());
            });
        };
        insert_texture_handle_to_hashmap_from_folder("Images.Flags/FlagIcons");
        insert_texture_handle_to_hashmap_from_folder("Images/TileSets/Default");
        insert_texture_handle_to_hashmap_from_folder("Images/TileSets/HexaRealm/Tiles");

        MaterialResource { textures }
    }
}

pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaterialResource>();
    }
}
