//! 科技相关消息定义

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Technology;

/// 当科技研发完成时发送的消息
#[derive(Message)]
pub struct TechResearchedMessage {
    /// 完成科技的文明实体
    pub nation: Entity,
    /// 完成的科技
    pub tech: Technology,
}
