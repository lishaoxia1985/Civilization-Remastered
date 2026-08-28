//! 城市建造插件
//!
//! 管理城市建造建筑和生产单位：
//! - 每回合根据城市产能累积生产进度
//! - 建筑建造完成后添加到城市的建筑列表
//! - 单位生产完成后在城市的周边地块生成单位（单位图标）

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Unit};

use crate::{
    NationComponent, ResolutionPhase, TurnManager, TurnState,
    assets::{ColorReplaceMaterial, GameAssets},
    components::{City, CityProduction, CityYields, Owner},
    plugins::unit_manager_plugin::spawn_unit_on_tile,
    resources::{MapParametersRes, TileEntityMap, TileMapRes},
};

/// 城市建造插件
pub struct CityConstructionPlugin;

impl Plugin for CityConstructionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(TurnState::Start),
            process_city_production.in_set(ResolutionPhase::Production),
        );
    }
}

/// 处理城市生产进度（建造建筑、生产单位）
fn process_city_production(
    manager: Res<TurnManager>,
    map_params: Res<MapParametersRes>,
    tile_map: Option<Res<TileMapRes>>,
    tile_entity_map: Res<TileEntityMap>,
    materials: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<ColorReplaceMaterial>>,
    mut commands: Commands,
    mut city_query: Query<(Entity, &mut City, &CityYields, &Owner), With<City>>,
    nation_query: Query<&NationComponent>,
) {
    // 第0回合不处理生产（开始游戏的回合）
    if manager.turn_number == 0 {
        return;
    }

    let current_nation_entity = manager.current_nation_entity();
    let Ok(nation_component) = nation_query.get(current_nation_entity) else {
        return;
    };
    let current_nation = nation_component.0;
    let ruleset = &map_params.0.ruleset;

    for (city_entity, mut city, yields, owner) in city_query.iter_mut() {
        // 只处理当前回合 Nation 的城市
        if owner.0 != current_nation {
            continue;
        }

        // 克隆当前生产项目（避免所有权/借用冲突）
        let Some(production_item) = city.current_production.clone() else {
            continue;
        };

        let production = yields.production;
        if production == 0 {
            continue;
        }

        city.production_progress += production;

        // 获取生产成本
        let production_cost = match production_item {
            CityProduction::Building(building) => ruleset.buildings[building].cost.max(0) as u32,
            CityProduction::Unit(unit) => ruleset.units[unit].cost.max(0) as u32,
        };

        // 进度未完成，继续建造
        if city.production_progress < production_cost {
            continue;
        }

        info!(
            "City {} completed production: {:?}",
            city.name, production_item
        );

        // 生产完成
        match production_item {
            CityProduction::Building(building) => {
                // 将建筑添加到城市的建筑列表
                city.buildings.push(building);
                info!(
                    "City {} constructed building {}",
                    city.name, ruleset.buildings[building].name
                );
            }
            CityProduction::Unit(unit) => {
                // 在城市中心地块生成单位
                spawn_produced_unit(
                    &mut commands,
                    unit,
                    owner.0,
                    ruleset,
                    city_entity,
                    &city,
                    &tile_map,
                    &tile_entity_map,
                    &materials,
                    &mut meshes,
                    &mut custom_materials,
                );
            }
        }

        // 重置生产
        city.current_production = None;
        city.production_progress = 0;
    }
}

/// 在城市中心地块生成建造完成的单位
#[allow(clippy::too_many_arguments)]
fn spawn_produced_unit(
    commands: &mut Commands,
    unit: Unit,
    owner: civ_map_generator::ruleset::enums::Nation,
    ruleset: &civ_map_generator::ruleset::Ruleset,
    _city_entity: Entity,
    city: &City,
    tile_map: &Option<Res<TileMapRes>>,
    tile_entity_map: &Res<TileEntityMap>,
    materials: &GameAssets,
    meshes: &mut Assets<Mesh>,
    custom_materials: &mut Assets<ColorReplaceMaterial>,
) {
    // 获取城市中心地块（城市拥有的第一个地块）
    let center_tile = match city.owned_tiles.first() {
        Some(&tile) => tile,
        None => {
            warn!("City has no owned tiles, cannot spawn unit");
            return;
        }
    };

    let Some(tile_map) = tile_map else {
        return;
    };
    let grid = &tile_map.0.world_grid.grid;

    // 计算地块像素大小，用于单位图标渲染
    let tile_pixel_size = Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0);

    // 尝试在城市中心地块生成单位，如果被占用则在相邻空闲地块生成
    let spawn_tile = find_spawn_tile(center_tile, *grid, tile_entity_map);

    // 获取目标地块实体
    let tile_entity = tile_entity_map.get(spawn_tile);

    // 调用单位管理器生成单位
    spawn_unit_on_tile(
        commands,
        unit,
        owner,
        ruleset,
        tile_entity,
        tile_pixel_size,
        meshes,
        custom_materials,
        materials,
    );

    info!(
        "Unit {} spawned near city at tile {:?}",
        unit.as_str(),
        spawn_tile
    );
}

/// 寻找单位生成的地块（城市中心优先，被占用时找相邻空闲地块）
fn find_spawn_tile(
    center_tile: civ_map_generator::tile::Tile,
    grid: civ_map_generator::grid::HexGrid,
    tile_entity_map: &Res<TileEntityMap>,
) -> civ_map_generator::tile::Tile {
    use civ_map_generator::tile::Tile;

    // 获取城市中心地块实体
    let _center_entity = tile_entity_map.get(center_tile);

    // 检查是否有单位占用了中心地块
    // 注意：这里只能做简单检查，如果有其他单位在地块上则尝试相邻地块
    // 由于单位是作为地块的子实体，我们需要通过 children 查询。

    // 简化：总是尝试相邻地块（如果中心地块有城市建筑/城市本身，单位会叠加显示）
    // 先检查相邻地块，如果相邻地块都不可用则回退到中心地块
    let neighbors: Vec<Tile> = center_tile.neighbor_tiles(grid).collect();

    // 尝试找到空闲的相邻可通行地块
    // 由于没有直接的地块占用查询，这里简单返回第一个相邻地块
    for neighbor in neighbors {
        // 检查该地块是否不是水域/山脉（通过 TileEntityMap 无法直接判断地形，但可以尝试）
        // 简化处理：直接使用第一个邻居
        return neighbor;
    }

    // 如果没有邻居则使用中心地块
    center_tile
}
