use bevy::prelude::*;
use civ_map_generator::ruleset::enums::EnumStr;
use civ_map_generator::tile::Tile;

use crate::{
    Civilizations, MapSetting, TileMapResource,
    unit_component::{Health, Movement, Owner, Strength, UnitComponent},
    world_map::WorldTile,
};

/// 选中的单位标记
#[derive(Component)]
pub struct SelectedUnit;

/// 可以攻击的敌方单位高亮
#[derive(Component)]
pub struct AttackTargetHighlight;

/// 单位选择与攻击信息面板
#[derive(Component)]
pub struct UnitInfoPanel;

/// 设置单位信息面板 - 必须包含 Text 组件
pub fn setup_unit_info_panel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            width: Val::Px(260.0),
            height: Val::Px(120.0),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        BorderColor::all(Color::WHITE),
        Text::new("No unit selected\nClick a unit to select it"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..Default::default()
        },
        TextColor(Color::WHITE),
        UnitInfoPanel,
    ));
}

/// 单位选择系统 - 点击地图上的单位进行选择
pub fn handle_unit_selection(
    mut click_events: MessageReader<Pointer<Click>>,
    unit_query: Query<(Entity, &UnitComponent, &Owner)>,
    mut commands: Commands,
    civs: Res<Civilizations>,
    selected_unit_query: Query<Entity, With<SelectedUnit>>,
) {
    for click in click_events.read() {
        for (entity, _unit_component, owner) in unit_query.iter() {
            if click.event_target() == entity {
                // 只允许选择玩家自己的单位
                let is_players_unit = match owner {
                    Owner::Civilization(nation) => *nation == civs.player_nation,
                    Owner::CityState(_) => false,
                };

                if is_players_unit {
                    // 清除之前的选择
                    for selected_entity in selected_unit_query.iter() {
                        commands.entity(selected_entity).remove::<SelectedUnit>();
                    }
                    // 标记当前选中单位
                    commands.entity(entity).insert(SelectedUnit);
                }
                break;
            }
        }
    }
}

/// 攻击系统 - 选中的单位点击敌方单位进行攻击
pub fn handle_unit_attack(
    mut click_events: MessageReader<Pointer<Click>>,
    unit_query: Query<(Entity, &Owner, &Strength, &WorldTile)>,
    selected_unit_query: Query<
        (Entity, &Owner, &Health, &Strength, &WorldTile),
        With<SelectedUnit>,
    >,
    mut commands: Commands,
    civs: Res<Civilizations>,
    tile_map: Option<Res<TileMapResource>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;

    // 如果没有选中的单位，直接返回
    let Ok((
        attacker_entity,
        attacker_owner_ref,
        attacker_health,
        attacker_strength,
        attacker_tile,
    )) = selected_unit_query.single()
    else {
        return;
    };

    // 复制需要的值以避免借用冲突
    let attacker_nation = match attacker_owner_ref {
        Owner::Civilization(nation) => *nation,
        Owner::CityState(nation) => *nation,
    };
    let attacker_health_current = attacker_health.current;
    let attacker_health_max = attacker_health.max;
    let attacker_strength_val = attacker_strength.0;
    let attacker_tile_val = attacker_tile.0;

    for click in click_events.read() {
        for (target_entity, target_owner, target_strength, target_tile) in unit_query.iter() {
            let is_click_on_target = click.event_target() == target_entity;

            if !is_click_on_target {
                continue;
            }

            // 不能攻击自己的单位
            let is_same_owner = match target_owner {
                Owner::Civilization(nation) => *nation == attacker_nation,
                Owner::CityState(nation) => *nation == attacker_nation,
            };

            if is_same_owner {
                continue;
            }

            // 检查目标是否在攻击范围内（相邻格）
            if !are_tiles_adjacent(attacker_tile_val, target_tile.0, tile_map) {
                continue;
            }

            // 执行攻击
            let attacker_score = attacker_strength_val as f32;
            let defender_score = target_strength.0 as f32;

            // 简单战斗公式
            let total = attacker_score + defender_score;
            let attacker_win_chance = if total > 0.0 {
                attacker_score / total
            } else {
                0.5
            };

            // 使用随机数判断胜负
            let mut rng = SimpleRng::new(civs.turn as u64);
            let roll = rng.f32();

            if roll < attacker_win_chance {
                // 攻击者获胜 - 摧毁目标单位
                commands.entity(target_entity).despawn();
                info!("Attacker destroyed target!");
            } else {
                // 防御者获胜 - 对攻击者造成伤害
                let damage = (defender_score * 0.3).max(1.0) as u32;
                let new_health = attacker_health_current.saturating_sub(damage);
                commands.entity(attacker_entity).insert(Health {
                    current: new_health,
                    max: attacker_health_max,
                });
                info!("Defender damaged attacker for {} damage!", damage);
            }

            // 清除选中状态（攻击后取消选中）
            commands.entity(attacker_entity).remove::<SelectedUnit>();
            break;
        }
    }
}

