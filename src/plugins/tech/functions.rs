//! 科技逻辑函数
//!
//! 包含科技成本计算、科技可研究性判断、研究进度计算等纯逻辑函数。

use civ_map_generator::ruleset::enums::{EnumStr, Technology};

use crate::{
    plugins::tech::{TechState, TechStateManager},
    resources::{GameSettings, MapParametersRes},
};

/// 计算科技成本
///
/// TODO: 目前的科技成本计算仅考虑了玩家难度和游戏速度的修正，
///       未来可能需要考虑更多因素，例如
///       1. 文明特性
///       2. 已知文明研发了科技后
///       3. 建立了新的城市
pub fn cost_of_tech(
    tech: Technology,
    is_player: bool,
    game_settings: &GameSettings,
    map_params: &MapParametersRes,
) -> u32 {
    let ruleset = &map_params.0.ruleset;
    let tech_info = &ruleset.technologies[tech];
    let difficulty_info = &ruleset.difficulties[game_settings.difficulty];
    let speed_info = &ruleset.speeds[game_settings.speed];

    let mut tech_cost = tech_info.cost as f32;

    // 玩家难度修正
    if is_player {
        tech_cost *= difficulty_info.research_cost_modifier;
    }

    // 游戏速度修正
    tech_cost *= speed_info.science_cost_modifier;

    tech_cost as u32
}

/// 检查科技是否已研究
pub fn is_researched(tech: Technology, tech_state_manager: &TechStateManager) -> bool {
    matches!(
        tech_state_manager.0[tech],
        TechState::Researched | TechState::ResearchedAndRepeatable
    )
}

/// 检查科技是否可以研究
pub fn can_be_researched(
    tech: Technology,
    tech_state_manager: &TechStateManager,
    map_params: &MapParametersRes,
) -> bool {
    let ruleset = &map_params.0.ruleset;
    let tech_info = &ruleset.technologies[tech];

    let is_continually_researchable = tech_info
        .uniques
        .contains(&"Can be continually researched".to_string());

    if is_researched(tech, tech_state_manager) && !is_continually_researchable {
        return false;
    }

    tech_info
        .prerequisites
        .iter()
        .all(|prereq| is_researched(Technology::from_str(prereq), tech_state_manager))
}
