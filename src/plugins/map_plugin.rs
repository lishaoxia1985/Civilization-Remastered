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
    ruleset::{Ruleset, enums::*},
    tile::Tile,
    tile_map::RiverEdge,
};

use crate::{
    AppState,
    assets::{ColorReplaceMaterial, GameAssets, hex_mesh, line_mesh},
    components::{Experience, Health, Movement, Owner, Strength, UnitComponent, WorldTile},
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
    map_params: Res<MapParametersRes>,
    tile_map: Option<Res<TileMapRes>>,
    materials: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut custom_materials: ResMut<Assets<ColorReplaceMaterial>>,
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

        let ruleset = &map_params.0.ruleset;
        let radius = tile_pixel_size.min_element() / 3.0;

        let inner_rectangle = meshes.add(Rectangle::new(radius / 2., radius / 2.));
        let outer_rectangle = meshes.add(Rectangle::new(radius, radius));

        // 在起始位置生成单位
        if let Some(&civilization) = tile_map.starting_tile_and_civilization.get(&tile) {
            let replace_warrior_unit = ruleset.units.values().find(|&unit| {
                unit.unique_to == civilization.as_str() && unit.replaces == "Warrior"
            });
            let military_unit = if let Some(unit) = replace_warrior_unit {
                Unit::from_str(&unit.name)
            } else {
                Unit::Warrior
            };

            commands.entity(tile_entity).with_children(|parent| {
                parent.spawn(unit_bundle(
                    UnitComponent::Military(military_unit),
                    Owner::Civilization(civilization),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));

                parent.spawn(unit_bundle(
                    UnitComponent::Civilian(Unit::Settler),
                    Owner::Civilization(civilization),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));
            });
        }

        // 城邦起始位置生成单位
        if let Some(&city_state) = tile_map.starting_tile_and_city_state.get(&tile) {
            commands.entity(tile_entity).with_children(|parent| {
                parent.spawn(unit_bundle(
                    UnitComponent::Civilian(Unit::Settler),
                    Owner::CityState(city_state),
                    ruleset,
                    inner_rectangle.clone(),
                    outer_rectangle.clone(),
                    &mut custom_materials,
                    &materials,
                    tile_pixel_size,
                ));
            });
        }
    }

    commands.insert_resource(tile_entity_map);
}

/// 创建单位组（包含战斗系统所需的所有组件）
fn unit_bundle(
    unit: UnitComponent,
    owner: Owner,
    ruleset: &Ruleset,
    inner_rectangle: Handle<Mesh>,
    outer_rectangle: Handle<Mesh>,
    custom_materials: &mut ResMut<Assets<ColorReplaceMaterial>>,
    materials: &GameAssets,
    tile_pixel_size: Vec2,
) -> impl Bundle {
    let (unit_name, transform_y, out_texture_name) = match &unit {
        UnitComponent::Civilian(unit) => (unit.as_str(), -tile_pixel_size.y / 4., "sv_unitcitizen"),
        UnitComponent::Military(unit) => (unit.as_str(), tile_pixel_size.y / 4., "sv_unitmilitary"),
    };

    let nation = match owner {
        Owner::Civilization(nation) | Owner::CityState(nation) => nation,
    };

    let outer_color = ruleset.nations[nation].outer_color;
    let inner_color = ruleset.nations[nation].inner_color;

    // 从 ruleset 中获取单位属性
    let unit_key = *match &unit {
        UnitComponent::Military(u) => u,
        UnitComponent::Civilian(u) => u,
    };
    let unit_info = &ruleset.units[unit_key];

    let (strength, health, movement) = match &unit {
        UnitComponent::Military(_) => {
            let hp = 100u32;
            let mv = unit_info.movement.max(0) as u32;
            (
                Strength(unit_info.strength.max(0) as u32),
                Health {
                    current: hp,
                    max: hp,
                },
                Movement {
                    current: mv,
                    max: mv,
                },
            )
        }
        UnitComponent::Civilian(_) => {
            let hp = 50u32;
            let mv = unit_info.movement.max(0) as u32;
            (
                Strength(0),
                Health {
                    current: hp,
                    max: hp,
                },
                Movement {
                    current: mv,
                    max: mv,
                },
            )
        }
    };

    (
        unit,
        owner,
        strength,
        health,
        movement,
        Experience {
            current: 0,
            max: 100,
        },
        Mesh2d(inner_rectangle.clone()),
        MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
            inner_color: bevy::color::LinearRgba::from_u8_array_no_alpha(inner_color),
            outer_color: bevy::color::LinearRgba::from_u8_array_no_alpha(outer_color),
            texture: materials.texture_handle(&unit_name),
        })),
        Transform {
            translation: Vec3::new(0., transform_y, 6.),
            ..Default::default()
        },
        children![(
            Mesh2d(outer_rectangle.clone()),
            MeshMaterial2d(custom_materials.add(ColorReplaceMaterial {
                inner_color: bevy::color::LinearRgba::from_u8_array_no_alpha(inner_color,),
                outer_color: bevy::color::LinearRgba::from_u8_array_no_alpha(outer_color,),
                texture: materials.texture_handle(out_texture_name),
            },)),
            Transform::from_xyz(0., 0., -1.),
        )],
    )
}
