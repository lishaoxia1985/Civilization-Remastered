//! 城市管理插件
//!
//! 管理城市的完整生命周期，包括：
//! - 城市收益统计（每回合计算粮食、产能、科研、金币、文化、信仰）
//! - 人口增长（粮食积累与消耗）
//! - 城市边界扩张（文化积累与环状扩张，初始1格→最大5格）
//! - 市民工作地块管理（只能在3格范围内工作）
//! - 城市边界渲染（外轮廓线条）
//! - 市民工作图标渲染（小人图标）
//!
//! 注意：城市生产（建造建筑、生产单位）由 CityConstructionPlugin 管理

use bevy::prelude::*;
use civ_map_generator::{
    grid::{Grid, HexGrid},
    ruleset::{
        Ruleset,
        enums::{TerrainType, TileImprovement, Unit},
    },
    tile::Tile,
    tile_map::TileMap,
};

use crate::{
    FoundCityRequestMessage, NationComponent, ResolutionPhase, TurnManager, TurnState,
    assets::{GameAssets, hex_edge_mesh},
    components::{
        City, CityBorderHighlight, CityYields, Civilian, Owner, TileImprovementComponent,
        UnitComponent,
    },
    resources::{MapParametersRes, TileEntityMap, TileMapRes},
};

/// 城市管理插件
pub struct CityManagementPlugin;

impl Plugin for CityManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(TurnState::Start),
            (
                process_city_yields.in_set(ResolutionPhase::Science),
                process_city_expansion.in_set(ResolutionPhase::Production),
            ),
        )
        .add_systems(
            Update,
            (render_city_borders,).run_if(in_state(crate::ScreenState::WorldMap)),
        )
        .add_observer(handle_found_city_request);
    }
}

// ============ 城市收益系统 ============

