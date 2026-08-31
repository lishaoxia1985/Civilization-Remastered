//! 单位交互插件
//!
//! 管理玩家与单位的交互操作，包括：
//! - 移动范围显示与高亮（含敌人红色圆圈标记）
//! - 点击移动目标后发出移动请求（由 MovementPlugin 处理）
//! - 单位操作菜单（移动、攻击、建城、建造设施、跳过回合等）
//! - 单位信息面板更新
//!
//! 注意：点击地块选择单位/城市由 [`TileSelectionPlugin`] 统一处理。

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::{prelude::*, ui_render::ui_material::MaterialNode};
use civ_map_generator::{
    grid::{HexGrid, HexOrientation},
    ruleset::enums::{EnumStr, TerrainType, TileImprovement, Unit},
    tile::Tile,
    tile_map::TileMap,
};

use crate::{
    AppState, AttackRequestMessage, BuildRequestMessage, FoundCityRequestMessage,
    MoveRequestMessage, NationComponent, Player, ScreenState, TurnManager,
    assets::{ColorReplaceMaterial, GameAssets, RingProgressMaterial},
    components::*,
    resources::{TileEntityMap, TileMapRes},
};

/// 单位图标（叠加在环形进度条之上）
#[derive(Component)]
struct UnitInfoIcon;

/// 单位信息面板字段类型
#[derive(Component)]
pub enum UnitInfoField {
    /// 单位名称
    Name,
    /// 攻击力
    Strength,
    /// 生命值
    Health,
    /// 移动力
    Movement,
}

/// 环形进度条标记（由 MaterialNode 渲染）
#[derive(Component)]
struct UnitRingProgress;

/// 单位交互插件
pub struct UnitInteractionPlugin;

impl Plugin for UnitInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(show_movement_range) // 监听 OnAdd<Selected> 事件
            .add_observer(hide_movement_range) // 监听 OnRemove<Selected> 事件
            .add_systems(
                OnEnter(AppState::GameStart),
                (setup_unit_info_panel, setup_unit_action_menu),
            )
            .add_systems(
                Update,
                (handle_unit_action_click, handle_move_target_click)
                    .chain()
                    .run_if(in_state(ScreenState::WorldMap)),
            )
            .add_systems(
                Update,
                (
                    handle_unit_attack_click,
                    update_unit_info_panel,
                    show_unit_action_menu,
                    animate_selected_unit,
                )
                    .run_if(in_state(ScreenState::WorldMap)),
            );
    }
}

fn show_movement_range(
    trigger: On<Add, SelectedUnit>, // 触发事件的源实体
    mut commands: Commands,
    world_tile_query: Query<&WorldTile>,
    tile_entity_map: Res<TileEntityMap>,
    unit_query: Query<(&Owner, &ChildOf, &Movement), With<UnitComponent>>,
    tile_map: Option<Res<TileMapRes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok((selected_owner, child_of, movement)) = unit_query.get(trigger.entity) else {
        unreachable!("Invalid unit selection")
    };

    // 移动力耗尽时无需显示移动范围
    if movement.current == 0 {
        return;
    }

    let mut reachable_tiles = HashSet::new();
    if let Ok(tile) = world_tile_query.get(child_of.0) {
        if let Some(tile_map) = &tile_map {
            let grid = tile_map.0.world_grid.grid;
            reachable_tiles = calculate_move_range(tile.0, movement.current, &tile_map.0, grid);
        }
    }

    let highlight_mesh = meshes.add(Circle::new(30.0));
    let move_material =
        color_materials.add(ColorMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 0.3)));
    let enemy_material =
        color_materials.add(ColorMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 0.4)));

    for tile in reachable_tiles {
        let entity = tile_entity_map.get(tile);

        // 检查该地块是否有敌方单位
        // 如果有敌方单位，则显示红色,否则显示绿色
        let has_enemy = unit_query.iter().any(|(owner, child_of, ..)| {
            child_of.0 == entity && !is_same_owner(owner, selected_owner)
        });

        let material = if has_enemy {
            enemy_material.clone()
        } else {
            move_material.clone()
        };

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh2d(highlight_mesh.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(0.0, 0.0, 10.0),
                MoveRangeHighlight,
                Pickable::IGNORE,
            ));
        });
    }
}

