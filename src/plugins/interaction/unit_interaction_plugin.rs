//! 单位交互插件
//!
//! 管理玩家与单位的交互操作，包括：
//! - 单位选择（点击地块选择单位，同地块多个单位循环切换）
//! - 移动范围显示与移动操作
//! - 单位操作菜单（移动、攻击、建城、跳过回合等）
//! - 单位信息面板更新

use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;
use civ_map_generator::{
    ruleset::enums::{EnumStr, Unit},
    tile::Tile,
};

use crate::{
    AppState, NationComponent, Player, ScreenState,
    components::*,
    resources::{TileEntityMap, TileMapRes},
};

/// 单位交互插件
pub struct UnitInteractionPlugin;

impl Plugin for UnitInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (setup_unit_info_panel, setup_unit_action_menu),
        )
        .add_systems(
            Update,
            (
                handle_unit_selection,
                handle_unit_action_click,
                handle_move_target_click,
                clear_move_range_on_deselect,
            )
                .chain()
                .run_if(in_state(ScreenState::WorldMap)),
        )
        .add_systems(
            Update,
            (
                handle_unit_attack_click,
                update_unit_info_panel,
                show_unit_action_menu,
            )
                .run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

// ============ UI 面板初始化 ============

/// 设置单位信息面板（左下角）
fn setup_unit_info_panel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(60.0),
            width: Val::Px(280.0),
            height: Val::Auto,
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        BorderColor::all(Color::WHITE),
        children![(
            Text::new("No unit selected\nClick a unit to select it"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..Default::default()
            },
            UnitInfoText,
            TextColor(Color::WHITE),
        )],
    ));
}

/// 设置单位操作菜单（左下角，信息面板下方）
fn setup_unit_action_menu(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            width: Val::Px(280.0),
            height: Val::Auto,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        Visibility::Hidden,
        UnitActionMenu,
    ));
}

// ============ 单位选择系统 ============

/// 处理单位选择 - 点击地块选择单位，同地块多个单位循环切换
fn handle_unit_selection(
    mut click_events: MessageReader<Pointer<Click>>,
    unit_query: Query<(Entity, &Owner, &UnitComponent, &ChildOf)>,
    world_tile_query: Query<&WorldTile>,
    mut commands: Commands,
    query_player: Single<&NationComponent, With<Player>>,
    selected_unit_query: Query<Entity, With<SelectedUnit>>,
    mut clear_move_range: Local<bool>,
) {
    let nation_component = query_player.into_inner();
    for click in click_events.read() {
        let Ok(clicked_tile) = world_tile_query.get(click.event_target()) else {
            continue;
        };

        let mut units_on_tile: Vec<(Entity, &UnitComponent)> = Vec::new();
        for (entity, owner, unit_component, child_of) in unit_query.iter() {
            let is_players_unit = match owner {
                Owner::Civilization(nation) => *nation == nation_component.0,
                Owner::CityState(_) => false,
            };
            if !is_players_unit {
                continue;
            }

            if let Ok(tile_component) = world_tile_query.get(child_of.0) {
                if tile_component.0 == clicked_tile.0 {
                    units_on_tile.push((entity, unit_component));
                }
            }
        }

        if units_on_tile.is_empty() {
            for selected in selected_unit_query.iter() {
                commands.entity(selected).remove::<SelectedUnit>();
            }
            *clear_move_range = true;
            continue;
        }

        let current_selected = selected_unit_query.iter().next();

        let entity_to_select = if let Some(current) = current_selected {
            let is_on_same_tile = units_on_tile.iter().any(|(e, _)| *e == current);
            if is_on_same_tile {
                let current_idx = units_on_tile
                    .iter()
                    .position(|(e, _)| *e == current)
                    .unwrap_or(0);
                let next_idx = (current_idx + 1) % units_on_tile.len();
                units_on_tile[next_idx].0
            } else {
                units_on_tile[0].0
            }
        } else {
            units_on_tile[0].0
        };

        for selected in selected_unit_query.iter() {
            commands.entity(selected).remove::<SelectedUnit>();
        }

        commands.entity(entity_to_select).insert(SelectedUnit);
        *clear_move_range = true;
    }
}

// ============ 移动范围计算与显示 ============