/// 每回合计算城市产量
fn process_city_yields(
    manager: Res<TurnManager>,
    map_params: Res<MapParametersRes>,
    tile_map: Option<Res<TileMapRes>>,
    tile_entity_map: Res<TileEntityMap>,
    improvement_query: Query<&TileImprovementComponent>,
    mut city_query: Query<(&mut City, &mut CityYields, &Owner)>,
    nation_query: Query<&NationComponent>,
) {
    // 第0回合不处理产量（开始游戏的回合）
    if manager.turn_number == 0 {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;
    let ruleset = &map_params.0.ruleset;
    let grid = tile_map.world_grid.grid;

    // 获取当前回合的 Nation
    let current_nation_entity = manager.current_nation_entity();
    let Ok(nation_component) = nation_query.get(current_nation_entity) else {
        return;
    };
    let current_nation = nation_component.0;

    for (mut city, mut yields, owner) in city_query.iter_mut() {
        // 只处理当前回合 Nation 的城市
        if owner.0 != current_nation {
            continue;
        }

        let total = calculate_city_yields(
            &*city,
            tile_map,
            grid,
            ruleset,
            &tile_entity_map,
            &improvement_query,
        );

        *yields = total;

        // 处理人口增长
        city.food += total.food;
        while city.food >= city.food_needed {
            city.food -= city.food_needed;
            city.population += 1;
            // 文明5：人口增长所需粮食随人口增加而增加
            city.food_needed = food_needed_for_population(city.population);
            // 增长人口后，自动分配额外的市民到空闲地块
            assign_worker_tiles(
                &mut city,
                tile_map,
                ruleset,
                &tile_entity_map,
                &improvement_query,
            );
        }

        info!(
            "City {}: Pop={}, Food={}/{}, Prod={}, Sci={}, Gold={}, Culture={}, Faith={}",
            city.name,
            city.population,
            city.food,
            city.food_needed,
            total.production,
            total.science,
            total.gold,
            total.culture,
            total.faith
        );
    }
}

/// 计算城市一回合的总产量
fn calculate_city_yields(
    city: &City,
    tile_map: &TileMap,
    grid: HexGrid,
    ruleset: &Ruleset,
    tile_entity_map: &TileEntityMap,
    improvement_query: &Query<&TileImprovementComponent>,
) -> CityYields {
    let mut total = CityYields::default();

    let center_tile = *city
        .owned_tiles
        .first()
        .expect("City must have a center tile");
    let work_radius = city.work_radius as i32;

    // 1. 地块产量（被市民工作的的地块 + 城市中心）
    for (idx, &tile) in city.owned_tiles.iter().enumerate() {
        let is_worked = idx == 0 // 城市中心总是被工作
            || (city.worked_tiles.contains(&tile)
                && grid.distance_to(center_tile.to_cell(), tile.to_cell()) <= work_radius);
        // 只能工作在3格范围内

        if !is_worked {
            continue;
        }

        // 获取地块上的设施（如果有）
        let tile_entity = tile_entity_map.get(tile);
        let improvement = improvement_query.get(tile_entity).ok().map(|comp| comp.0);

        let yields = tile_yields_with_improvement(tile, tile_map, ruleset, improvement);
        total.food += yields.food;
        total.production += yields.production;
        total.science += yields.science;
        total.gold += yields.gold;
        total.culture += yields.culture;
        total.faith += yields.faith;
    }

    // 2. 建筑产量
    for &building in &city.buildings {
        let building_info = &ruleset.buildings[building];
        total.food += building_info.yields.food.max(0) as u32;
        total.production += building_info.yields.production.max(0) as u32;
        total.science += building_info.yields.science.max(0) as u32;
        total.gold += building_info.yields.gold.max(0) as u32;
        total.culture += building_info.yields.culture.max(0) as u32;
        total.faith += building_info.yields.faith.max(0) as u32;
        total.happiness += building_info.yields.happiness as i32;
    }

    total
}

/// 计算单个地块的总产量（地形 + 特性 + 资源 + 设施）
fn tile_yields_with_improvement(
    tile: Tile,
    tile_map: &TileMap,
    ruleset: &Ruleset,
    improvement: Option<TileImprovement>,
) -> CityYields {
    let mut yields = CityYields::default();

    // 1. 基础地形产量
    let base_terrain = tile.base_terrain(tile_map);
    let base_info = &ruleset.base_terrains[base_terrain];
    yields.food += base_info.yields.food.max(0) as u32;
    yields.production += base_info.yields.production.max(0) as u32;
    yields.science += base_info.yields.science.max(0) as u32;
    yields.gold += base_info.yields.gold.max(0) as u32;
    yields.culture += base_info.yields.culture.max(0) as u32;
    yields.faith += base_info.yields.faith.max(0) as u32;

    // 2. 地形类型产量（如丘陵）
    let terrain_type = tile.terrain_type(tile_map);
    let terrain_info = &ruleset.terrain_types[terrain_type];
    yields.food += terrain_info.yields.food.max(0) as u32;
    yields.production += terrain_info.yields.production.max(0) as u32;
    yields.science += terrain_info.yields.science.max(0) as u32;
    yields.gold += terrain_info.yields.gold.max(0) as u32;
    yields.culture += terrain_info.yields.culture.max(0) as u32;
    yields.faith += terrain_info.yields.faith.max(0) as u32;

    // 3. 特性产量（如森林）
    if let Some(feature) = tile.feature(tile_map) {
        let feature_info = &ruleset.features[feature];
        yields.food += feature_info.yields.food.max(0) as u32;
        yields.production += feature_info.yields.production.max(0) as u32;
        yields.science += feature_info.yields.science.max(0) as u32;
        yields.gold += feature_info.yields.gold.max(0) as u32;
        yields.culture += feature_info.yields.culture.max(0) as u32;
        yields.faith += feature_info.yields.faith.max(0) as u32;
    }

    // 4. 资源产量
    if let Some((resource, _)) = tile.resource(tile_map) {
        let resource_info = &ruleset.resources[resource];
        yields.food += resource_info.yields.food.max(0) as u32;
        yields.production += resource_info.yields.production.max(0) as u32;
        yields.science += resource_info.yields.science.max(0) as u32;
        yields.gold += resource_info.yields.gold.max(0) as u32;
        yields.culture += resource_info.yields.culture.max(0) as u32;
        yields.faith += resource_info.yields.faith.max(0) as u32;
    }

    // 5. 设施产量
    if let Some(improvement) = improvement {
        let improvement_info = &ruleset.tile_improvements[improvement];
        yields.food = yields
            .food
            .saturating_add(improvement_info.yields.food.max(0) as u32);
        yields.production = yields
            .production
            .saturating_add(improvement_info.yields.production.max(0) as u32);
        yields.science = yields
            .science
            .saturating_add(improvement_info.yields.science.max(0) as u32);
        yields.gold = yields
            .gold
            .saturating_add(improvement_info.yields.gold.max(0) as u32);
        yields.culture = yields
            .culture
            .saturating_add(improvement_info.yields.culture.max(0) as u32);
        yields.faith = yields
            .faith
            .saturating_add(improvement_info.yields.faith.max(0) as u32);
    }

    yields
}

/// 计算人口增长所需粮食（文明5公式）
///
/// 文明5人口增长公式: 15 + 8*(n-1) + (n-1)^1.5
/// 其中 n 为当前人口
fn food_needed_for_population(population: u32) -> u32 {
    let pop_minus_one = (population - 1) as f32;
    (15.0 + 8.0 * pop_minus_one + pop_minus_one.powf(1.5)).round() as u32
}

/// 为市民自动分配工作地块
#[allow(clippy::too_many_arguments)]
fn assign_worker_tiles(
    city: &mut City,
    tile_map: &TileMap,
    ruleset: &Ruleset,
    tile_entity_map: &TileEntityMap,
    improvement_query: &Query<&TileImprovementComponent>,
) {
    // 当前已分配的市民数 = 已工作地块数（城市中心自动工作，不占用市民名额）
    let assigned_workers = city.worked_tiles.len();

    if assigned_workers >= city.population as usize {
        return;
    }

    let center_tile = *city
        .owned_tiles
        .first()
        .expect("City must have a center tile");
    let grid = tile_map.world_grid.grid;
    let work_radius = city.work_radius as i32;

    // 寻找未被市民工作的地块并从最高产量开始分配
    for _ in assigned_workers..city.population as usize {
        // 在非城市中心的所有权地块中，找到产量最高的未工作地块
        // 且必须在工作半径（3格）范围内
        let best_tile = city
            .owned_tiles
            .iter()
            .enumerate()
            .filter(|&(idx, &tile)| {
                idx > 0
                    && !city.worked_tiles.contains(&tile)
                    && grid.distance_to(center_tile.to_cell(), tile.to_cell()) <= work_radius
            })
            .max_by_key(|&(_, &tile)| {
                let tile_entity = tile_entity_map.get(tile);
                let improvement = improvement_query.get(tile_entity).ok().map(|comp| comp.0);
                let yields = tile_yields_with_improvement(tile, tile_map, ruleset, improvement);
                yields.food + yields.production + yields.science + yields.gold + yields.culture
            })
            .map(|(_, &tile)| tile);

        if let Some(tile) = best_tile {
            city.worked_tiles.push(tile);
        } else {
            break;
        }
    }
}

// ============ 城市边界扩张系统 ============

/// 处理城市边界扩张（文化积累）
fn process_city_expansion(
    manager: Res<TurnManager>,
    tile_map: Option<Res<TileMapRes>>,
    mut city_query: Query<(&mut City, &CityYields, &Owner)>,
    nation_query: Query<&NationComponent>,
) {
    if manager.turn_number == 0 {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;
    let grid = tile_map.world_grid.grid;

    let current_nation_entity = manager.current_nation_entity();
    let Ok(nation_component) = nation_query.get(current_nation_entity) else {
        return;
    };
    let current_nation = nation_component.0;

    for (mut city, yields, owner) in city_query.iter_mut() {
        // 只处理当前回合 Nation 的城市
        if owner.0 != current_nation {
            continue;
        }

        city.culture += yields.culture;

        // 检查是否达到扩张所需文化
        while city.culture >= city.culture_to_expand {
            city.culture -= city.culture_to_expand;

            // 扩张边界
            let expanded = expand_city_border(&mut city, grid, tile_map);

            if !expanded {
                break;
            }

            info!(
                "City {} expanded border to radius {}!",
                city.name, city.border_radius
            );

            // 扩张后文化需求增加（文明5公式）
            city.culture_to_expand += 10 + city.border_radius * 5;
        }
    }
}

/// 扩张城市边界 - 添加下一个距离环上的地块
fn expand_city_border(city: &mut City, grid: HexGrid, tile_map: &TileMap) -> bool {
    let center_tile = *city
        .owned_tiles
        .first()
        .expect("City must have a center tile");
    let next_radius = city.border_radius + 1;

    // 检查是否已达到最大边界半径
    if next_radius > city.max_border_radius {
        return false;
    }

    // 获取下一个环上的所有地块
    let new_tiles: Vec<Tile> = center_tile
        .tiles_at_distance(next_radius, grid)
        .filter(|&tile| {
            // 地块不能是水域/山脉（不可工作/不可通过的地块不能扩张）
            let terrain = tile.terrain_type(tile_map);
            terrain != TerrainType::Water && terrain != TerrainType::Mountain
        })
        .collect();

    if new_tiles.is_empty() {
        return false;
    }

    city.border_radius = next_radius;
    city.owned_tiles.extend(new_tiles);
    true
}

/// 处理建城请求
fn handle_found_city_request(
    event: On<FoundCityRequestMessage>,
    mut commands: Commands,
    unit_query: Query<(Entity, &ChildOf, &UnitComponent, &Owner), With<Civilian>>,
    tile_entity_map: Res<TileEntityMap>,
    city_query: Query<(), With<City>>,
    tile_map: Option<Res<TileMapRes>>,
    map_params: Res<MapParametersRes>,
    improvement_query: Query<&TileImprovementComponent>,
    materials: Res<GameAssets>,
) {
    let found_city_request = event.event();
    let unit_entity = found_city_request.unit;
    let target_tile = found_city_request.target_tile;

    let Ok((_, _child_of, unit_component, owner)) = unit_query.get(unit_entity) else {
        return;
    };

    // 只有移民单位可以建立城市
    if unit_component.0 != Unit::Settler {
        return;
    }

    let target_tile_entity = tile_entity_map.get(target_tile);

    // 检查目标地块是否已有城市
    if city_query.get(target_tile_entity).is_ok() {
        warn!("Tile already has a city");
        return;
    }

    // 获取地块像素大小用于渲染城市图片
    let tile_pixel_size = tile_map
        .as_ref()
        .map(|tile_map| {
            let grid = &tile_map.0.world_grid.grid;
            Vec2::from(grid.layout.size) * Vec2::new(2.0, 2.0)
        })
        .unwrap_or(Vec2::new(100.0, 100.0));

    // 创建城市实体
    let mut city = City::new(format!("City at tile {}", target_tile.index()));
    // 城市中心地块自动被城市拥有
    city.owned_tiles.push(target_tile);

    // 初始化城市边界：初始拥有城市中心周围1格（文明5规则）
    // 城市领土通过文化积累逐步扩展到最大5格
    city.border_radius = 1;
    city.max_border_radius = 5;
    city.work_radius = 3;

    // 添加半径1的所有可工作地块
    if let Some(tile_map) = &tile_map {
        let grid = tile_map.0.world_grid.grid;
        let ring_tiles: Vec<Tile> = target_tile
            .tiles_at_distance(1, grid)
            .filter(|&tile| {
                // 排除水域/山脉（不可工作地块）
                let terrain = tile.terrain_type(&tile_map.0);
                terrain != TerrainType::Water && terrain != TerrainType::Mountain
            })
            .collect();
        city.owned_tiles.extend(ring_tiles);
    }

    // 自动分配初始市民到产出最多的地块（文明5规则）
    // 城市中心不需要分配市民即可自动工作
    if let Some(tile_map) = &tile_map {
        let tile_map = &tile_map.0;
        let ruleset = &map_params.0.ruleset;
        assign_worker_tiles(
            &mut city,
            tile_map,
            ruleset,
            &tile_entity_map,
            &improvement_query,
        );
    }

    commands
        .entity(target_tile_entity)
        .insert(city)
        .insert(Owner(owner.0))
        .insert(CityYields::default())
        // 在地块上添加城市中心设施
        .insert(TileImprovementComponent(TileImprovement::CityCenter))
        // 添加城市图片渲染（Pickable::IGNORE 确保点击事件穿透到地块）
        .with_children(|parent| {
            parent.spawn((
                Sprite {
                    custom_size: Some(tile_pixel_size),
                    image: materials.texture_handle("city"),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 6.0),
                Pickable::IGNORE,
            ));
        });

    // 移除移民单位
    commands.entity(unit_entity).despawn();
    info!("Founded city on tile {:?}", target_tile);
}

// ============ 城市边界渲染系统 ============

/// 渲染城市边界 - 只绘制城市区域的外轮廓边（不填充内部）
fn render_city_borders(
    mut commands: Commands,
    city_query: Query<&City>,
    tile_entity_map: Res<TileEntityMap>,
    tile_map: Option<Res<TileMapRes>>,
    border_query: Query<Entity, With<CityBorderHighlight>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut cached_handles: Local<Option<(Handle<Mesh>, Handle<ColorMaterial>)>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };
    let grid = tile_map.0.world_grid.grid;

    // 清除旧的边界高亮
    for entity in border_query.iter() {
        commands.entity(entity).despawn();
    }

    // 缓存材质句柄，避免每帧创建新资源
    let border_material = match &*cached_handles {
        Some((_, material)) => material.clone(),
        None => {
            let material =
                color_materials.add(ColorMaterial::from_color(Color::srgba(0.3, 0.7, 1.0, 0.9)));
            *cached_handles = Some((Handle::default(), material.clone()));
            material
        }
    };

    // 获取六边形6条边的方向（与 hex_edge_mesh 的 edge_index 对应）
    let edge_directions = grid.edge_direction_array();

    // 为每个城市绘制外轮廓边
    for city in city_query.iter() {
        // 城市拥有的地块集合
        let city_tile_set: std::collections::HashSet<Tile> =
            city.owned_tiles.iter().copied().collect();

        for &tile in &city.owned_tiles {
            // 检查每个方向的邻居，如果邻居不属于城市，则该方向的边是外轮廓边
            for (edge_idx, &direction) in edge_directions.iter().enumerate() {
                let is_border = match tile.neighbor_tile(direction, grid) {
                    Some(neighbor) => !city_tile_set.contains(&neighbor),
                    None => true, // 地图边界
                };

                if is_border {
                    // 该边是外轮廓边，绘制单条边
                    let edge_mesh = meshes.add(hex_edge_mesh(&grid, edge_idx));
                    let entity = tile_entity_map.get(tile);
                    commands.entity(entity).with_children(|parent| {
                        parent.spawn((
                            Mesh2d(edge_mesh.clone()),
                            MeshMaterial2d(border_material.clone()),
                            Transform::from_xyz(0.0, 0.0, 1.0),
                            CityBorderHighlight,
                            Pickable::IGNORE,
                        ));
                    });
                }
            }
        }
    }
}