fn hide_movement_range(
    _trigger: On<Remove, SelectedUnit>,
    mut commands: Commands,
    query: Query<Entity, With<MoveRangeHighlight>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ============ UI 面板初始化 ============

/// 设置单位信息面板（左下角）
/// 类似文明5布局：左侧大图标（带HP/XP半圆环），右侧信息区
/// 布局顺序：移动力（第一行）、名称、类型、分隔线、力量（仅军事单位）、HP
fn setup_unit_info_panel(
    mut commands: Commands,
    materials: Res<GameAssets>,
    mut ring_materials: ResMut<Assets<RingProgressMaterial>>,
) {
    // 创建环形进度条材质（初始全满 HP、0 XP）
    let ring_material_handle = ring_materials.add(RingProgressMaterial {
        left_progress: 1.0,
        right_progress: 0.0,
        left_color: LinearRgba::new(0.0, 1.0, 0.0, 1.0),
        right_color: LinearRgba::new(0.0, 0.5, 1.0, 1.0),
        background_color: LinearRgba::new(0.2, 0.2, 0.2, 0.8),
        ring_thickness: 0.15,
    });

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(60.0),
            width: Val::Px(280.0),
            height: Val::Px(110.0),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        BorderColor::all(Color::WHITE),
        Visibility::Hidden,
        UnitInfoPanel,
        children![
            // 左侧：单位大图标（64x64），带HP/XP半圆环
            (
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    position_type: PositionType::Relative,
                    ..Default::default()
                },
                children![
                    // 底层：环形进度条（UiMaterial 渲染 HP 左半环 + XP 右半环）
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..Default::default()
                        },
                        MaterialNode(ring_material_handle),
                        UnitRingProgress,
                    ),
                    // 顶层：单位图标
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(8.0),
                            top: Val::Px(8.0),
                            width: Val::Px(48.0),
                            height: Val::Px(48.0),
                            ..Default::default()
                        },
                        ImageNode::new(materials.texture_handle("Warrior")),
                        UnitInfoIcon,
                    ),
                ],
            ),
            // 右侧：信息区
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..Default::default()
                },
                children![
                    // 单位名称
                    (
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        UnitInfoField::Name,
                    ),
                    // 分隔线
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(3.0)),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                    ),
                    // 移动力
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(5.0),
                            ..Default::default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    ..Default::default()
                                },
                                ImageNode::new(materials.texture_handle("icon_moves")),
                            ),
                            (
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..Default::default()
                                },
                                TextColor(Color::srgba(0.0, 0.5, 1.0, 1.0)),
                                UnitInfoField::Movement,
                            ),
                        ],
                    ),
                    // 战斗力行（剑图标 + 数值）- 平民单位隐藏
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(5.0),
                            ..Default::default()
                        },
                        BackgroundColor(Color::NONE),
                        children![
                            (
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    ..Default::default()
                                },
                                ImageNode::new(materials.texture_handle("icon_strength")),
                            ),
                            (
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..Default::default()
                                },
                                TextColor(Color::srgba(1.0, 0.84, 0.0, 1.0)),
                                UnitInfoField::Strength,
                            ),
                        ],
                    ),
                    // HP行（HP文字 + 数值）
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(5.0),
                            ..Default::default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    ..Default::default()
                                },
                                children![(
                                    Text::new("HP"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..Default::default()
                                    },
                                    TextColor(Color::srgba(0.0, 1.0, 0.0, 1.0)),
                                ),]
                            ),
                            (
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..Default::default()
                                },
                                TextColor(Color::srgba(0.0, 1.0, 0.0, 1.0)),
                                UnitInfoField::Health,
                            ),
                        ],
                    ),
                ],
            ),
        ],
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

// ============ 移动范围计算与显示 ============

