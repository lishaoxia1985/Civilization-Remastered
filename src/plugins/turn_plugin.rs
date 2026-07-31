use bevy::prelude::*;

use crate::{
    AppState, NationComponent, Player, ResolutionPhase, TurnManager, TurnPhase, TurnState,
};

pub struct TurnPlugin;

impl Plugin for TurnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::GameStart), initialize_turn_system)
            // 定义收到开始回合事件时的执行顺序：Science -> Production （非必须 你需要将那些处理事件的系统分别放到对应的系统集中，然后这些系统可以根据此顺序执行）
            .configure_sets(
                OnEnter(TurnState::Start),
                (
                    ResolutionPhase::Science,
                    ResolutionPhase::Production,
                    ResolutionPhase::AiSelectTech.run_if(in_state(TurnPhase::EnemyTurn)),
                    ResolutionPhase::AutoEndTurn.run_if(in_state(TurnPhase::EnemyTurn)),
                )
                    .chain()
                    .run_if(in_state(AppState::GameStart)),
            )
            .add_systems(
                OnEnter(TurnState::Start),
                auto_end_enemy_turn.in_set(ResolutionPhase::AutoEndTurn),
            )
            .add_systems(OnEnter(TurnState::End), advance_turn_queue);
    }
}

fn initialize_turn_system(
    mut commands: Commands,
    mut next_turn_state: ResMut<NextState<TurnState>>,
    mut next_turn_phase: ResMut<NextState<TurnPhase>>,
    query_nations: Query<Entity, With<NationComponent>>,
    player_query: Single<Entity, With<Player>>,
) {
    let player_entity = player_query.into_inner();
    let turn_queue: Vec<_> = query_nations.iter().sort::<Entity>().collect();
    let entity = *&turn_queue[0];
    commands.insert_resource(TurnManager {
        turn_queue,
        current_index: 0,
        turn_number: 0,
    });
    next_turn_state.set(TurnState::Start);
    if player_entity == entity {
        next_turn_phase.set(TurnPhase::PlayTurn);
    } else {
        next_turn_phase.set(TurnPhase::EnemyTurn);
    }
}

fn auto_end_enemy_turn(
    current_phase: Res<State<TurnPhase>>,
    mut next_state: ResMut<NextState<TurnState>>,
    manager: Res<TurnManager>,
) {
    // 只在敌人回合时自动发送结束消息
    if *current_phase.get() == TurnPhase::EnemyTurn {
        let entity = manager.current_nation_entity();
        next_state.set(TurnState::End);
        info!("Auto-ending enemy turn for {}", entity);
    }
}

fn advance_turn_queue(
    mut manager: ResMut<TurnManager>,
    player_query: Single<Entity, With<Player>>,
    mut next_turn_state: ResMut<NextState<TurnState>>,
    mut next_turn_phase: ResMut<NextState<TurnPhase>>,
) {
    let player_entity = player_query.into_inner();
    if !manager.turn_queue.is_empty() {
        manager.current_index = (manager.current_index + 1) % manager.turn_queue.len();

        // 上一个玩家已经结束回合，获取当前玩家并进入当前玩家的回合
        let entity = manager.turn_queue[manager.current_index];

        next_turn_state.set(TurnState::Start);

        info!("Starting turn resolution for {}", entity);

        if player_entity == entity {
            next_turn_phase.set(TurnPhase::PlayTurn);
        } else {
            next_turn_phase.set(TurnPhase::EnemyTurn);
        }

        // 【新增】如果索引回到 0，说明所有实体都行动完毕，新回合开始
        if manager.current_index == 0 {
            manager.turn_number += 1;
            println!("📅 [回合系统] 第 {} 回合开始！", manager.turn_number);
        }
    }
}
