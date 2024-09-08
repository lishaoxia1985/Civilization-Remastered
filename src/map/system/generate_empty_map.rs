use bevy::prelude::{Commands, Res, ResMut};

use crate::{
    grid::hex::OffsetCoordinate,
    map::{base_terrain::BaseTerrain, terrain_type::TerrainType, AreaId, HexPosition, TileStorage},
    tile_map::MapParameters,
};

pub fn generate_empty_map(
    mut commands: Commands,
    map_parameters: Res<MapParameters>,
    mut tile_storage: ResMut<TileStorage>,
) {
    let height = map_parameters.map_size.height;
    let width = map_parameters.map_size.width;
    let offset = map_parameters.offset;
    let hex_layout = map_parameters.hex_layout;

    tile_storage.tiles = Vec::with_capacity((height * width) as usize);

    for y in 0..height {
        for x in 0..width {
            let offset_coordinate = OffsetCoordinate::new(x, y);
            let hex_coordinate = offset_coordinate.to_hex(offset, hex_layout.orientation);

            let tile_entity = commands
                .spawn((
                    HexPosition(hex_coordinate.to_array()),
                    TerrainType::Water,
                    BaseTerrain::Ocean,
                    AreaId(-1),
                ))
                .id();
            tile_storage.tiles.push(tile_entity);
        }
    }
}