/// BFS 计算可移动范围
fn calculate_move_range(
    start_tile: Tile,
    movement_points: u32,
    tile_map: &TileMap,
    grid: HexGrid,
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

/// 判断两个所有者是否相同
fn is_same_owner(owner1: &Owner, owner2: &Owner) -> bool {
    owner1 == owner2
}

// ============ 移动系统 ============

/// 处理点击移动目标地块
/// Move 按钮激活时，点击任意地块立即退出激活状态，然后根据点击位置处理：
/// - 点击另一个单位 → 切换选中该单位（Move 恢复未激活）
/// - 点击合法移动目标 → 执行移动
/// - 点击无效地块 → 仅取消移动激活状态
fn handle_move_target_click(
    mut click_events: MessageReader<Pointer<Click>>,
    move_range_query: Query<(Entity, &ChildOf), With<MoveRangeHighlight>>,
    world_tile_query: Query<&WorldTile>,
    selected_unit_query: Query<
        (Entity, &ChildOf, &UnitComponent, &Owner, &Movement),
        With<SelectedUnit>,
    >,
    unit_query: Query<(Entity, &Owner, &UnitComponent, &ChildOf)>,
    military_unit_query: Query<(Entity, &Owner, &ChildOf), With<Military>>,
    tile_map: Option<Res<TileMapRes>>,
    move_button_query: Query<Entity, With<MoveButtonActive>>,
    mut commands: Commands,
    player_query: Query<&NationComponent, With<Player>>,
    turn_manager: Res<TurnManager>,
) {
    // 只有 Move 按钮激活时才允许点击地块移动
    if move_button_query.iter().next().is_none() {
        return;
    }

    // 获取当前回合的nation实体，只在玩家回合处理选择
    let current_entity = turn_manager.current_nation_entity();

    let Ok(nation_component) = player_query.get(current_entity) else {
        return;
    };

    let Ok((unit_entity, unit_child_of, _, unit_owner, movement)) = selected_unit_query.single()
    else {
        return;
    };

    if movement.current == 0 {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;

    // 计算可移动范围
    let Ok(unit_tile) = world_tile_query.get(unit_child_of.0) else {
        return;
    };
    let grid = tile_map.world_grid.grid;
    let reachable = calculate_move_range(unit_tile.0, movement.current, tile_map, grid);

    for click in click_events.read() {
        // 获取点击的目标地块实体
        let target_tile_entity =
            if let Ok((_, highlight_child_of)) = move_range_query.get(click.event_target()) {
                // 点击的是高亮区域，取其父地块
                highlight_child_of.0
            } else {
                // 点击的是地块本身
                click.event_target()
            };

        // 如果点击的不是地块（例如 UI 按钮），跳过，不干扰按钮切换逻辑
        let Ok(target_tile) = world_tile_query.get(target_tile_entity) else {
            continue;
        };

        // 点击的是地块，立即退出 Move 激活状态
        for entity in move_button_query.iter() {
            commands.entity(entity).remove::<MoveButtonActive>();
            commands
                .entity(entity)
                .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
        }

        // 情况1：点击的是另一个单位 → 切换选中该单位
        if let Ok((clicked_unit_entity, clicked_owner, _, _)) = unit_query.get(click.event_target())
        {
            if clicked_unit_entity != unit_entity {
                let is_players_unit = clicked_owner.0 == nation_component.0;

                if is_players_unit {
                    // 切换选中到点击的单位
                    commands.entity(unit_entity).remove::<SelectedUnit>();
                    commands.entity(clicked_unit_entity).insert(SelectedUnit);
                    continue;
                }
            }
        }

        // 情况2：点击合法移动目标 → 执行移动
        if reachable.contains(&target_tile.0) {
            // 查找目标地块上的敌方军事单位
            let enemy_on_target = military_unit_query
                .iter()
                .find(|(entity, owner, child_of)| {
                    *entity != unit_entity
                        && child_of.0 == target_tile_entity
                        && !is_same_owner(owner, unit_owner)
                });

            let is_attacker_military = military_unit_query.get(unit_entity).is_ok();

            // 检查攻击者与目标地块是否相邻
            let is_adjacent = are_tiles_adjacent(unit_tile.0, target_tile.0, tile_map);

            // 军事单位：Move 键即攻击键，点击相邻且有敌人的地块直接攻击
            if is_attacker_military && is_adjacent {
                if let Some((enemy_entity, _, _)) = enemy_on_target {
                    commands.trigger(AttackRequestMessage {
                        attacker: unit_entity,
                        target: enemy_entity,
                    });
                    commands.entity(unit_entity).remove::<SelectedUnit>();
                    continue;
                }
            }

            // 发出移动请求，由 MovementPlugin 处理移动
            commands.trigger(MoveRequestMessage {
                unit: unit_entity,
                target_tile: target_tile.0,
            });
        }

        // 情况3：无效地块 → 仅取消移动激活状态（已在上方清除）
    }
}

// ============ 操作菜单系统 ============

/// 显示单位操作菜单
/// 只在选中单位变化时重建按钮，避免每帧重建导致 Move 按钮激活状态丢失
fn show_unit_action_menu(
    action_menu: Single<(Entity, &mut Visibility, Option<&mut Children>), With<UnitActionMenu>>,
    selected_query: Query<(Entity, &UnitComponent), With<SelectedUnit>>,
    military_query: Query<(), With<Military>>,
    mut commands: Commands,
    mut last_selected: Local<Option<Entity>>,
) {
    let (action_menu_entity, mut visibility, children_option) = action_menu.into_inner();

    if let Ok((selected_entity, unit_component)) = selected_query.single() {
        *visibility = Visibility::Visible;

        // 如果选中的单位没有变化，不重建按钮（保留 Move 按钮激活状态）
        if *last_selected == Some(selected_entity) {
            return;
        }
        *last_selected = Some(selected_entity);

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

                match unit_component.0 {
                    // 军事单位：Move 键即攻击键，不需要单独的 Attack 按钮
                    _ if military_query.get(selected_entity).is_ok() => {}
                    Unit::Settler => {
                        spawn_action_button(builder, "Found City", ActionButton::FoundCity);
                    }
                    Unit::Worker => {
                        spawn_action_button(builder, "Build Farm", ActionButton::BuildFarm);
                        spawn_action_button(builder, "Build Mine", ActionButton::BuildMine);
                    }
                    _ => {}
                }

                spawn_action_button(builder, "Skip Turn", ActionButton::SkipTurn);
            });
    } else {
        *visibility = Visibility::Hidden;
        *last_selected = None;
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
    action_button_query: Query<(&Interaction, &ActionButton, Entity), Changed<Interaction>>,
    selected_unit_query: Query<(Entity, &Movement, &ChildOf), With<SelectedUnit>>,
    world_tile_query: Query<&WorldTile>,
    tile_map: Option<Res<TileMapRes>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    tile_entity_map: Res<TileEntityMap>,
    move_range_query: Query<Entity, With<MoveRangeHighlight>>,
    move_button_query: Query<Entity, With<MoveButtonActive>>,
) {
    let Ok((entity, movement, child_of)) = selected_unit_query.single() else {
        return;
    };

    for (interaction, action, button_entity) in &action_button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            ActionButton::Move => {
                // 检查当前按钮是否已激活
                let is_active = move_button_query.iter().any(|e| e == button_entity);

                if is_active {
                    // 已激活 → 取消激活（恢复未激活状态）
                    commands.entity(button_entity).remove::<MoveButtonActive>();
                    commands
                        .entity(button_entity)
                        .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
                } else {
                    // 未激活 → 激活
                    // 清除其他按钮的激活状态
                    for entity in move_button_query.iter() {
                        commands.entity(entity).remove::<MoveButtonActive>();
                        commands
                            .entity(entity)
                            .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
                    }

                    if movement.current == 0 {
                        continue;
                    }

                    // 激活 Move 按钮（变色）
                    commands.entity(button_entity).insert(MoveButtonActive);
                    commands
                        .entity(button_entity)
                        .insert(BackgroundColor(Color::srgb(0.1, 0.8, 0.1)));
                }
            }
            ActionButton::Attack => {
                // 清除 Move 按钮激活状态
                for entity in move_button_query.iter() {
                    commands.entity(entity).remove::<MoveButtonActive>();
                    commands
                        .entity(entity)
                        .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
                }
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
                if let Ok((entity, _, child_of)) = selected_unit_query.single() {
                    if let Ok(tile) = world_tile_query.get(child_of.0) {
                        // 发出建城请求，由 ConstructionPlugin 处理
                        // 如果建城成功上述插件会自动销毁该单位实体
                        commands.trigger(FoundCityRequestMessage {
                            unit: entity,
                            target_tile: tile.0,
                        });
                    }
                }
            }
            ActionButton::BuildFarm => {
                if let Ok(tile) = world_tile_query.get(child_of.0) {
                    // 发出建造请求，由 ConstructionPlugin 处理
                    commands.trigger(BuildRequestMessage {
                        unit: entity,
                        target_tile: tile.0,
                        improvement: TileImprovement::Farm,
                    });
                    commands.entity(entity).remove::<SelectedUnit>();
                }
            }
            ActionButton::BuildMine => {
                if let Ok(tile) = world_tile_query.get(child_of.0) {
                    // 发出建造请求，由 ConstructionPlugin 处理
                    commands.trigger(BuildRequestMessage {
                        unit: entity,
                        target_tile: tile.0,
                        improvement: TileImprovement::Mine,
                    });
                    commands.entity(entity).remove::<SelectedUnit>();
                }
            }
            ActionButton::SkipTurn => {
                // 清除 Move 按钮激活状态
                // TODO: 跳过当前单位，应当是在结束回合的按钮上点击下一个单位时不会再选中该单位
                //       即跳过该单位，而不是清零该单位的移动力
                for entity in move_button_query.iter() {
                    commands.entity(entity).remove::<MoveButtonActive>();
                    commands
                        .entity(entity)
                        .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.6)));
                }

                commands.entity(entity).insert(Movement {
                    current: movement.max,
                    max: movement.max,
                });
                commands.entity(entity).remove::<SelectedUnit>();
            }
            ActionButton::CycleUnit => {}
        }
    }
}

