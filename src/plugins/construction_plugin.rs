//! 地块设施建造插件
//!
//! 仅用于管理 TileImprovement（地块改良设施）的建造。
//! 工人单位发起建造请求后，需要持续多个回合才能完成建造。
//!
//! 建造流程：
//! 1. 工人单位发出 BuildRequestMessage
//! 2. 目标地块插入 TileImprovementBuildProgress 组件
//! 3. 每回合开始时进度 +1
//! 4. 进度达到所需回合数后，地块获得 TileImprovementComponent（建造完成）

use bevy::prelude::*;

use crate::{
    BuildRequestMessage, NationComponent, ResolutionPhase, TurnManager, TurnState,
    components::{
        Civilian, Owner, TileImprovementBuildProgress, TileImprovementComponent, UnitComponent,
    },
    resources::{MapParametersRes, TileEntityMap},
};

/// 地块设施建造插件
pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_build_request).add_systems(
            OnEnter(TurnState::Start),
            process_improvement_progress.in_set(ResolutionPhase::Production),
        );
    }
}

/// 处理建造请求 - 在地块上开始建造设施（需要多回合）
fn handle_build_request(
    event: On<BuildRequestMessage>,
    mut commands: Commands,
    unit_query: Query<(Entity, &ChildOf, &UnitComponent, &Owner), With<Civilian>>,
    tile_entity_map: Res<TileEntityMap>,
    improvement_query: Query<(), With<TileImprovementComponent>>,
    build_progress_query: Query<(), With<TileImprovementBuildProgress>>,
    map_params: Res<MapParametersRes>,
) {
    let build_request = event.event();
    let unit_entity = build_request.unit;
    let target_tile = build_request.target_tile;
    let improvement = build_request.improvement;

    let Ok((_, _child_of, unit_component, owner)) = unit_query.get(unit_entity) else {
        return;
    };

    // 只有工人单位可以建造设施
    if unit_component.0 != civ_map_generator::ruleset::enums::Unit::Worker {
        return;
    }

    let target_tile_entity = tile_entity_map.get(target_tile);

    // 检查目标地块是否已有设施或正在建造设施
    if improvement_query.get(target_tile_entity).is_ok() {
        warn!("Tile already has an improvement");
        return;
    }
    if build_progress_query.get(target_tile_entity).is_ok() {
        warn!("Tile already has an improvement under construction");
        return;
    }

    // 获取该设施所需的建造回合数（来自 ruleset）
    let ruleset = &map_params.0.ruleset;
    let improvement_info = &ruleset.tile_improvements[improvement];
    let total_turns = improvement_info.turns_to_build.max(1);

    // 在地块上添加建造进度组件
    commands
        .entity(target_tile_entity)
        .insert(TileImprovementBuildProgress {
            improvement,
            progress: 0,
            total_turns,
            owner: owner.0,
            worker: unit_entity,
        });

    info!(
        "Worker started building {:?} on tile {:?} ({} turns)",
        improvement, target_tile, total_turns
    );
}

/// 每回合处理设施建造进度
fn process_improvement_progress(
    manager: Res<TurnManager>,
    nation_query: Query<&NationComponent>,
    mut commands: Commands,
    mut build_progress_query: Query<(Entity, &mut TileImprovementBuildProgress)>,
) {
    // 第0回合不处理建造进度（开始游戏的回合）
    if manager.turn_number == 0 {
        return;
    }

    // 获取当前回合的 Nation
    let current_nation_entity = manager.current_nation_entity();
    let Ok(nation_component) = nation_query.get(current_nation_entity) else {
        return;
    };
    let current_nation = nation_component.0;

    for (tile_entity, mut progress) in build_progress_query.iter_mut() {
        // 只处理属于当前回合文明的建造进度
        if progress.owner != current_nation {
            continue;
        }

        progress.progress += 1;
        let improvement = progress.improvement;

        info!(
            "Building {:?} progress: {}/{}",
            improvement, progress.progress, progress.total_turns
        );

        // 建造完成
        if progress.progress >= progress.total_turns {
            commands
                .entity(tile_entity)
                .remove::<TileImprovementBuildProgress>()
                .insert(TileImprovementComponent(improvement));

            info!(
                "Finished building {:?} on tile {:?}",
                improvement, tile_entity
            );
        }
    }
}