/// BFS 计算可移动范围
fn calculate_move_range(
    start_tile: Tile,
    movement_points: u32,
    tile_map: &civ_map_generator::tile_map::TileMap,
    grid: civ_map_generator::grid::HexGrid,
) -> HashSet<Tile> {
    let mut reachable = HashSet::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((start_tile, movement_points));
    visited.insert(start_tile);

    while let Some((current_tile, remaining)) = queue.pop_front() {
        if remaining == 0 {
            continue;
        }

        let neighbors: Vec<Tile> = current_tile.neighbor_tiles(grid).collect();
        for neighbor in &neighbors {
            if visited.contains(neighbor) {
                continue;
            }
            visited.insert(*neighbor);

            let cost = movement_cost(neighbor, tile_map);
            if cost > 0 && cost <= remaining {
                reachable.insert(*neighbor);
                queue.push_back((*neighbor, remaining - cost));
            }
        }
    }

    reachable
}

/// 计算进入一个地块的移动消耗
fn movement_cost(tile: &Tile, tile_map: &civ_map_generator::tile_map::TileMap) -> u32 {
    let terrain_type = tile.terrain_type(tile_map);

    match terrain_type {
        civ_map_generator::ruleset::enums::TerrainType::Flatland => 1,
        civ_map_generator::ruleset::enums::TerrainType::Hill => 2,
        civ_map_generator::ruleset::enums::TerrainType::Mountain => {
            return 0;
        }
        civ_map_generator::ruleset::enums::TerrainType::Water => {
            return 0;
        }
    }
}

/// 当取消选择时清除移动范围高亮
fn clear_move_range_on_deselect(
    mut commands: Commands,
    selected_query: Query<Entity, With<SelectedUnit>>,
    move_range_query: Query<Entity, With<MoveRangeHighlight>>,
    mut clear_flag: Local<bool>,
) {
    if *clear_flag || selected_query.iter().next().is_none() {
        for entity in move_range_query.iter() {
            commands.entity(entity).despawn();
        }
        *clear_flag = false;
    }
}

/// 显示可移动范围高亮
fn show_move_range(
    commands: &mut Commands,
    reachable_tiles: &HashSet<Tile>,
    tile_entity_map: &TileEntityMap,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let highlight_mesh = meshes.add(Circle::new(30.0));
    let highlight_material =
        materials.add(ColorMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 0.3)));

    for &tile in reachable_tiles {
        if let Some(entity) = tile_entity_map.get(tile) {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Mesh2d(highlight_mesh.clone()),
                    MeshMaterial2d(highlight_material.clone()),
                    Transform::from_xyz(0.0, 0.0, 10.0),
                    MoveRangeHighlight,
                    Pickable::default(),
                ));
            });
        }
    }
}

// ============ 移动系统 ============

/// 处理点击移动高亮区域 - 移动单位
fn handle_move_target_click(
    mut click_events: MessageReader<Pointer<Click>>,
    move_range_query: Query<&ChildOf, With<MoveRangeHighlight>>,
    world_tile_query: Query<&WorldTile>,
    selected_unit_query: Query<(Entity, &ChildOf, &Movement), With<SelectedUnit>>,
    mut commands: Commands,
) {
    let Ok((unit_entity, _unit_child_of, movement)) = selected_unit_query.single() else {
        return;
    };

    if movement.current == 0 {
        return;
    }

    for click in click_events.read() {
        if let Ok(highlight_child_of) = move_range_query.get(click.event_target()) {
            let target_tile_entity = highlight_child_of.0;

            if let Ok(_target_tile) = world_tile_query.get(target_tile_entity) {
                commands
                    .entity(unit_entity)
                    .set_parent_in_place(target_tile_entity);

                commands.entity(unit_entity).insert(Movement {
                    current: 0,
                    max: movement.max,
                });

                commands.entity(unit_entity).remove::<SelectedUnit>();
            }
        }
    }
}

// ============ 操作菜单系统 ============

