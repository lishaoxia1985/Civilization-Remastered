//! 地图插件
//!
//! 管理地图生成、世界地图渲染和可见区域控制。

use std::{collections::HashMap, f32::consts::FRAC_PI_2, sync::Arc};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, block_on, futures_lite::future},
};
use civ_map_generator::{
    generate_map,
    grid::{Grid, Hex, HexOrientation},
    ruleset::enums::*,
    tile::Tile,
    tile_map::RiverEdge,
};

use crate::{
    AppState,
    assets::{GameAssets, hex_mesh, line_mesh},
    components::WorldTile,
    resources::{MapGeneratorTask, MapParametersRes, TileEntityMap, TileMapRes},
};

/// 地图插件
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MapGenerating), generate_tile_map)
            .add_systems(
                Update,
                check_map_generate_status.run_if(in_state(AppState::MapGenerating)),
            )
            .add_systems(OnExit(AppState::MapGenerating), setup_tile_map);
    }
}

// ============ 地图生成 ============

/// 开始生成地图（异步任务）
fn generate_tile_map(mut commands: Commands, map_params: Res<MapParametersRes>) {
    let map_parameters = Arc::clone(&map_params.0);
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move { generate_map(&map_parameters) });
    commands.insert_resource(MapGeneratorTask(task));
}

/// 检查地图生成是否完成
fn check_map_generate_status(
    mut commands: Commands,
    task: Option<ResMut<MapGeneratorTask>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(mut task) = task else {
        return;
    };

    if let Some(tile_map) = block_on(future::poll_once(&mut task.0)) {
        commands.insert_resource(TileMapRes(tile_map));
        commands.remove_resource::<MapGeneratorTask>();
        next_state.set(AppState::GameStart);
    }
}

// ============ 世界地图渲染 ============

/// 设置世界地图上的地块
fn setup_tile_map(
    mut commands: Commands,
    tile_map: Option<Res<TileMapRes>>,
    materials: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;

    let grid = tile_map.world_grid.grid;

    let base_terrain_and_material: enum_map::EnumMap<BaseTerrain, Handle<ColorMaterial>> = enum_map::enum_map! {
        base_terrain => color_materials.add(materials.texture_handle(base_terrain.as_str())),
    };

    let mut tile_and_river_flow_direction = HashMap::new();

    tile_map.river_list.iter().flatten().for_each(|river_edge| {
        tile_and_river_flow_direction
            .entry(river_edge.tile)
            .or_insert_with(Vec::new)
            .push(river_edge.flow_direction);
    });

    let all_possible_river_edge_mesh: Vec<_> = grid
        .corner_direction_array()
        .iter()
        .map(|&flow_direction| {
            let river_edge = RiverEdge {
                tile: Tile::new(0),
                flow_direction,
            };

            let [start_corner_direction, end_corner_direction] =
                river_edge.start_and_end_corner_directions(grid);
            let start_corner_position = grid.layout.corner(Hex::new(0, 0), start_corner_direction);
            let end_corner_position = grid.layout.corner(Hex::new(0, 0), end_corner_direction);

            let start = [start_corner_position[0], start_corner_position[1], 0.0];
            let end = [end_corner_position[0], end_corner_position[1], 0.0];
            let line_mesh = line_mesh(start.into(), end.into(), 1.5);
            (flow_direction, line_mesh)
        })
        .collect();

    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);

    let feature_ice_sprite_rotation = match grid.layout.orientation {
        HexOrientation::Pointy => Quat::default(),
        HexOrientation::Flat => Quat::from_rotation_z(FRAC_PI_2 * 3.),
    };

    let hex_mesh = meshes.add(hex_mesh(&grid));

    let tile_count = grid.size.area();

    let mut tile_entity_map = TileEntityMap::with_capacity(tile_count as usize);

    for tile in tile_map.all_tiles() {
        let tile_entity = commands
            .spawn((
                Mesh2d(hex_mesh.clone()),
                MeshMaterial2d(base_terrain_and_material[tile.base_terrain(tile_map)].clone()),
                Pickable::default(),
                WorldTile(tile),
            ))
            .id();

        tile_entity_map.push(tile_entity);

        commands.entity(tile_entity).with_children(|parent| {
            // 绘制河流
            if let Some(flow_direction_list) = tile_and_river_flow_direction.get(&tile) {
                flow_direction_list.iter().for_each(|direction| {
                    let (_, line_mesh) = all_possible_river_edge_mesh
                        .iter()
                        .find(|(d, _)| d == direction)
                        .unwrap();
                    parent.spawn((
                        Mesh2d(meshes.add(line_mesh.clone())),
                        MeshMaterial2d(
                            color_materials
                                .add(ColorMaterial::from_color(Color::srgb_u8(140, 215, 215))),
                        ),
                        Transform {
                            translation: Vec3::new(0., 0., 5.),
                            ..Default::default()
                        },
                    ));
                })
            };

            // 绘制地形
            let terrain_type = tile.terrain_type(tile_map);
            let is_mountain_without_wonder =
                terrain_type == TerrainType::Mountain && tile.natural_wonder(tile_map).is_none();

            if is_mountain_without_wonder || terrain_type == TerrainType::Hill {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(terrain_type.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 3.),
                        ..Default::default()
                    },
                ));
            }

            // 绘制地貌特征
            if let Some(feature) = tile.feature(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(feature.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        rotation: if feature == Feature::Ice {
                            feature_ice_sprite_rotation
                        } else {
                            Quat::default()
                        },
                        ..Default::default()
                    },
                ));
            }

            // 绘制自然奇观
            if let Some(natural_wonder) = tile.natural_wonder(tile_map) {
                parent.spawn((
                    Sprite {
                        custom_size: Some(tile_pixel_size),
                        image: materials.texture_handle(natural_wonder.as_str()),
                        ..Default::default()
                    },
                    Transform {
                        translation: Vec3::new(0., 0., 2.),
                        ..Default::default()
                    },
                ));
            }
        });
    }

    commands.insert_resource(tile_entity_map);
}
