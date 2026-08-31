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
    components::{City, CityProduction, CityYields, Civilian, Military, Owner},
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
    // 单位占用查询：用于决定新单位放置位置
    // 规则：每格最多一个军事 + 一个平民，且同格异类型单位必须属于同一文明
    unit_query: Query<(Entity, &ChildOf, &Owner), Or<(With<Military>, With<Civilian>)>>,
    military_query: Query<(), With<Military>>,
    civilian_query: Query<(), With<Civilian>>,
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
                // 在城市周边找到合适地块生成单位
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
                    &unit_query,
                    &military_query,
                    &civilian_query,
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
    unit_query: &Query<(Entity, &ChildOf, &Owner), Or<(With<Military>, With<Civilian>)>>,
    military_query: &Query<(), With<Military>>,
    civilian_query: &Query<(), With<Civilian>>,
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

    // 判断新单位是否为军事单位（军事单位拥有 Strength > 0）
    let is_military = ruleset.units[unit].strength > 0;

    // 在城市周边寻找符合「每格最多一个军事 + 一个平民」规则的生成地块
    let spawn_tile = find_spawn_tile(
        center_tile,
        *grid,
        city,
        is_military,
        owner,
        tile_entity_map,
        unit_query,
        military_query,
        civilian_query,
    );

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

/// 寻找单位生成的地块。
///
/// 遵循以下规则：
/// - 每个地块最多只能同时有一个军事单位和一个平民单位；
/// - 若地块上已有异类型单位，则该单位必须与待放置单位属于同一文明
///   （即：平民只能叠加在自己文明的军事单位旁，军事只能叠加在自己文明的平民单位旁）。
///
/// 候选地块按优先级：城市拥有的地块（含中心，`owned_tiles` 顺序）→ 中心地块的相邻地块。
/// 若所有候选地块都不满足规则，则回退到中心地块（保证单位一定能生成）。
/// 
/// TODO: 应当在没有可放置地块时告知玩家空出地块以放置建造完成的单位，或者其他处理方式，
///       如继续寻找城市的边缘地块以供放置单位。
fn find_spawn_tile(
    center_tile: civ_map_generator::tile::Tile,
    grid: civ_map_generator::grid::HexGrid,
    city: &City,
    is_military: bool,
    unit_owner: civ_map_generator::ruleset::enums::Nation,
    tile_entity_map: &Res<TileEntityMap>,
    unit_query: &Query<(Entity, &ChildOf, &Owner), Or<(With<Military>, With<Civilian>)>>,
    military_query: &Query<(), With<Military>>,
    civilian_query: &Query<(), With<Civilian>>,
) -> civ_map_generator::tile::Tile {
    use civ_map_generator::tile::Tile;

    // 检查某个地块是否允许放置待放置单位
    let can_place_on = |tile: Tile| -> bool {
        let tile_entity = tile_entity_map.get(tile);
        unit_query
            .iter()
            .all(|(entity, child_of, other_owner)| {
                if child_of.0 != tile_entity {
                    // 不在该地块上的单位不参与判定
                    return true;
                }
                let is_other_military = military_query.contains(entity);
                let is_other_civilian = civilian_query.contains(entity);
                if is_military && is_other_military {
                    // 已经有一个军事单位，不能再放军事单位
                    return false;
                }
                if !is_military && is_other_civilian {
                    // 已经有一个平民单位，不能再放平民单位
                    return false;
                }
                // 地块上已有异类型单位（军事 vs 平民）：要求属于同一文明
                other_owner.0 == unit_owner
            })
    };

    // 收集候选地块：先城市拥有的地块（含中心），再补中心相邻地块（去重）
    let mut candidates: Vec<Tile> = Vec::new();
    for &tile in &city.owned_tiles {
        if !candidates.contains(&tile) {
            candidates.push(tile);
        }
    }
    for neighbor in center_tile.neighbor_tiles(grid) {
        if !candidates.contains(&neighbor) {
            candidates.push(neighbor);
        }
    }

    // 找到第一个满足规则的地块
    for tile in candidates {
        if can_place_on(tile) {
            return tile;
        }
    }

    // 所有候选地块都不满足规则，回退到中心地块（最后兜底）
    warn!(
        "No valid tile for producing {} unit near city {}, falling back to center tile",
        if is_military { "military" } else { "civilian" },
        city.name
    );
    center_tile
}