/// 检查两个格子是否相邻
fn are_tiles_adjacent(
    tile1: Tile,
    tile2: Tile,
    tile_map: &civ_map_generator::tile_map::TileMap,
) -> bool {
    let grid = tile_map.world_grid.grid;
    let offset1 = tile1.to_offset(grid);
    let offset2 = tile2.to_offset(grid);

    let dx = (offset1.0.x - offset2.0.x).abs();
    let dy = (offset1.0.y - offset2.0.y).abs();

    // 六边形网格中相邻格子的判断
    // 对于Pointy布局：相邻格子在水平和垂直方向上相差1
    match grid.layout.orientation {
        civ_map_generator::grid::HexOrientation::Pointy => {
            dx <= 1 && dy <= 1 && (dx + dy) <= 2 && !(dx == 0 && dy == 0)
        }
        civ_map_generator::grid::HexOrientation::Flat => {
            dx <= 1 && dy <= 1 && !(dx == 0 && dy == 0)
        }
    }
}

/// 更新单位信息面板
pub fn update_unit_info_panel(
    mut panel: Single<&mut Text, With<UnitInfoPanel>>,
    selected_unit_query: Query<
        (&UnitComponent, &Owner, &Health, &Strength, &Movement),
        With<SelectedUnit>,
    >,
) {
    if let Ok((unit_component, owner, health, strength, movement)) = selected_unit_query.single() {
        let unit_name = match unit_component {
            UnitComponent::Civilian(unit) => unit.as_str(),
            UnitComponent::Military(unit) => unit.as_str(),
        };

        let owner_name = match owner {
            Owner::Civilization(nation) | Owner::CityState(nation) => nation.as_str(),
        };

        let combat_str = if strength.0 > 0 {
            format!("Combat Strength: {}", strength.0)
        } else {
            "Non-combat unit".to_string()
        };

        panel.0 = format!(
            "Selected Unit\n\
             Type: {}\n\
             Owner: {}\n\
             {} \n\
             Health: {}/{}\n\
             Movement: {}/{}\n\
             \n\
             Click on enemy unit to attack!",
            unit_name,
            owner_name,
            combat_str,
            health.current,
            health.max,
            movement.current,
            movement.max,
        );
    } else {
        panel.0 = "No unit selected\nClick a unit to select it".to_string();
    }
}

/// 回合推进时 - 推进科技研究且恢复单位移动力
pub fn advance_turn_system(
    mut civs: ResMut<Civilizations>,
    mut unit_query: Query<&mut Movement>,
    map_setting: Res<MapSetting>,
) {
    // 推进玩家科技研究
    let player = civs.player_mut();
    if let Some(completed_tech) = player.advance_research(&map_setting) {
        info!("Player researched: {}", completed_tech);
    }

    // 恢复所有单位的移动力
    for mut movement in unit_query.iter_mut() {
        movement.current = movement.max;
    }
}

/// AI攻击系统 - 敌方文明自动攻击
pub fn ai_attack_system(
    civs: Res<Civilizations>,
    unit_query: Query<(Entity, &Owner, &Strength, &Health, &WorldTile)>,
    selected_unit_query: Query<
        (Entity, &Owner, &Health, &Strength, &WorldTile),
        With<SelectedUnit>,
    >,
    mut commands: Commands,
    tile_map: Option<Res<TileMapResource>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };

    let tile_map = &tile_map.0;
    // 找到所有敌方选中的单位
    for (attacker_entity, attacker_owner, attacker_health, attacker_strength, attacker_tile) in
        selected_unit_query.iter()
    {
        // 只处理敌方单位
        let is_enemy = match attacker_owner {
            Owner::Civilization(nation) => civs.is_enemy(*nation),
            Owner::CityState(_) => false,
        };
        if !is_enemy {
            continue;
        }

        // 寻找相邻的玩家单位
        for (target_entity, target_owner, target_strength, _target_health, target_tile) in
            unit_query.iter()
        {
            let is_target_player = match target_owner {
                Owner::Civilization(nation) => *nation == civs.player_nation,
                Owner::CityState(_) => false,
            };
            if !is_target_player {
                continue;
            }

            // 检查是否相邻
            if !are_tiles_adjacent(attacker_tile.0, target_tile.0, tile_map) {
                continue;
            }

            // 执行攻击
            let attacker_score = attacker_strength.0 as f32;
            let defender_score = target_strength.0 as f32;
            let total = attacker_score + defender_score;
            let attacker_win_chance = if total > 0.0 {
                attacker_score / total
            } else {
                0.5
            };

            let mut rng = SimpleRng::new(civs.turn as u64);
            let roll = rng.f32();

            if roll < attacker_win_chance {
                commands.entity(target_entity).despawn();
                info!("Enemy destroyed player unit!");
            } else {
                let damage = (defender_score * 0.3).max(1.0) as u32;
                let new_health = attacker_health.current.saturating_sub(damage);
                commands.entity(attacker_entity).insert(Health {
                    current: new_health,
                    max: attacker_health.max,
                });
                info!("Player damaged enemy for {} damage!", damage);
            }

            commands.entity(attacker_entity).remove::<SelectedUnit>();
            break;
        }
    }
}

/// 简易随机数生成器
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as f32 / (1u64 << 31) as f32
    }
}
