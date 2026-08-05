//! 时代更新插件
//!
//! 监听科技研发成功事件，当满足以下任一条件时，当前文明进入下一个时代：
//! 1. 当前时代的所有科技都已研发完成
//! 2. 研发了下一个时代的科技

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Era};
use enum_map::Enum;

use crate::{
    AppState, NationComponent,
    plugins::tech::{TechResearchedMessage, TechState, TechStateManager},
    resources::{GameSettings, MapParametersRes},
};

/// 文明当前所处时代组件
#[derive(Component)]
pub struct EraComponent(pub Era);

/// 时代更新插件
pub struct EraPlugin;

impl Plugin for EraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::GameStart), insert_era_for_every_nation)
            .add_systems(Update, update_era_on_tech_researched);
    }
}

/// 插入时代组件到所有国家
fn insert_era_for_every_nation(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    query_nation: Query<Entity, With<NationComponent>>,
) {
    for entity in query_nation.iter() {
        commands
            .entity(entity)
            .insert(EraComponent(game_settings.start_era));
    }
}

/// 监听科技研发成功消息，更新文明时代
fn update_era_on_tech_researched(
    mut tech_researched_messages: MessageReader<TechResearchedMessage>,
    map_params: Res<MapParametersRes>,
    mut query: Query<(&mut EraComponent, &TechStateManager)>,
) {
    for message in tech_researched_messages.read() {
        let Ok((mut era_component, tech_state_manager)) = query.get_mut(message.nation) else {
            continue;
        };

        let current_era = era_component.0;

        // 如果已经是最后一个时代，无需更新
        let Some(next_era) = next_era(current_era) else {
            continue;
        };

        // 情况1：研发了下一个时代的科技
        let researched_tech_era = map_params.0.ruleset.technologies[message.tech].era.clone();
        if researched_tech_era == next_era.as_str() {
            era_component.0 = next_era;
            info!(
                "Nation {:?} has advanced to the next era: {:?}",
                message.nation, next_era
            );
            continue;
        }

        // 情况2：当前时代的所有科技都已研发完成
        if all_techs_of_era_researched(current_era, &tech_state_manager, &map_params) {
            info!(
                "Nation {:?} has advanced to the next era: {:?}",
                message.nation, next_era
            );
            era_component.0 = next_era;
        }
    }
}

/// 获取下一个时代，如果当前是最后一个时代则返回 None
fn next_era(era: Era) -> Option<Era> {
    let index = era.into_usize();
    if index + 1 < Era::LENGTH {
        Some(Era::from_usize(index + 1))
    } else {
        None
    }
}

/// 检查当前时代的所有科技是否都已研发完成
fn all_techs_of_era_researched(
    era: Era,
    tech_state_manager: &TechStateManager,
    map_params: &MapParametersRes,
) -> bool {
    let ruleset = &map_params.0.ruleset;
    let era_str = era.as_str();

    ruleset
        .technologies
        .iter()
        .filter(|(_, tech_info)| tech_info.era == era_str)
        .all(|(tech, _)| tech_state_manager.0[tech] == TechState::Researched)
}
