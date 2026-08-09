//! 建造系统插件
//!
//! 管理平民单位的建造逻辑，包括：
//! - 工人单位建造地块设施（农场、矿山等）
//! - 移民单位建立城市
//! - 地块设施组件管理

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Unit;

use crate::{
    BuildRequestMessage, FoundCityRequestMessage,
    components::{City, Owner, TileImprovementComponent, UnitComponent},
    resources::TileEntityMap,
};

/// 建造插件
pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_build_request)
            .add_observer(handle_found_city_request);
    }
}

/// 处理建造请求
fn handle_build_request(
    event: On<BuildRequestMessage>,
    mut commands: Commands,
    unit_query: Query<(Entity, &ChildOf, &UnitComponent, &Owner)>,
    tile_entity_map: Res<TileEntityMap>,
    improvement_query: Query<(), With<TileImprovementComponent>>,
) {
    let build_request = event.event();
    let unit_entity = build_request.unit;
    let target_tile = build_request.target_tile;
    let improvement = build_request.improvement;

    let Ok((_, _child_of, unit_component, _owner)) = unit_query.get(unit_entity) else {
        return;
    };

    // 只有工人单位可以建造设施
    let is_worker = matches!(
        unit_component,
        UnitComponent::Civilian(Unit::Worker)
    );
    if !is_worker {
        return;
    }

    let Some(target_tile_entity) = tile_entity_map.get(target_tile) else {
        return;
    };

    // 检查目标地块是否已有设施
    if improvement_query.get(target_tile_entity).is_ok() {
        warn!("Tile already has an improvement");
        return;
    }

    // 在目标地块上添加设施组件
    commands
        .entity(target_tile_entity)
        .insert(TileImprovementComponent(improvement));

    info!("Built {:?} on tile {:?}", improvement, target_tile);
}

/// 处理建城请求
fn handle_found_city_request(
    event: On<FoundCityRequestMessage>,
    mut commands: Commands,
    unit_query: Query<(Entity, &ChildOf, &UnitComponent, &Owner)>,
    tile_entity_map: Res<TileEntityMap>,
    city_query: Query<(), With<City>>,
) {
    let found_city_request = event.event();
    let unit_entity = found_city_request.unit;
    let target_tile = found_city_request.target_tile;

    let Ok((_, _child_of, unit_component, _owner)) = unit_query.get(unit_entity) else {
        return;
    };

    // 只有移民单位可以建立城市
    let is_settler = matches!(
        unit_component,
        UnitComponent::Civilian(civ_map_generator::ruleset::enums::Unit::Settler)
    );
    if !is_settler {
        return;
    }

    let Some(target_tile_entity) = tile_entity_map.get(target_tile) else {
        return;
    };

    // 检查目标地块是否已有城市
    if city_query.get(target_tile_entity).is_ok() {
        warn!("Tile already has a city");
        return;
    }

    // 在目标地块上添加城市组件
    commands.entity(target_tile_entity).insert(City {
        name: format!("City at tile {}", target_tile.index()),
    });

    // 移除移民单位
    commands.entity(unit_entity).despawn();

    info!("Founded city on tile {:?}", target_tile);
}
