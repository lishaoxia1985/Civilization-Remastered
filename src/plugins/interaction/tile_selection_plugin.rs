//! 地块选择插件
//!
//! 统一管理点击地块的选中逻辑，在本地块的（城市 > 军事单位 > 平民单位）之间
//! 循环切换选中，仅处理当前玩家回合内的本国目标：
//! - 点击单位实体 → 直接选中该单位
//! - 点击地块 → 切换选择该地块上的目标；若当前已选中本地块的某个目标，则循环到下一个
//! - 点击无任何可选目标的地块 → 取消所有选中
//!
//! 选中结果通过写入 [`SelectedCity`] / [`SelectedUnit`] 标记体现，消费方
//! （单位操作菜单、城市面板等）各自监听对应标记。

use bevy::prelude::*;

use crate::{
    NationComponent, Player, ScreenState, TurnManager,
    components::{
        City, Civilian, Military, MoveButtonActive, Owner, SelectedCity, SelectedUnit,
        UnitComponent, WorldTile,
    },
};

/// 地块上的可选目标类型：城市（挂在地块实体上）或单位（地块的子实体）
enum TileSelectTarget {
    City(Entity),
    Unit(Entity),
}

/// 地块选择插件
pub struct TileSelectionPlugin;

impl Plugin for TileSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_tile_selection.run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

/// 处理地块点击选择
fn handle_tile_selection(
    mut click_events: MessageReader<Pointer<Click>>,
    mut commands: Commands,
    unit_query: Query<(Entity, &Owner, &ChildOf), With<UnitComponent>>,
    tile_query: Query<(Entity, &WorldTile, Option<&Children>)>,
    city_query: Query<(&City, &Owner)>,
    military_query: Query<(), With<Military>>,
    civilian_query: Query<(), With<Civilian>>,
    player_query: Query<&NationComponent, With<Player>>,
    selected_unit_query: Query<Entity, With<SelectedUnit>>,
    selected_city_query: Query<Entity, With<SelectedCity>>,
    move_button_query: Query<Entity, With<MoveButtonActive>>,
    turn_manager: Res<TurnManager>,
) {
    // 移动模式激活时，完全不处理选择，交由移动系统
    if move_button_query.single().is_ok() {
        return;
    }

    // 仅在玩家回合处理选择
    let current_entity = turn_manager.current_nation_entity();
    let Ok(nation_component) = player_query.get(current_entity) else {
        return;
    };
    let player_nation = nation_component.0;

    // 消除 MoveButtonActive 状态的辅助闭包
    let deactivate_move_button = |commands: &mut Commands| {
        if let Ok(entity) = move_button_query.single() {
            commands
                .entity(entity)
                .remove::<MoveButtonActive>()
                .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
        }
    };

    for click in click_events.read() {
        // 1. 点击的是单位实体 → 直接选中该单位
        if let Ok((unit_entity, owner, _)) = unit_query.get(click.event_target()) {
            // 消除 MoveButtonActive 状态
            deactivate_move_button(&mut commands);

            // 只处理本国单位
            if owner.0 == player_nation {
                // 取消旧的选中态
                for e in selected_city_query.iter() {
                    commands.entity(e).remove::<SelectedCity>();
                }
                for e in selected_unit_query.iter() {
                    commands.entity(e).remove::<SelectedUnit>();
                }
                commands.entity(unit_entity).insert(SelectedUnit);
            }
            continue;
        }

        // 2. 点击的是地块本身
        let Ok((tile_entity, _, _)) = tile_query.get(click.event_target()) else {
            continue;
        };

        // 取消旧选中并关闭移动模式
        for e in selected_city_query.iter() {
            commands.entity(e).remove::<SelectedCity>();
        }
        for e in selected_unit_query.iter() {
            commands.entity(e).remove::<SelectedUnit>();
        }
        deactivate_move_button(&mut commands);

        // 收集本地块上属于当前玩家的可选目标，顺序：城市 > 军事单位 > 平民单位
        // 城市组件直接挂在地块实体上
        let mut targets: Vec<TileSelectTarget> = Vec::new();

        if let Ok((_, city_owner)) = city_query.get(tile_entity) {
            if city_owner.0 == player_nation {
                targets.push(TileSelectTarget::City(tile_entity));
            }
        }

        // 军事单位
        for (entity, owner, child_of) in unit_query.iter() {
            if child_of.0 == tile_entity
                && owner.0 == player_nation
                && military_query.get(entity).is_ok()
            {
                targets.push(TileSelectTarget::Unit(entity));
            }
        }

        // 平民单位
        for (entity, owner, child_of) in unit_query.iter() {
            if child_of.0 == tile_entity
                && owner.0 == player_nation
                && civilian_query.get(entity).is_ok()
            {
                targets.push(TileSelectTarget::Unit(entity));
            }
        }

        // 无任何可选目标（空地块）→ 保持取消选中状态
        if targets.is_empty() {
            continue;
        }

        // 确定当前处于本地块的目标，用于循环切换
        let current_city_on_tile = selected_city_query.single().ok() == Some(tile_entity);
        let current_unit_on_tile = selected_unit_query.single().ok();

        // 注意：上方已清除选中态，因此此处根据点击前的状态计算下一个目标
        let current_index = if current_city_on_tile {
            targets
                .iter()
                .position(|t| matches!(t, TileSelectTarget::City(_)))
        } else if let Some(unit) = current_unit_on_tile {
            // 仅当该单位位于本地块时才参与循环
            let on_this_tile = unit_query
                .get(unit)
                .map(|(_, _, c)| c.0 == tile_entity)
                .unwrap_or(false);
            if on_this_tile {
                targets
                    .iter()
                    .position(|t| matches!(t, TileSelectTarget::Unit(e) if *e == unit))
            } else {
                None
            }
        } else {
            None
        };

        let next_index = match current_index {
            Some(idx) => (idx + 1) % targets.len(),
            None => 0,
        };

        // 应用新的选中
        match targets[next_index] {
            TileSelectTarget::City(tile) => {
                commands.entity(tile).insert(SelectedCity);
            }
            TileSelectTarget::Unit(unit) => {
                commands.entity(unit).insert(SelectedUnit);
            }
        }
    }
}
