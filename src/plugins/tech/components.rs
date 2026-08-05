//! 科技与时代组件定义
//!
//! 包含与科技研究、时代相关的所有数据组件。
//! 组件仅为数据结构，不含任何逻辑。

use std::collections::HashMap;

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Technology;
use enum_map::EnumMap;

/// 当前正在研发的科技
///
/// # Notes
///
/// TODO: 在Unciv项目中，使用科技队列来存储按先后顺序进行研发的科技，在队列中，第一个科技是正在研发的科技。
/// 在当前实现中，我们不在科技系统中使用科技队列，
/// 未来可能会在科技选择界面或AI选择科技的逻辑中实现科技队列功能。
#[derive(Component, Default)]
pub struct ResearchingTech(pub Option<Technology>);

/// 进行中的科技，只有已经研究且有科研值积累但尚未完成的科技存储在此。
///
/// 值为已投入的科技点数，不可能为 `0`。也就是说一个科技从未研究过，它不会出现在这个HashMap中。
#[derive(Component, Default)]
pub struct TechProgressManager(pub HashMap<Technology, i32>);

/// 研发科技所需的科技值
///
/// 值不可能为`0`. 如果一个科技已经研发过且不是像`Future Tech`这种可以重复研究的，它不会出现在这个HashMap中。
#[derive(Component, Default)]
pub struct TechCostManager(pub HashMap<Technology, i32>);

#[derive(Component, Default)]
pub struct TechStateManager(pub EnumMap<Technology, TechState>);

/// 科技可用性状态
#[derive(Default, PartialEq, Eq, Copy, Clone, Debug)]
pub enum TechState {
    /// 可研究（前置科技已完成）
    Available,
    /// 已研究完成
    ///
    /// Notes: 如果某科技可重复研究且已经研究完成，你应当使用[`Self::ResearchedAndRepeatable`]
    Researched,
    /// 已经研究完成且可重复研究
    ///
    /// 在original Civ中，仅`Technology::FutureTech`是可重复研究的
    ResearchedAndRepeatable,
    /// 不可用（前置科技未完成）
    #[default]
    Locked,
}

/// 溢出的科研值
///
/// 当研发某项科技时，如果投入的科研值超过了该科技的成本，超出的部分会被存储在此组件中，
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
///       当前暂时将其作为科技系统的必须组件
#[derive(Component, Default)]
pub struct ScienceOfLast8Turns(pub [i32; 8]);

/// 科技系统标记 - 标记实体的科技研究状态
///
/// 通过 `#[require]` 自动附带所有科技研究相关的数据组件。
#[derive(Component)]
#[require(
    ResearchingTech,
    TechProgressManager,
    TechStateManager,
    TechCostManager,
    OverflowScience,
    ScienceOfLast8Turns,
    ScienceFromResearchAgreements
)]
pub struct TechSystem;
