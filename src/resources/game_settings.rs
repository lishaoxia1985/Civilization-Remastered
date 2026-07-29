//! 游戏全局资源与状态定义
//!
//! 包含游戏配置、文明状态、科技管理等全局资源。

use std::{
    cmp::max,
    cmp::min,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bevy::prelude::*;
use bevy::tasks::Task;
use civ_map_generator::{
    map_parameters::MapParameters,
    ruleset::enums::{Difficulty, EnumStr, Era, Nation, Speed, Technology},
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

// ============ 文明状态 ============

/// 单个文明的数据
pub struct CivData {
    /// 文明所属国家
    pub nation: Nation,
    /// 当前金币
    pub gold: i32,
    /// 每回合金币收入
    pub gold_per_turn: i32,
    /// 每回合科技产出
    pub science_per_turn: i32,
    /// 当前文化值
    pub culture: i32,
    /// 每回合文化产出
    pub culture_per_turn: i32,
    /// 是否为人类玩家
    pub is_human: bool,
}

impl CivData {
    /// 创建新的文明数据
    pub fn new(nation: Nation, is_human: bool) -> Self {
        Self {
            nation,
            gold: if is_human { 500 } else { 500 },
            gold_per_turn: 0,
            science_per_turn: 3,
            culture: 0,
            culture_per_turn: 0,
            is_human,
        }
    }

    /// 结束回合
    pub fn end_turn(&mut self) {
        self.gold += self.gold_per_turn;
        self.culture += self.culture_per_turn;
    }

    /* /// 开始研究科技
    pub fn start_research(
        &self,
        tech: Technology,
        tech_registry: &mut TechManagerRegistry,
    ) -> bool {
        tech_registry.0.entry(self.nation).and_modify(|tm| {
            tm.techs_to_research.clear();
            tm.techs_to_research.push(tech)
        });
        true
    }

    /// AI 自动选择要研究的科技（选择最便宜的未研究科技）
    pub fn ai_choose_research(
        &mut self,
        map_params: &MapParametersRes,
    ) {
        let tech_manager = &tech_registry.0[&self.nation];

        if tech_manager.current_researching_technology().is_some() {
            return; // 已经在研究中
        }

        let mut available: Vec<(Technology, i32)> = map_params
            .0
            .ruleset
            .technologies
            .iter()
            .filter(|(tech, _)| tech_manager.can_be_researched(*tech, map_params))
            .map(|(tech, info)| (tech, info.cost))
            .collect();
        available.sort_by_key(|(_, cost)| *cost);
        if let Some(&(tech, _)) = available.first() {
            self.start_research(tech, tech_registry);
        }
    } */
}

// ============ 科技管理 ============

#[derive(Component)]
/// 科技管理器 - 管理单个文明的科技研究状态
pub struct TechManager {
    /// 当前时代
    pub era: Era,
    /// 已研究科技列表
    pub researched_technologies: Vec<Technology>,
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

    /// 最近8回合的科技值
    pub science_of_last_8_turns: [i32; 8],
    /// 研究协议提供的科技值
    pub science_from_research_agreements: i32,

    /// 已研究科技集合
    pub techs_researched: HashSet<Technology>,

    /// 待研究科技队列。
    /// 当前正在研究的科技始终位于队列的第一个。
    pub techs_to_research: Vec<Technology>,

    /// 溢出科技值
    pub overflow_science: i32,

    /// 进行中的科技，只有正在研究但尚未完成的科技存储在此。
    /// 值为已投入的科技点数，不能为 `0`。
    pub techs_in_progress: HashMap<Technology, i32>,

    /// 金币转科技比率
    pub gold_percent_converted_to_science: f32,
}

impl TechManager {
    /// 创建新的科技管理器
    pub fn new(era: Era) -> Self {
        Self {
            era,
            researched_technologies: Vec::new(),
            tech_uniques: HashMap::new(),
            units_can_embark: false,
            embarked_units_can_enter_ocean: false,
            all_units_can_enter_ocean: false,
            specific_units_can_enter_ocean: false,
            movement_speed_on_roads: 1.0,
            roads_connect_across_rivers: false,
            all_techs_are_researched: false,
            free_techs: 0,
            repeating_techs_researched: 0,
            science_of_last_8_turns: [0; 8],
            science_from_research_agreements: 0,
            techs_researched: HashSet::new(),
            techs_to_research: Vec::new(),
            overflow_science: 0,
            techs_in_progress: HashMap::new(),
            gold_percent_converted_to_science: 0.6,
        }
    }

    /// 获取已研究科技数量
    pub fn researched_count(&self) -> i32 {
        self.techs_researched.len() as i32
    }

    /// 获取溢出科技值
    pub fn overflow_science_value(&self) -> i32 {
        self.overflow_science
    }

    /// 计算科技成本
    pub fn cost_of_tech(
        &self,
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

        // TODO: 科技科技修正
        // let science_modifier = self.science_modifier(tech, civ_data, map_params);
        // tech_cost /= science_modifier;

        tech_cost as i32
    }

    /// 获取科技修正因子
    fn science_modifier(
        &self,
        _tech: Technology,
        _civ_data: &CivData,
        _map_params: &MapParametersRes,
    ) -> f32 {
        1.0
    }

    /// 获取当前正在研究的科技
    pub fn current_researching_technology(&self) -> Option<Technology> {
        self.techs_to_research.first().copied()
    }

    /// 获取科技的研究进度（已投入的科技点数）
    pub fn research_progress(&self, tech: Technology) -> i32 {
        self.techs_in_progress.get(&tech).copied().unwrap_or(0)
    }

    /// 计算完成科技还需要的剩余科技点数
    pub fn remaining_science_to_tech(
        &self,
        tech: Technology,
        is_player: bool,
        game_settings: &GameSettings,
        map_params: &MapParametersRes,
    ) -> i32 {
        let spare_science = if self.can_be_researched(tech, map_params) {
            self.overflow_science
        } else {
            0
        };

        let cost = self.cost_of_tech(tech, is_player, game_settings, map_params);
        let researched = self.research_progress(tech);

        cost - researched - spare_science
    }

    /// 计算完成科技还需要的回合数
    pub fn turns_to_tech(
        &self,
        tech: Technology,
        science_per_turn: i32,
        is_player: bool,
        game_settings: &GameSettings,
        map_params: &MapParametersRes,
    ) -> String {
        if self.is_researched(tech) && tech != Technology::FutureTech {
            return String::new();
        }

        let remaining_cost =
            self.remaining_science_to_tech(tech, is_player, game_settings, map_params) as f32;

        if remaining_cost <= 0.0 {
            return String::new();
        }

        if science_per_turn <= 0 {
            return "∞ turns".to_string();
        }

        let turns = (remaining_cost / science_per_turn as f32).ceil() as i32;
        format!("{} turns", turns.max(1))
    }

    /// 检查科技是否已研究
    pub fn is_researched(&self, tech: Technology) -> bool {
        self.techs_researched.contains(&tech)
    }

    /// 检查科技是否可以研究
    pub fn can_be_researched(&self, tech: Technology, map_params: &MapParametersRes) -> bool {
        let ruleset = &map_params.0.ruleset;
        let tech_info = &ruleset.technologies[tech];

        let is_continually_researchable = tech_info
            .uniques
            .contains(&"Can be continually researched".to_string());

        if self.is_researched(tech) && !is_continually_researchable {
            return false;
        }

        tech_info
            .prerequisites
            .iter()
            .all(|prereq| self.is_researched(Technology::from_str(prereq)))
    }

    /// 检查科技是否不可研究
    fn _is_unresearchable(&self, _tech: &Technology, _map_params: &MapParametersRes) -> bool {
        false
    }

    /// 检查所有科技是否已研究完毕
    pub fn all_techs_researched(&self) -> bool {
        self.all_techs_are_researched
    }

    /// 添加科技点
    pub fn add_science(
        &mut self,
        science: i32,
        current_tech: Technology,
        is_player: bool,
        science_per_turn: i32,
        game_settings: &GameSettings,
        map_params: &MapParametersRes,
    ) {
        let cost = self.cost_of_tech(current_tech, is_player, game_settings, map_params);
        let current = self.techs_in_progress.entry(current_tech).or_insert(0);
        *current += science;

        if *current >= cost {
            let extra_science = *current - cost;
            self.overflow_science += self.limit_overflow_science(
                extra_science,
                current_tech,
                science_per_turn,
                map_params,
            );
            self.add_technology(current_tech);
        }
    }

    /// 限制溢出科技点，防止过多结转到下一个科技
    fn limit_overflow_science(
        &self,
        overflow: i32,
        current_tech: Technology,
        science_per_turn: i32,
        map_params: &MapParametersRes,
    ) -> i32 {
        let ruleset = &map_params.0.ruleset;
        let tech_cost = ruleset.technologies[current_tech].cost;
        min(overflow, max(science_per_turn * 5, tech_cost))
    }

    /// 完成科技研究时添加科技
    pub fn add_technology(&mut self, tech: Technology) {
        let is_new = self.techs_researched.insert(tech);

        self.techs_to_research.retain(|t| t != &tech);
        self.techs_in_progress.remove(&tech);

        self.update_transient_booleans();

        if is_new {
            // TODO: 添加弹窗通知
        }
    }

    /// 更新瞬态布尔值
    fn update_transient_booleans(&mut self) {
        // TODO: 实现独特能力检查
    }

    /// 在回合结束时更新
    pub fn end_turn(
        &mut self,
        science_per_turn: i32,
        is_player: bool,
        turn: u32,
        game_settings: &GameSettings,
        map_params: &MapParametersRes,
    ) {
        self.science_of_last_8_turns[turn as usize % 8] = science_per_turn;

        let current_tech = match self.current_researching_technology() {
            Some(tech) => tech,
            None => panic!("No technology is being researched"),
        };

        let mut final_science = science_per_turn;

        if self.science_from_research_agreements != 0 {
            let boost = self.science_from_research_agreements / 3;
            final_science += boost;
            self.science_from_research_agreements = 0;
        }

        if self.overflow_science != 0 {
            final_science += self.overflow_science;
            self.overflow_science = 0;
        }

        self.add_science(
            final_science,
            current_tech,
            is_player,
            science_per_turn,
            game_settings,
            map_params,
        );
    }

    /// 设置瞬态数据
    pub fn set_transients(&mut self, _map_params: &MapParametersRes) {
        self.update_era();
        self.update_transient_booleans();
    }

    /// 更新时代
    fn update_era(&mut self) {
        if self.techs_researched.is_empty() {
            return;
        }
        // TODO: 实现时代更新逻辑
    }
}

impl Default for TechManager {
    fn default() -> Self {
        Self::new(Era::AncientEra)
    }
}
