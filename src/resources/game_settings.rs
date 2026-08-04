//! 游戏全局资源与状态定义
//!
//! 包含游戏配置、文明状态、科技管理等全局资源。

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bevy::prelude::*;
use bevy::tasks::Task;
use civ_map_generator::{
    map_parameters::MapParameters,
    ruleset::enums::{Difficulty, EnumStr, Era, Speed, Technology},
    tile::Tile,
    tile_map::TileMap,
};

// ============ 游戏配置 ============

/// 地图参数资源（包装 Arc 以实现线程安全共享）
#[derive(Resource)]
pub struct MapParametersRes(pub Arc<MapParameters>);

/// 生成的瓦片地图资源
#[derive(Resource)]
pub struct TileMapRes(pub TileMap);

#[derive(Resource)]
pub struct TileEntityMap(pub Vec<Entity>);

impl TileEntityMap {
    /// 创建指定容量的 TileEntityMap
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn push(&mut self, entity: Entity) {
        self.0.push(entity);
    }

    /// 获取指定瓦片对应的实体
    pub fn get(&self, tile: Tile) -> Option<Entity> {
        self.0.get(tile.index()).copied()
    }
}

/// 地图生成任务资源（用于异步生成地图）
#[derive(Resource)]
pub struct MapGeneratorTask(pub Task<TileMap>);

/// 游戏设置
#[derive(Resource)]
pub struct GameSettings {
    /// 游戏速度
    pub speed: Speed,
    /// 难度
    pub difficulty: Difficulty,
    /// 起始时代
    pub start_era: Era,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            speed: Speed::Standard,
            difficulty: Difficulty::Chieftain,
            start_era: Era::AncientEra,
        }
    }
}

// ============ 科技管理相关组件 ============

/// 当前正在研发的科技
///
/// # Notes
///
/// TODO: 在Unciv项目中，使用科技队列来存储按先后顺序进行研发的科技，在队列中，第一个科技是正在研发的科技。
/// 在当前实现中，我们不在TechManager中使用科技队列，
/// 未来可能会在科技选择界面或AI选择科技的逻辑中实现科技队列功能。
#[derive(Component, Default)]
pub struct ResearchingTech(pub Option<Technology>);

/// 已经研发的科技list
#[derive(Component, Default)]
pub struct ResearchedTechList(pub HashSet<Technology>);

/// 进行中的科技，只有已经研究且有科研值积累但尚未完成的科技存储在此。
/// 值为已投入的科技点数，不可能为 `0`。
#[derive(Component, Default)]
pub struct TechProgress(pub HashMap<Technology, i32>);

/// 溢出的科研值
///
/// 当研发某项科技时，如果投入的科研值超过了该科技的成本，超出的部分会被存储在此资源中，
/// 并在下一项科技的研发中继续使用。
#[derive(Component, Default)]
pub struct OverflowScience(pub i32);

/// 科研协议提供的科技值
///
/// TODO: 目前还没有实现科研协议的功能，因此这个组件暂时没有被使用。
///       科研协议和外交相关，科研协议提供的科技值在科研协议完成后一次性添加到完成那回合初的科技研发计算中。
#[derive(Component, Default)]
pub struct ScienceFromResearchAgreements(pub i32);

/// 存储最近8个回合的科技值
///
/// TODO: 用于计算消耗大科学家时获得的科技值, 但目前还没有实现消耗大科学家的功能。
///       此组件应当在管理大科学家、大工程师的组件中定义和插入，
///       当前暂时将其作为TechManager的必须组件
#[derive(Component, Default)]
pub struct ScienceOfLast8Turns(pub [i32; 8]);

/// 科技管理器 - 管理单个文明的科技研究状态
#[derive(Component)]
#[require(
    ResearchingTech,
    TechProgress,
    ResearchedTechList,
    OverflowScience,
    ScienceOfLast8Turns,
    ScienceFromResearchAgreements
)]
pub struct TechSystem;

/* {
    /// 科技独特能力
    pub tech_uniques: HashMap<String, Vec<String>>,

    // 单位移动相关标志
    /// 单位能否下海
    pub units_can_embark: bool,
    /// 下海单位能否进入海洋
    pub embarked_units_can_enter_ocean: bool,
    /// 所有单位能否进入海洋
    pub all_units_can_enter_ocean: bool,
    /// 特定单位能否进入海洋
    pub specific_units_can_enter_ocean: bool,

    // 移动速度相关
    /// 在道路上的移动速度
    pub movement_speed_on_roads: f32,
    /// 道路能否跨越河流
    pub roads_connect_across_rivers: bool,
    /// 所有科技是否已研究完毕
    pub all_techs_are_researched: bool,

    /// 免费科技数量
    pub free_techs: i32,
    /// 重复科技研究次数
    pub repeating_techs_researched: i32,

    /// 金币转科技比率
    pub gold_percent_converted_to_science: f32,
} */

/// 计算科技成本
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

    // TODO: 科技修正
    // let science_modifier = self.science_modifier(tech, civ_data, map_params);
    // tech_cost /= science_modifier;

    tech_cost as i32
}

/// 获取科技修正因子
/// TODO: 需要进一步完善
fn science_modifier() -> f32 {
    1.0
}

/// 获取科技的研究进度（已投入的科技点数）
pub fn research_progress(tech: Technology, tech_progress: &TechProgress) -> i32 {
    tech_progress.0.get(&tech).copied().unwrap_or(0)
}

/// 计算完成科技还需要的剩余科技点数
pub fn remaining_science_to_tech(
    tech: Technology,
    is_player: bool,
    tech_progress: &TechProgress,
    researched_techs: &ResearchedTechList,
    overflow_science: &OverflowScience,
    game_settings: &GameSettings,
    map_params: &MapParametersRes,
) -> i32 {
    let spare_science = if can_be_researched(tech, researched_techs, map_params) {
        overflow_science.0
    } else {
        0
    };

    let cost = cost_of_tech(tech, is_player, game_settings, map_params);
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

// ============ 时代相关组件 ============
#[derive(Component)]
pub struct EraComponent(pub Era);
