//! 游戏全局资源与状态定义
//!
//! 包含游戏配置、文明状态、科技管理等全局资源。

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::Task;
use civ_map_generator::{
    map_parameters::MapParameters,
    ruleset::enums::{Difficulty, Era, Speed},
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
    pub fn get(&self, tile: Tile) -> Entity {
        self.0[tile.index()]
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
