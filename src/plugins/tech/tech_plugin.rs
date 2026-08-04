//! 科技插件
//!
//! 管理科技研究流程，包括：
//! - 为每个文明插入科技相关组件
//! - 每回合处理科研值并结算科技研发
//! - 发送科技研发完成消息

use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
};

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Era, Technology};
use enum_map::Enum;

use crate::{
    AppState, NationComponent, Player, ResolutionPhase, SciencePerTurn, TurnManager, TurnState,
    plugins::tech::TechCostManager,
    resources::{GameSettings, MapParametersRes},
};

use super::{
    components::{
        OverflowScience, ResearchedTechList, ResearchingTech, ScienceFromResearchAgreements,
        ScienceOfLast8Turns, TechProgressManager, TechSystem,
    },
    functions::cost_of_tech,
    messages::TechResearchedMessage,
};

/// 科技插件
pub struct TechPlugin;

impl Plugin for TechPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TechResearchedMessage>()
            .add_systems(
                OnEnter(AppState::GameStart),
                insert_tech_system_for_every_nation,
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
fn insert_tech_system_for_every_nation(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
    query_nation: Query<(Entity, Option<&Player>), With<NationComponent>>,
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

    let tech_costs_of_player: HashMap<Technology, i32> = ruleset
        .technologies
        .iter()
        .filter(|(_, tech_info)| {
            let tech_era = Era::from_str(&tech_info.era);
            tech_era.into_usize() >= start_era_index
        })
        .map(|(tech, _)| (tech, cost_of_tech(tech, true, &game_settings, &map_params)))
        .collect();

    let tech_costs_of_enemy: HashMap<Technology, i32> = ruleset
        .technologies
        .iter()
        .filter(|(_, tech_info)| {
            let tech_era = Era::from_str(&tech_info.era);
            tech_era.into_usize() >= start_era_index
        })
        .map(|(tech, _)| (tech, cost_of_tech(tech, false, &game_settings, &map_params)))
        .collect();

    for (entity, player) in query_nation.iter() {
        commands.entity(entity).insert((
            TechSystem,
            ResearchedTechList(pre_start_era_techs.clone()),
            TechCostManager(if player.is_some() {
                tech_costs_of_player.clone()
            } else {
                tech_costs_of_enemy.clone()
            }),
        ));
    }
}

/// 每回合开始时处理科研值
fn process_science_on_turn_start(
    manager: Res<TurnManager>,
    mut query: Query<(
        Entity,
        &mut ResearchingTech,
        &mut TechProgressManager,
        &mut ResearchedTechList,
        &mut TechCostManager,
        &mut OverflowScience,
        &mut ScienceOfLast8Turns,
        &mut ScienceFromResearchAgreements,
        &SciencePerTurn,
    )>,
    mut tech_complete_messages: MessageWriter<TechResearchedMessage>,
    map_params: Res<MapParametersRes>,
) {
    let turn = manager.turn_number;
    if turn == 0 {
        // 第0回合不处理科技，即开始游戏的回合暂时不处理科技
        // 因为开始的回合未积累任何科研值，且未选择科技进行研究
        return;
    }

    let entity = manager.current_nation_entity();
    if let Ok((
        entity,
        mut researching_tech,
        mut tech_progress,
        mut researched_techs,
        mut tech_cost_manager,
        mut overflow_science,
        mut science_of_last_8_turns,
        mut science_from_research_agreements,
        science_per_turn,
    )) = query.get_mut(entity)
    {
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

        let Some(&cost) = tech_cost_manager.0.get(&current_tech) else {
            panic!(
                "Tech cost for {:?} not found in TechCostManager",
                current_tech
            );
        };

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

            // 只有当前科技不是`Technology::FutureTech`时，我们才从TechCostManager中删除该科技的成本信息
            // 因为`Technology::FutureTech`可以重复研究，此参数依旧有用，不应当在TechCostManager中删除该科技
            if current_tech != Technology::FutureTech {
                tech_cost_manager.0.remove(&current_tech);
            }

            // 如果科技是`Technology::FutureTech`，此处添加科技到已经研发过的科技列表中会添加失败，那么：
            // TODO:如此我们无需再发送科技研发成功消息？
            //      或许`Technology::FutureTech`研发过，我们照样需要发送消息，读取该消息来更新胜利分数
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
