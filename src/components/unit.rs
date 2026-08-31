//! 单位相关组件
//!
//! 定义游戏中所有单位的组件类型。

use bevy::prelude::*;
use civ_map_generator::{
    ruleset::enums::{Building, Nation, TileImprovement, Unit},
    tile::Tile,
};

/// 单位所属者
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Owner(pub Nation);

/// 单位类型组件
#[derive(Component, Clone, Copy, Debug)]
pub struct UnitComponent(pub Unit);

/// 平民单位标签
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Civilian;

/// 军事单位标签
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Military;

/// 单位战斗力
#[derive(Component, Clone, Copy, Debug)]
pub struct Strength(pub u32);

#[derive(Component, Clone, Copy, Debug)]
pub struct RangedStrength(pub u32);

#[derive(Component, Clone, Copy, Debug)]
pub struct Range(pub u32);

/// 单位生命值
#[derive(Component, Clone, Copy, Debug)]
pub struct Health {
    /// 当前生命值
    pub current: u32,
    /// 最大生命值
    pub max: u32,
}

/// 单位移动力
#[derive(Component, Clone, Copy, Debug)]
pub struct Movement {
    /// 当前剩余移动力
    pub current: u32,
    /// 最大移动力
    pub max: u32,
}

/// 单位晋升
#[derive(Component, Clone, Debug)]
pub struct Promotion(Vec<String>);

/// 单位经验值
#[derive(Component, Clone, Copy, Debug)]
pub struct Experience {
    /// 当前经验值
    pub current: u32,
    /// 升级所需经验值
    pub max: u32,
}

// ============ 选择系统组件 ============

/// 已选中的单位标记
#[derive(Component)]
pub struct SelectedUnit;

/// 已选中的城市标记
#[derive(Component)]
pub struct SelectedCity;

/// 可移动范围高亮标记
#[derive(Component)]
pub struct MoveRangeHighlight;

/// 可攻击的敌方单位高亮标记
#[derive(Component)]
pub struct AttackTargetHighlight;

/// 单位信息面板容器
#[derive(Component)]
pub struct UnitInfoPanel;

/// 单位操作菜单面板
#[derive(Component)]
pub struct UnitActionMenu;

/// 单位操作按钮
#[derive(Component)]
pub enum ActionButton {
    /// 移动（打开移动范围）
    Move,
    /// 攻击
    Attack,
    /// 建立城市（移民）
    FoundCity,
    /// 建造农场（工人）
    BuildFarm,
    /// 建造矿山（工人）
    BuildMine,
    /// 跳过回合
    SkipTurn,
    /// 切换同一个地块上的另一个单位
    CycleUnit,
}

/// 移动按钮激活状态标记（点击 Move 后变为激活状态，此时点击地块可移动）
#[derive(Component)]
pub struct MoveButtonActive;

/// 市民分配按钮激活状态标记（点击 Assign 后进入分配屏幕并变绿，再点击或退出时取消）
#[derive(Component)]
pub struct CitizenAssignActive;

// ============ 建造系统组件 ============

/// 地块设施组件 - 标记地块上已建造的设施
#[derive(Component, Clone, Copy, Debug)]
pub struct TileImprovementComponent(#[allow(dead_code)] pub TileImprovement);

/// 地块设施建造进度组件 - 标记地块上正在建造的设施
#[derive(Component, Clone, Copy, Debug)]
pub struct TileImprovementBuildProgress {
    /// 正在建造的设施类型
    pub improvement: TileImprovement,
    /// 当前建造进度（已完成的回合数）
    pub progress: u32,
    /// 建造所需总回合数
    pub total_turns: u32,
    /// 建造所属文明
    pub owner: Nation,
    /// 建造工人实体
    pub worker: Entity,
}

/// 城市当前生产队列项目
#[derive(Component, Clone, Debug)]
pub enum CityProduction {
    /// 生产建筑
    Building(Building),
    /// 生产单位
    Unit(Unit),
}

/// 城市收益统计
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CityYields {
    /// 粮食
    pub food: u32,
    /// 产能
    pub production: u32,
    /// 科研
    pub science: u32,
    /// 金币
    pub gold: u32,
    /// 文化
    pub culture: u32,
    /// 信仰
    pub faith: u32,
    /// 快乐
    pub happiness: i32,
}

/// 城市组件 - 标记地块上的城市
#[derive(Component, Clone, Debug)]
pub struct City {
    /// 城市名称
    pub name: String,
    /// 城市人口（市民数）
    pub population: u32,
    /// 当前粮食储量
    pub food: u32,
    /// 人口增长所需粮食
    pub food_needed: u32,
    /// 城市已建造的建筑
    pub buildings: Vec<Building>,
    /// 当前正在生产的内容
    pub current_production: Option<CityProduction>,
    /// 生产进度
    pub production_progress: u32,
    /// 城市拥有的地块（含城市中心）
    pub owned_tiles: Vec<Tile>,
    /// 当前被市民工作的地块（不含城市中心）
    pub worked_tiles: Vec<Tile>,
    /// 文化点数（用于边界扩张）
    pub culture: u32,
    /// 边界扩张所需文化
    pub culture_to_expand: u32,
    /// 当前边界半径（扩张环数）
    pub border_radius: u32,
    /// 最大边界半径（城市领土可扩展到的最大范围）
    pub max_border_radius: u32,
    /// 市民可工作的最大半径（城市中心周围3格）
    pub work_radius: u32,
}

impl City {
    /// 创建一个新城市
    pub fn new(name: String) -> Self {
        Self {
            name,
            population: 1,
            food: 0,
            food_needed: 15,                   // 文明5：人口1->2需要15粮食
            buildings: vec![Building::Palace], // 初始自带宫殿
            current_production: None,
            production_progress: 0,
            owned_tiles: Vec::new(),
            worked_tiles: Vec::new(),
            culture: 0,
            culture_to_expand: 20, // 初始边界扩张需要20文化
            border_radius: 1,      // 初始拥有城市中心周围1格
            max_border_radius: 5,  // 城市领土最大可扩展到5格
            work_radius: 3,        // 市民只能工作在3格范围内（文明5规则）
        }
    }
}

// ============ 城市交互组件 ============

/// 城市边界高亮标记 - 标记地块上渲染的城市边界可视化边框
#[derive(Component)]
pub struct CityBorderHighlight;

/// 市民图标所关联的地块 - UI 节点形式指示其对应的地块，用于点击分配/取消分配
#[derive(Component)]
pub struct CitizenTile(pub Tile);

// ============ 世界地图组件 ============

/// 世界地图上的地块组件
/// Notes: 我们未在小地图上插入此组件
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldTile(pub Tile);

// ============ 小地图组件 ============

/// 信息面板
#[derive(Component)]
pub struct InfoPanel;

// ============ 相机组件 ============

/// 主相机标记
#[derive(Component)]
pub struct MainCamera;