/// 显示单位操作菜单
fn show_unit_action_menu(
    action_menu: Single<(Entity, &mut Visibility, Option<&mut Children>), With<UnitActionMenu>>,
    selected_query: Query<&UnitComponent, With<SelectedUnit>>,
    mut commands: Commands,
) {
    let (action_menu_entity, mut visibility, children_option) = action_menu.into_inner();

    if let Ok(unit_component) = selected_query.single() {
        *visibility = Visibility::Visible;

        // 清除旧的按钮（如果有）
        if let Some(children) = children_option {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        // 在 action menu 实体下创建按钮
        commands
            .entity(action_menu_entity)
            .with_children(|builder| {
                spawn_action_button(builder, "Move", ActionButton::Move);

                match unit_component {
                    UnitComponent::Military(_) => {
                        spawn_action_button(builder, "Attack", ActionButton::Attack);
                    }
                    UnitComponent::Civilian(unit) => {
                        if *unit == Unit::Settler {
                            spawn_action_button(builder, "Found City", ActionButton::FoundCity);
                        }
                    }
                }

                spawn_action_button(builder, "Skip Turn", ActionButton::SkipTurn);
                spawn_action_button(builder, "Deselect", ActionButton::Deselect);
            });
    } else {
        *visibility = Visibility::Hidden;
    }
}

/// 生成操作按钮
fn spawn_action_button(builder: &mut ChildSpawnerCommands, label: &str, action: ActionButton) {
    builder.spawn((
        Node {
            width: Val::Auto,
            height: Val::Auto,
            border: UiRect::all(Val::Px(1.0)),
            padding: UiRect::all(Val::Px(4.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.3, 0.3, 0.6)),
        BorderColor::all(Color::WHITE),
        Text::new(label.to_string()),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..Default::default()
        },
        TextColor(Color::WHITE),
        Button,
        action,
    ));
}

/// 处理操作按钮点击
fn handle_unit_action_click(
    action_button_query: Query<(&Interaction, &ActionButton)>,
    selected_unit_query: Query<
        (
            Entity,
            &UnitComponent,
            &Owner,
            &Health,
            &Strength,
            &Movement,
            &ChildOf,
        ),
        With<SelectedUnit>,
    >,
    world_tile_query: Query<&WorldTile>,
    tile_map: Option<Res<TileMapRes>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    tile_entity_map: Res<TileEntityMap>,
    move_range_query: Query<Entity, With<MoveRangeHighlight>>,
    attack_target_query: Query<Entity, With<AttackTargetHighlight>>,
) {
    for (interaction, action) in &action_button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            ActionButton::Move => {
                for entity in move_range_query.iter() {
                    commands.entity(entity).despawn();
                }

                if let Ok((_, _, _, _, _, movement, child_of)) = selected_unit_query.single() {
                    if movement.current == 0 {
                        continue;
                    }

                    if let Ok(tile) = world_tile_query.get(child_of.0) {
                        if let Some(tile_map) = &tile_map {
                            let grid = tile_map.0.world_grid.grid;
                            let reachable =
                                calculate_move_range(tile.0, movement.current, &tile_map.0, grid);

                            show_move_range(
                                &mut commands,
                                &reachable,
                                &tile_entity_map,
                                &mut meshes,
                                &mut color_materials,
                            );
                        }
                    }
                }
            }
            ActionButton::Attack => {
                // 清除移动范围高亮
                for entity in move_range_query.iter() {
                    commands.entity(entity).despawn();
                }
                // 显示可攻击范围（相邻敌方单位高亮）
                show_attack_targets(
                    &mut commands,
                    &selected_unit_query,
                    &world_tile_query,
                    &tile_map,
                    &tile_entity_map,
                    &mut meshes,
                    &mut color_materials,
                );
            }
            ActionButton::FoundCity => {
                if let Ok((entity, _, _, _, _, _, child_of)) = selected_unit_query.single() {
                    if let Ok(tile) = world_tile_query.get(child_of.0) {
                        info!("Founding city on tile: {:?}", tile.0);
                        commands.entity(entity).despawn();
                    }
                }
            }
            ActionButton::SkipTurn => {
                if let Ok((entity, _, _, _, _, movement, _)) = selected_unit_query.single() {
                    commands.entity(entity).insert(Movement {
                        current: 0,
                        max: movement.max,
                    });
                    commands.entity(entity).remove::<SelectedUnit>();
                }
            }
            ActionButton::Deselect => {
                for (entity, _, _, _, _, _, _) in selected_unit_query.iter() {
                    commands.entity(entity).remove::<SelectedUnit>();
                }
                for entity in move_range_query.iter() {
                    commands.entity(entity).despawn();
                }
                for entity in attack_target_query.iter() {
                    commands.entity(entity).despawn();
                }
            }
            ActionButton::CycleUnit => {}
        }
    }
}

// ============ 攻击目标显示系统 ============

