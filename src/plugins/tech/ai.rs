//! AI 科技选择
//!
//! 管理 AI 文明自动选择要研究的科技。

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Technology;

use crate::{
    Enemy, ResolutionPhase, TurnManager, TurnState,
    plugins::tech::{TechState, TechStateManager},
    resources::MapParametersRes,
};

use super::components::ResearchingTech;

/// AI 科技选择插件
pub struct AiTechPlugin;

impl Plugin for AiTechPlugin {
    fn build(&self, app: &mut App) {
        // AI 只在敌方回合激活
        app.add_systems(
            OnEnter(TurnState::Start),
            ai_select_tech.in_set(ResolutionPhase::AiSelectTech),
        );
    }
}

fn ai_select_tech(
    manager: Res<TurnManager>,
    mut enemy_query: Query<(&mut ResearchingTech, &TechStateManager), With<Enemy>>,
    map_params: Res<MapParametersRes>,
) {
    let entity = manager.current_nation_entity();
    let Ok((mut researching_tech, tech_state_manager)) = enemy_query.get_mut(entity) else {
        unreachable!("Enemy tech manager not found")
    };
    if researching_tech.0.is_some() {
        return; // 已经在研究中
    }

    let mut available: Vec<(Technology, i32)> = map_params
        .0
        .ruleset
        .technologies
        .iter()
        .filter(|&(tech, _)| {
            matches!(
                tech_state_manager.0[tech],
                TechState::Available | TechState::ResearchedAndRepeatable
            )
        })
        .map(|(tech, info)| (tech, info.cost))
        .collect();
    available.sort_by_key(|(_, cost)| *cost);
    if let Some(&(tech, _)) = available.first() {
        researching_tech.0 = Some(tech);
        info!("AI chooses to research {:?}", tech);
    }
}
