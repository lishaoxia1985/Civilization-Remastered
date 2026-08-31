//! 移动系统插件
//!
//! 管理单位移动逻辑，包括：
//! - 响应移动请求（MoveRequestMessage）
//! - 处理移动后自动攻击（目标地块有敌人时自动选择相邻位置攻击）
//! - 移动力消耗

use bevy::prelude::*;
use civ_map_generator::{ruleset::enums::TerrainType, tile::Tile, tile_map::TileMap};

use crate::{
    AttackRequestMessage, MoveRequestMessage,
    components::{Civilian, Military, Movement, Owner, UnitComponent},
    resources::{TileEntityMap, TileMapRes},
};

/// 移动插件
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_move_request);
    }
}

/// 处理移动请求
fn handle_move_request(
    event: On<MoveRequestMessage>,
    mut commands: Commands,
    military_unit_query: Query<
        (Entity, &ChildOf, &Movement, &Owner, &UnitComponent),
        With<Military>,
    >,
    civilian_unit_query: Query<(Entity, &ChildOf, &Owner, &UnitComponent), With<Civilian>>,
    unit_query: Query<(Entity, &ChildOf, &Movement, &Owner, &UnitComponent), With<UnitComponent>>,
    tile_map: Option<Res<TileMapRes>>,
    tile_entity_map: Res<TileEntityMap>,
) {
    let move_request = event.event();
    let unit_entity = move_request.unit;
    let target_tile = move_request.target_tile;

    let Ok((_, _, movement, unit_owner, _)) = unit_query.get(unit_entity) else {
        unreachable!(
            "The unit you are trying to move does not exist! You should never send a move request to a non-existent unit!"
        );
    };

    if movement.current == 0 {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;

    let target_tile_entity = tile_entity_map.get(target_tile);

    // 判断当前移动单位是否是平民单位
    let is_civilian = civilian_unit_query.contains(unit_entity);

    // 收集目标地块上的军事单位（排除自身）
    let military_on_tile: Vec<(Entity, &Owner)> = military_unit_query
        .iter()
        .filter(|(entity, child_of, _, _, _)| {
            *entity != unit_entity && child_of.0 == target_tile_entity
        })
        .map(|(entity, _, _, owner, _)| (entity, owner))
        .collect();

    // 收集目标地块上的平民单位（排除自身）
    let civilian_on_tile: Vec<(Entity, &Owner)> = civilian_unit_query
        .iter()
        .filter(|(entity, child_of, _, _)| {
            *entity != unit_entity && child_of.0 == target_tile_entity
        })
        .map(|(entity, _, owner, _)| (entity, owner))
        .collect();

    // 检查目标地块是否有敌方军事单位（只有军事单位才会阻挡移动并触发攻击）
    let has_enemy = military_on_tile
        .iter()
        .any(|(_, owner)| !is_same_owner(owner, unit_owner));

    // 计算移动到目标地块的实际移动消耗
    let move_cost = movement_cost(&target_tile, tile_map);
    let new_movement = if move_cost > 0 && move_cost <= movement.current {
        movement.current - move_cost
    } else {
        movement.current
    };

    if has_enemy {
        // 目标地块有敌人 - 尝试移动到相邻位置并攻击
        let grid = tile_map.world_grid.grid;
        let neighbors: Vec<Tile> = target_tile.neighbor_tiles(grid).collect();

        for neighbor in neighbors {
            let neighbor_entity = tile_entity_map.get(neighbor);
            let is_occupied = military_unit_query
                .iter()
                .any(|(_, child_of, _, _, _)| child_of.0 == neighbor_entity);

            if !is_occupied {
                // 移动到相邻位置
                commands.entity(neighbor_entity).add_child(unit_entity);
                // 扣除移动到相邻位置的移动消耗（攻击后移动力清零）
                commands.entity(unit_entity).insert(Movement {
                    current: 0,
                    max: movement.max,
                });

                // 找到目标地块上的敌人并触发攻击
                for (enemy_entity, enemy_child_of, _, enemy_owner, _) in unit_query.iter() {
                    if enemy_child_of.0 == target_tile_entity
                        && enemy_entity != unit_entity
                        && !is_same_owner(enemy_owner, unit_owner)
                    {
                        commands.trigger(AttackRequestMessage {
                            attacker: unit_entity,
                            target: enemy_entity,
                        });
                        break;
                    }
                }
                break;
            }
        }
    } else {
        // 目标地块没有敌人 - 先判断该地块是否允许当前单位进入（堆叠规则）
        // 规则：一个地块最多同时有一个军事单位和一个平民单位，且两者必须属于同一文明
        let can_enter = if is_civilian {
            // 平民单位：不能与另一个平民单位同格；可单独，或与该文明的一个军事单位同格
            let no_other_civilian = civilian_on_tile.is_empty();
            let compatible_military = military_on_tile.is_empty()
                || (military_on_tile.len() == 1
                    && military_on_tile
                        .iter()
                        .all(|(_, owner)| is_same_owner(owner, unit_owner)));
            no_other_civilian && compatible_military
        } else {
            // 军事单位：不能与另一个军事单位同格；可单独，或与该文明的一个平民单位同格
            let no_other_military = military_on_tile.is_empty();
            let compatible_civilian = civilian_on_tile.is_empty()
                || (civilian_on_tile.len() == 1
                    && civilian_on_tile
                        .iter()
                        .all(|(_, owner)| is_same_owner(owner, unit_owner)));
            no_other_military && compatible_civilian
        };

        if !can_enter {
            // 地块堆叠受限，阻止本次移动
            return;
        }

        // 正常移动，只扣除实际移动消耗
        commands.entity(target_tile_entity).add_child(unit_entity);
        commands.entity(unit_entity).insert(Movement {
            current: new_movement,
            max: movement.max,
        });
    }
}

/// 计算进入一个地块的移动消耗
fn movement_cost(tile: &Tile, tile_map: &TileMap) -> u32 {
    let terrain_type = tile.terrain_type(tile_map);

    match terrain_type {
        TerrainType::Flatland => 1,
        TerrainType::Hill => 2,
        TerrainType::Mountain => {
            return 0;
        }
        TerrainType::Water => {
            return 0;
        }
    }
}

/// 判断两个单位是否属于同一所有者
fn is_same_owner(owner1: &Owner, owner2: &Owner) -> bool {
    owner1 == owner2
}