/// 显示可攻击的敌方单位高亮
fn show_attack_targets(
    commands: &mut Commands,
    selected_unit_query: &Query<
        (
            Entity,
            &UnitComponent,
            &Owner,
            &Health,
            &Strength,
            &Movement,
            &ChildOf,
        ),
        With<SelectedUnit>,
    >,
    world_tile_query: &Query<&WorldTile>,
    tile_map: &Option<Res<TileMapRes>>,
    tile_entity_map: &Res<TileEntityMap>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;

    let Ok((_, _, _, _, _, _, child_of)) = selected_unit_query.single() else {
        return;
    };

    let Ok(attacker_tile) = world_tile_query.get(child_of.0) else {
        return;
    };

    let grid = tile_map.world_grid.grid;
    let neighbors: Vec<Tile> = attacker_tile.0.neighbor_tiles(grid).collect();

    let highlight_mesh = meshes.add(Circle::new(30.0));
    let highlight_material =
        materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 0.4)));

    for &tile in &neighbors {
        if let Some(entity) = tile_entity_map.get(tile) {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Mesh2d(highlight_mesh.clone()),
                    MeshMaterial2d(highlight_material.clone()),
                    Transform::from_xyz(0.0, 0.0, 10.0),
                    AttackTargetHighlight,
                    Pickable::default(),
                ));
            });
        }
    }
}

// ============ 攻击点击处理 ============

/// 处理攻击点击 - 点击敌方单位进行攻击
fn handle_unit_attack_click(
    mut click_events: MessageReader<Pointer<Click>>,
    unit_query: Query<(Entity, &Owner, &Strength, &ChildOf)>,
    selected_unit_query: Query<(Entity, &Owner, &ChildOf), With<SelectedUnit>>,
    world_tile_query: Query<&WorldTile>,
    tile_map: Option<Res<TileMapRes>>,
    move_range_query: Query<Entity, With<MoveRangeHighlight>>,
    attack_target_query: Query<Entity, With<AttackTargetHighlight>>,
    mut commands: Commands,
) {
    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;

    let Ok((attacker_entity, attacker_owner, attacker_child_of)) = selected_unit_query.single()
    else {
        return;
    };

    let attacker_nation = match attacker_owner {
        Owner::Civilization(nation) => *nation,
        Owner::CityState(nation) => *nation,
    };

    let Ok(attacker_tile) = world_tile_query.get(attacker_child_of.0) else {
        return;
    };

    for click in click_events.read() {
        for (target_entity, target_owner, _, target_child_of) in unit_query.iter() {
            let is_click_on_target = click.event_target() == target_entity;

            if !is_click_on_target {
                continue;
            }

            let is_same_owner = match target_owner {
                Owner::Civilization(nation) => *nation == attacker_nation,
                Owner::CityState(nation) => *nation == attacker_nation,
            };

            if is_same_owner {
                continue;
            }

            let Ok(target_tile) = world_tile_query.get(target_child_of.0) else {
                continue;
            };

            // 检查是否相邻
            if !are_tiles_adjacent(attacker_tile.0, target_tile.0, tile_map) {
                continue;
            }

            // 发送攻击请求（触发观察者）
            commands.trigger(crate::AttackRequestMessage {
                attacker: attacker_entity,
                target: target_entity,
            });

            // 清除高亮
            for entity in move_range_query.iter() {
                commands.entity(entity).despawn();
            }
            for entity in attack_target_query.iter() {
                commands.entity(entity).despawn();
            }
            commands.entity(attacker_entity).remove::<SelectedUnit>();

            break;
        }
    }
}

/// 检查两个地块是否相邻
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

    match grid.layout.orientation {
        civ_map_generator::grid::HexOrientation::Pointy => {
            dx <= 1 && dy <= 1 && (dx + dy) <= 2 && !(dx == 0 && dy == 0)
        }
        civ_map_generator::grid::HexOrientation::Flat => {
            dx <= 1 && dy <= 1 && !(dx == 0 && dy == 0)
        }
    }
}

// ============ 面板更新系统 ============

/// 更新单位信息面板
fn update_unit_info_panel(
    mut panel: Single<&mut Text, (With<UnitInfoText>, Without<UnitActionMenu>)>,
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

        let unit_type = match unit_component {
            UnitComponent::Civilian(_) => "Civilian",
            UnitComponent::Military(_) => "Military",
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
            "Selected: {} ({})\n\
             Type: {}\n\
             Owner: {}\n\
             {} \n\
             Health: {}/{}\n\
             Movement: {}/{}\n\
             \n\
             Click 'Move' to see movement range\n\
             Click on enemy unit to attack!",
            unit_name,
            unit_type,
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