// ============ 攻击目标显示系统 ============

/// 显示可攻击的敌方单位高亮
fn show_attack_targets(
    commands: &mut Commands,
    selected_unit_query: &Query<(Entity, &Movement, &ChildOf), With<SelectedUnit>>,
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

    let Ok((_, _, child_of)) = selected_unit_query.single() else {
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
        let entity = tile_entity_map.get(tile);

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

// ============ 攻击点击处理 ============

/// 处理攻击点击 - 点击敌方单位进行攻击
fn handle_unit_attack_click(
    mut click_events: MessageReader<Pointer<Click>>,
    unit_query: Query<(Entity, &Owner, &Strength, &ChildOf)>,
    selected_unit_query: Query<(Entity, &Owner, &ChildOf), With<SelectedUnit>>,
    world_tile_query: Query<&WorldTile>,
    tile_map: Option<Res<TileMapRes>>,
    attack_target_query: Query<Entity, With<AttackTargetHighlight>>,
    move_button_query: Query<Entity, With<MoveButtonActive>>,
    mut commands: Commands,
) {
    // 如果 Move 按钮激活，点击地块是移动操作，不处理攻击
    if move_button_query.iter().next().is_some() {
        return;
    }

    let Some(tile_map) = tile_map else {
        return;
    };
    let tile_map = &tile_map.0;

    let Ok((attacker_entity, attacker_owner, attacker_child_of)) = selected_unit_query.single()
    else {
        return;
    };

    let attacker_nation = attacker_owner.0;

    let Ok(attacker_tile) = world_tile_query.get(attacker_child_of.0) else {
        return;
    };

    for click in click_events.read() {
        for (target_entity, target_owner, _, target_child_of) in unit_query.iter() {
            let is_click_on_target = click.event_target() == target_entity;

            if !is_click_on_target {
                continue;
            }

            // TODO：此处不太可能属于同一个拥有者吧
            let is_same_owner = target_owner.0 == attacker_nation;

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
            commands.trigger(AttackRequestMessage {
                attacker: attacker_entity,
                target: target_entity,
            });

            for entity in attack_target_query.iter() {
                commands.entity(entity).despawn();
            }
            commands.entity(attacker_entity).remove::<SelectedUnit>();

            break;
        }
    }
}

/// 检查两个地块是否相邻
fn are_tiles_adjacent(tile1: Tile, tile2: Tile, tile_map: &TileMap) -> bool {
    let grid = tile_map.world_grid.grid;
    let offset1 = tile1.to_offset(grid);
    let offset2 = tile2.to_offset(grid);

    let dx = (offset1.0.x - offset2.0.x).abs();
    let dy = (offset1.0.y - offset2.0.y).abs();

    match grid.layout.orientation {
        HexOrientation::Pointy => dx <= 1 && dy <= 1 && (dx + dy) <= 2 && !(dx == 0 && dy == 0),
        HexOrientation::Flat => dx <= 1 && dy <= 1 && !(dx == 0 && dy == 0),
    }
}

// ============ 面板更新系统 ============

// ============ 选中单位动画 ============

/// 存储单位原始颜色，用于恢复
#[derive(Default)]
struct OriginalColors {
    inner: LinearRgba,
    outer: LinearRgba,
}

/// 选中单位动画 - 缩放 + 颜色高亮变化
/// 选中单位时：单位在 1.0 ~ 1.15 之间缩放，颜色在原始颜色和更亮颜色之间变化
/// 取消选中时：恢复原始大小和颜色
fn animate_selected_unit(
    time: Res<Time>,
    mut selected_query: Query<
        (
            Entity,
            &mut Transform,
            &MeshMaterial2d<ColorReplaceMaterial>,
        ),
        With<SelectedUnit>,
    >,
    mut unselected_query: Query<
        (
            Entity,
            &mut Transform,
            &MeshMaterial2d<ColorReplaceMaterial>,
        ),
        Without<SelectedUnit>,
    >,
    mut materials: ResMut<Assets<ColorReplaceMaterial>>,
    mut original_colors: Local<HashMap<Entity, OriginalColors>>,
) {
    // 使用正弦波实现平滑动画
    let t = (time.elapsed_secs() * 3.0).sin() * 0.5 + 0.5;
    let scale = 1.0 + t * 0.25;
    let brightness = 0.3 + t * 1.2;

    // 选中单位：直接修改缩放和颜色
    for (entity, mut transform, material_handle) in &mut selected_query {
        // 修改缩放（立即生效，无延迟命令）
        transform.scale = Vec3::splat(scale);

        // 颜色高亮
        if let Some(mut material) = materials.get_mut(&material_handle.0) {
            if !original_colors.contains_key(&entity) {
                original_colors.insert(
                    entity,
                    OriginalColors {
                        inner: material.inner_color,
                        outer: material.outer_color,
                    },
                );
            }
            if let Some(original) = original_colors.get(&entity) {
                material.inner_color = original.inner * brightness;
                material.outer_color = original.outer * brightness;
            }
        }
    }

    // 未选中单位：恢复原始大小和颜色
    for (entity, mut transform, material_handle) in &mut unselected_query {
        // 如果缩放不是 1，直接改回 1
        if transform.scale != Vec3::ONE {
            transform.scale = Vec3::ONE;
        }

        // 恢复颜色
        if let Some(original) = original_colors.remove(&entity) {
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.inner_color = original.inner;
                material.outer_color = original.outer;
            }
        }
    }
}

// ============ 面板更新系统 ============

/// 更新单位信息面板
/// - 环形进度条：根据 HP/XP 重新生成扇形进度图片（无需 shader）
/// - 力量行：仅军事单位显示
/// - 文本字段更新
fn update_unit_info_panel(
    mut panel: Single<&mut Visibility, With<UnitInfoPanel>>,
    ring_progress_query: Single<&MaterialNode<RingProgressMaterial>, With<UnitRingProgress>>,
    unit_icon_query: Single<&mut ImageNode, (With<UnitInfoIcon>, Without<UnitRingProgress>)>,
    mut text_fields: Query<(&mut Text, &UnitInfoField)>,
    selected_unit_query: Query<
        (&UnitComponent, &Health, &Strength, &Movement, &Experience),
        With<SelectedUnit>,
    >,
    mut ring_materials: ResMut<Assets<RingProgressMaterial>>,
    materials: Res<GameAssets>,
) {
    if let Ok((unit_component, health, strength, movement, experience)) =
        selected_unit_query.single()
    {
        // 显示面板
        **panel = Visibility::Visible;

        // 更新环形进度条材质（HP 左半环 + XP 右半环）
        let hp_ratio = health.current as f32 / health.max.max(1) as f32;
        let xp_ratio = experience.current as f32 / experience.max.max(1) as f32;
        if let Some(mut material) = ring_materials.get_mut(&ring_progress_query.0) {
            material.left_progress = hp_ratio;
            material.right_progress = xp_ratio;
        }

        // 更新单位图标
        let icon_char = unit_component.0.as_str();
        unit_icon_query.into_inner().image = materials.texture_handle(icon_char);

        // 更新所有文本字段
        for (mut text, field) in text_fields.iter_mut() {
            match field {
                UnitInfoField::Name => {
                    // 更新单位名称
                    text.0 = unit_component.0.as_str().to_string();
                }
                UnitInfoField::Strength => {
                    // 更新攻击力
                    if strength.0 > 0 {
                        text.0 = strength.0.to_string();
                    } else {
                        text.0 = "Non-combat".to_string();
                    }
                }
                UnitInfoField::Health => {
                    // 更新HP
                    text.0 = format!("{} / {}", health.current, health.max);
                }
                UnitInfoField::Movement => {
                    // 更新移动力
                    text.0 = format!("{} / {}", movement.current, movement.max);
                }
            }
        }
    } else {
        // 无选中单位时隐藏面板
        **panel = Visibility::Hidden;
    }
}
