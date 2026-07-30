use bevy::prelude::*;

use crate::{
    AppState, Player, ResolutionPhase, TurnEndMessage, TurnManager, TurnPhase, TurnStartMessage,
};

pub struct TurnPlugin;

impl Plugin for TurnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TurnManager>()
            .add_message::<TurnStartMessage>()
            .add_message::<TurnEndMessage>()
            // 定义收到开始回合事件时的执行顺序：Science -> Production （非必须 你需要将那些处理事件的系统分别放到对应的系统集中，然后这些系统可以根据此顺序执行）
            .configure_sets(
                Update,
                (
                    ResolutionPhase::Science,
                    ResolutionPhase::Production,
                    ResolutionPhase::AiSelectTech.run_if(in_state(TurnPhase::EnemyTurn)),
                )
                    .chain()
                    .run_if(in_state(AppState::GameStart)),
            )
            .add_systems(
                Update,
                auto_end_enemy_turn
                    .after(ResolutionPhase::Production)
                    .run_if(in_state(AppState::GameStart)),
            )
            .add_systems(Update, advance_turn_queue);
    }
}

fn auto_end_enemy_turn(
    current_phase: Res<State<TurnPhase>>,
    mut turn_end_messages: MessageWriter<TurnEndMessage>,
    manager: Res<TurnManager>,
) {
    // 只在敌人回合时自动发送结束消息
    if *current_phase.get() == TurnPhase::EnemyTurn {
        let entity = manager.turn_queue[manager.current_index];
        turn_end_messages.write(TurnEndMessage { entity });
        info!("Auto-ending enemy turn for {}", entity);
    }
}

fn advance_turn_queue(
    mut manager: ResMut<TurnManager>,
    mut turn_end_messages: MessageReader<TurnEndMessage>,
    mut turn_start_messages: MessageWriter<TurnStartMessage>,
    player_query: Single<Entity, With<Player>>,
    mut next_state: ResMut<NextState<TurnPhase>>,
) {
    let player_entity = player_query.into_inner();
    for _event in turn_end_messages.read() {
        if !manager.turn_queue.is_empty() {
            manager.current_index = (manager.current_index + 1) % manager.turn_queue.len();

            // 上一个玩家已经结束回合，获取当前玩家并进入当前玩家的回合
            let entity = manager.turn_queue[manager.current_index];

            turn_start_messages.write(TurnStartMessage { entity });

            info!("Starting turn resolution for {}", entity);

            if player_entity == entity {
                next_state.set(TurnPhase::PlayTurn);
            } else {
                next_state.set(TurnPhase::EnemyTurn);
            }

            // 【新增】如果索引回到 0，说明所有实体都行动完毕，新回合开始
            if manager.current_index == 0 {
                manager.turn_number += 1;
                println!("📅 [回合系统] 第 {} 回合开始！", manager.turn_number);
            }
        }
    }
}
