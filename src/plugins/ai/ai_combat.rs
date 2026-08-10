//! AI 战斗插件
//!
//! 管理 AI 文明自动进行战斗决策，包括：
//! - AI 单位自动攻击相邻的敌方单位
//! - AI 单位移动决策（未来扩展）

use bevy::prelude::*;

use crate::{
    AttackRequestMessage, Enemy, TurnPhase,
    components::{Health, Movement, Owner, Strength},
    resources::TileMapRes,
};

/// AI 战斗插件
/// TODO: 需要全部重写
pub struct AiCombatPlugin;

impl Plugin for AiCombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (ai_attack_system,).run_if(in_state(TurnPhase::Enemy)),
        );
    }
}

/// AI 攻击系统 - 敌方文明自动攻击
///
/// 遍历所有敌方单位，检查是否有相邻的玩家单位，如果有则发起攻击。
///
/// TODO：Warning此处完全错误，单位不可能有Enemy组件
fn ai_attack_system(
    enemy_query: Query<(Entity, &Owner, &Strength, &Health, &Movement, &ChildOf), With<Enemy>>,
    player_unit_query: Query<(Entity, &Owner, &ChildOf)>,
    tile_map: Option<Res<TileMapRes>>,
    mut commands: Commands,
) {
    let Some(_tile_map) = tile_map else {
        return;
    };

    for (
        enemy_entity,
        enemy_owner,
        enemy_strength,
        _enemy_health,
        enemy_movement,
        enemy_child_of,
    ) in enemy_query.iter()
    {
        // 跳过无法移动或已行动的 AI 单位
        if enemy_movement.current == 0 {
            continue;
        }

        // 跳过没有战斗力的单位（平民单位）
        if enemy_strength.0 == 0 {
            continue;
        }

        let enemy_nation = enemy_owner.0;

        // 获取敌人所在位置
        let enemy_tile_entity = enemy_child_of.0;

        // 获取敌人相邻地块上的所有单位
        for (player_entity, player_owner, player_child_of) in player_unit_query.iter() {
            // 跳过不是玩家的单位
            let is_player = player_owner.0 != enemy_nation;
            if !is_player {
                continue;
            }

            // 获取玩家单位所在的地块
            let player_tile_entity = player_child_of.0;

            // 如果玩家单位就在敌人单位所在的地块上，直接攻击
            if player_tile_entity == enemy_tile_entity {
                info!("AI attacking player unit on same tile!");
                commands.trigger(AttackRequestMessage {
                    attacker: enemy_entity,
                    target: player_entity,
                });
                commands.entity(enemy_entity).insert(Movement {
                    current: 0,
                    max: enemy_movement.max,
                });
                break;
            }
        }
    }
}
