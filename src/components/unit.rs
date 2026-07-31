//! 单位相关组件
//!
//! 定义游戏中所有单位的组件类型。

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Technology;
use civ_map_generator::ruleset::enums::{Nation, Unit};
use civ_map_generator::tile::Tile;

/// 单位所属者
#[derive(Component, Clone, Copy, Debug)]
pub enum Owner {
    /// 属于文明
    Civilization(Nation),
    /// 属于城邦
    CityState(Nation),
}

/// 单位类型组件
#[derive(Component, Clone, Copy, Debug)]
pub enum UnitComponent {
    /// 平民单位
    Civilian(Unit),
    /// 军事单位
    Military(Unit),
}

/// 单位战斗力
#[derive(Component, Clone, Copy, Debug)]
pub struct Strength(pub u32);

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

// ============ 选择系统组件 ============

/// 已选中的单位标记
#[derive(Component)]
pub struct SelectedUnit;

/// 地块上单位列表标记 - 用于在相同地块切换选择不同单位
#[derive(Component)]
pub struct UnitOnTile;

/// 可移动范围高亮标记
#[derive(Component)]
pub struct MoveRangeHighlight;

/// 可攻击的敌方单位高亮标记
#[derive(Component)]
pub struct AttackTargetHighlight;

/// 单位信息和攻击面板
#[derive(Component)]
pub struct UnitInfoText;

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
    /// 跳过回合
    SkipTurn,
    /// 取消选择
    Deselect,
    /// 切换同一个地块上的另一个单位
    CycleUnit,
}

// ============ 世界地图组件 ============

/// 世界地图上的地块组件
/// Notes: 我们未在小地图上插入此组件
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldTile(pub Tile);

// ============ 科技树组件 ============

/// 科技按钮组件
#[derive(Component, Clone, Copy, Debug)]
pub struct TechButton(pub Technology);

/// 科技可用性状态
#[derive(Component, Clone, Debug, PartialEq)]
pub enum TechButtonState {
    /// 可研究（前置科技已完成）
    Available,
    /// 正在研究中
    InProgress,
    /// 已研究完成
    Researched,
    /// 不可用（前置科技未完成）
    Locked,
}

/// 科技树可滚动节点
#[derive(Component)]
pub struct TechTreeScrollableNode;

/// 关闭科技树按钮
#[derive(Component)]
pub struct CloseTechTreeButton;

// ============ 小地图组件 ============

/// 信息面板
#[derive(Component)]
pub struct InfoPanel;

// ============ 相机组件 ============

/// 主相机标记
#[derive(Component)]
pub struct MainCamera;
