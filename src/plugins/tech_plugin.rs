use std::{
    cmp::{max, min},
    collections::HashSet,
};

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Era, Technology};
use enum_map::Enum;

use crate::{
    AppState, NationComponent, Player, ResolutionPhase, SciencePerTurn, TechResearchedMessage,
    TurnManager, TurnState,
    resources::{
        GameSettings, MapParametersRes, OverflowScience, ResearchedTechList, ResearchingTech,
        ScienceFromResearchAgreements, ScienceOfLast8Turns, TechManager, TechProgress,
    },
};

pub struct TechPlugin;

impl Plugin for TechPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TechResearchedMessage>()
            .add_systems(
                OnEnter(AppState::GameStart),
                insert_tech_manager_for_every_nation,
            )
            .add_systems(
                OnEnter(TurnState::Start),
                process_science_on_turn_start.in_set(ResolutionPhase::Science),
            );
    }
}

/// 插入科技管理组件
///
/// 根据游戏设置中的起始时代初始化每个文明的科技管理器，
/// 并将起始时代之前的所有时代的科技标记为已经研发。
fn insert_tech_manager_for_every_nation(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
    query_nation: Query<Entity, With<NationComponent>>,
) {
    let start_era = game_settings.start_era;
    let start_era_index = start_era.into_usize();
    let ruleset = &map_params.0.ruleset;

    // 收集起始时代之前的所有时代的科技
    let mut pre_start_era_techs = HashSet::new();
    for (tech, tech_info) in ruleset.technologies.iter() {
        let tech_era = Era::from_str(&tech_info.era);
        if tech_era.into_usize() < start_era_index {
            pre_start_era_techs.insert(tech);
        }
    }

    for entity in query_nation.iter() {
        commands.entity(entity).insert((
            TechManager::new(start_era),
            ResearchedTechList(pre_start_era_techs.clone()),
        ));
    }
}

fn process_science_on_turn_start(
    manager: Res<TurnManager>,
    mut query: Query<(
        Entity,
        &mut ResearchingTech,
        &mut TechManager,
        &mut TechProgress,
        &mut ResearchedTechList,
        &mut OverflowScience,
        &mut ScienceOfLast8Turns,
        &mut ScienceFromResearchAgreements,
        &SciencePerTurn,
        Option<&Player>,
    )>,
    mut tech_complete_messages: MessageWriter<TechResearchedMessage>,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
) {
    let turn = manager.turn_number;
    if turn == 0 {
        // 第0回合不处理科技，即开始游戏的回合暂时不处理科技
        return;
    }

    let entity = manager.current_nation_entity();
    if let Ok((
        entity,
        mut researching_tech,
        tech_manager,
        mut tech_progress,
        mut researched_techs,
        mut overflow_science,
        mut science_of_last_8_turns,
        mut science_from_research_agreements,
        science_per_turn,
        player,
    )) = query.get_mut(entity)
    {
        let is_player = player.is_some();

        let science_per_turn = science_per_turn.0;

        // 存储最近8回合的科技值
        // TODO: 用于计算消耗大科学家时获得的科技值,我们暂时没有实现
        science_of_last_8_turns.0[turn as usize % 8] = science_per_turn;

        let current_tech = match researching_tech.0 {
            Some(tech) => tech,
            None => panic!("No technology is being researched"),
        };

        // 获取文明当前回合的科技值产出
        let mut final_science = science_per_turn;

        // TODO: 添加研究协议提供的科技值,当前未实现研究协议相关逻辑
        if science_from_research_agreements.0 != 0 {
            let boost = science_from_research_agreements.0 / 3;
            final_science += boost;
            science_from_research_agreements.0 = 0;
        }

        // 处理上次科技研发成功时溢出的科技值, 添加到当前科技值中
        if overflow_science.0 != 0 {
            final_science += overflow_science.0;
            overflow_science.0 = 0;
        }

        let cost = tech_manager.cost_of_tech(current_tech, is_player, &game_settings, &map_params);

        // 获取当前科技的进度，即其已投入的科技点数
        let current = tech_progress.0.entry(current_tech).or_insert(0);

        // 计算最终投入该科技的科技点数
        *current += final_science;

        // 如果已投入的科技点数达到科技消耗，则完成当前科技的研究
        if *current >= cost {
            // 获取当前科技研发成功后溢出的科技值
            let extra_science = *current - cost;
            overflow_science.0 +=
                limit_overflow_science(extra_science, current_tech, science_per_turn, &map_params);

            // 删除该科技的研发进度
            tech_progress.0.remove(&current_tech);
            // 清除当前研发的科技
            researching_tech.0 = None;

            // 添加科技, 如果科技是`Technology::FutureTech`, 且已经添加过, 则此处添加失败，
            // 如此我们无需再发送科技研发成功消息？
            // TODO: 或许`Technology::FutureTech`研发过，我们照样需要发送消息，读取该消息来更新胜利分数
            if researched_techs.0.insert(current_tech) {
                tech_complete_messages.write(TechResearchedMessage {
                    nation: entity,
                    tech: current_tech,
                });
            }
        }
    };
}

/// 限制溢出科技点，防止过多结转到下一个科技
pub fn limit_overflow_science(
    overflow: i32,
    current_tech: Technology,
    science_per_turn: i32,
    map_params: &MapParametersRes,
) -> i32 {
    let ruleset = &map_params.0.ruleset;
    let tech_cost = ruleset.technologies[current_tech].cost;
    min(overflow, max(science_per_turn * 5, tech_cost))
}
