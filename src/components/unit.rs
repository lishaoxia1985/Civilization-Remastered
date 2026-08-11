//! 单位相关组件
//!
//! 定义游戏中所有单位的组件类型。

use bevy::prelude::*;
use civ_map_generator::{
    ruleset::enums::{Nation, TileImprovement, Unit},
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

/// 单位类型标签 trait
pub trait UnitKind: Component + Default {
    /// 是否为军事单位
    const IS_MILITARY: bool;
}

impl UnitKind for Civilian {
    const IS_MILITARY: bool = false;
}

impl UnitKind for Military {
    const IS_MILITARY: bool = true;
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

/// 地块上单位列表标记 - 用于在相同地块切换选择不同单位
#[derive(Component)]
pub struct UnitOnTile;

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

// ============ 建造系统组件 ============

/// 地块设施组件 - 标记地块上已建造的设施
#[derive(Component, Clone, Copy, Debug)]
pub struct TileImprovementComponent(#[allow(dead_code)] pub TileImprovement);

/// 城市组件 - 标记地块上的城市
#[derive(Component, Clone, Debug)]
pub struct City {
    #[allow(dead_code)]
    pub name: String,
}

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
