//! 科技逻辑函数
//!
//! 包含科技成本计算、科技可研究性判断、研究进度计算等纯逻辑函数。

use civ_map_generator::ruleset::enums::{EnumStr, Technology};

use crate::{
    plugins::tech::TechCostManager,
    resources::{GameSettings, MapParametersRes},
};

use super::components::{OverflowScience, ResearchedTechList, TechProgressManager};

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
) -> i32 {
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

    tech_cost as i32
}

/// 获取科技的研究进度（已投入的科技点数）
pub fn research_progress(tech: Technology, tech_progress: &TechProgressManager) -> i32 {
    tech_progress.0.get(&tech).copied().unwrap_or(0)
}

/// 计算完成科技还需要的剩余科技点数
pub fn remaining_science_to_tech(
    tech: Technology,
    tech_progress: &TechProgressManager,
    researched_techs: &ResearchedTechList,
    tech_cost_manager: &TechCostManager,
    overflow_science: &OverflowScience,
    map_params: &MapParametersRes,
) -> i32 {
    let spare_science = if can_be_researched(tech, researched_techs, map_params) {
        overflow_science.0
    } else {
        0
    };

    let cost = tech_cost_manager.0.get(&tech).copied().unwrap_or(0);
    let researched = research_progress(tech, tech_progress);

    cost - researched - spare_science
}

/// 检查科技是否已研究
pub fn is_researched(tech: Technology, researched_techs: &ResearchedTechList) -> bool {
    researched_techs.0.contains(&tech)
}

/// 检查科技是否可以研究
pub fn can_be_researched(
    tech: Technology,
    researched_techs: &ResearchedTechList,
    map_params: &MapParametersRes,
) -> bool {
    let ruleset = &map_params.0.ruleset;
    let tech_info = &ruleset.technologies[tech];

    let is_continually_researchable = tech_info
        .uniques
        .contains(&"Can be continually researched".to_string());

    if is_researched(tech, researched_techs) && !is_continually_researchable {
        return false;
    }

    tech_info
        .prerequisites
        .iter()
        .all(|prereq| is_researched(Technology::from_str(prereq), researched_techs))
}
