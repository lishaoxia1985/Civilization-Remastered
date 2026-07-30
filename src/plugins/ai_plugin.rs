use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Technology;

use crate::{
    Enemy, ResolutionPhase, TurnStartMessage,
    resources::{MapParametersRes, ResearchingTech, TechManager},
};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // AI 只在敌方回合激活
        app.add_systems(Update, ai_select_tech.in_set(ResolutionPhase::AiSelectTech));
    }
}

fn ai_select_tech(
    mut turn_start_messages: MessageReader<TurnStartMessage>,
    mut enemy_query: Query<(&mut ResearchingTech, &mut TechManager), With<Enemy>>,
    map_params: Res<MapParametersRes>,
) {
    for message in turn_start_messages.read() {
        let entity = message.entity;
        let Ok((mut researching_tech, tech_manager)) = enemy_query.get_mut(entity) else {
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
            .filter(|(tech, _)| tech_manager.can_be_researched(*tech, &map_params))
            .map(|(tech, info)| (tech, info.cost))
            .collect();
        available.sort_by_key(|(_, cost)| *cost);
        if let Some(&(tech, _)) = available.first() {
            researching_tech.0 = Some(tech);
            info!("AI chooses to research {:?}", tech);
        }
    }
}
