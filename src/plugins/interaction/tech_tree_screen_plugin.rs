//! 科技插件
//!
//! 管理科技树屏幕、科技按钮、AI研究选择和科技管理器注册。

use bevy::{
    color::{
        Color,
        palettes::css::{BLACK, RED, WHITE},
    },
    math::Vec2,
    picking::events::{Drag, Pointer},
    prelude::*,
    ui::{
        BackgroundColor, BorderColor, Node, Overflow, PositionType, ScrollPosition, UiRect, Val,
        percent, widget::Text,
    },
};
use civ_map_generator::ruleset::{
    Ruleset, TechnologyInfo,
    enums::{EnumStr, Technology},
};
use enum_map::EnumMap;
use std::collections::HashMap;

use crate::{
    NationComponent, Player, SciencePerTurn, ScreenState, TurnManager,
    assets::GameAssets,
    plugins::tech::{
        OverflowScience, ResearchingTech, TechCostManager, TechProgressManager, TechState,
        TechStateManager,
    },
    resources::MapParametersRes,
};

/// 科技树列宽（像素）
const COLUMN_WIDTH: f32 = 400.0;
/// 时代头高度百分比
const ERA_HEADER_PERCENT: f32 = 5.0;
const LINE_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
const LINE_WIDTH: f32 = 2.0;

// ============ 科技树组件 ============

/// 科技按钮组件
#[derive(Component, Clone, Copy, Debug)]
pub struct TechButton(pub Technology);

/// 科技树可滚动节点
#[derive(Component)]
pub struct TechTreeScrollableNode;

/// 关闭科技树按钮
#[derive(Component)]
pub struct CloseTechTreeButton;

#[derive(Component)]
struct TechTree;

/// 科技插件
pub struct TechTreeScreenPlugin;

impl Plugin for TechTreeScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_tech_click_system.run_if(in_state(ScreenState::TechTree)),
        )
        .add_systems(OnEnter(ScreenState::TechTree), spawn_technology_screen);
    }
}

/// 处理科技按钮点击
fn handle_tech_click_system(
    tech_button_query: Query<(&Interaction, &TechButton)>,
    close_button_query: Query<(&Interaction, &CloseTechTreeButton)>,
    mut player_query: Query<(&mut ResearchingTech, &TechStateManager), With<Player>>,
    turn_manager: Res<TurnManager>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    // 只在玩家回合处理科技按钮点击
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok((mut researching_tech, tech_state_manager)) = player_query.get_mut(current_entity)
    else {
        return;
    };

    // 处理关闭按钮
    for (interaction, _) in &close_button_query {
        if *interaction == Interaction::Pressed {
            next_state.set(ScreenState::WorldMap);
            return;
        }
    }

    // 处理科技按钮
    for (interaction, tech_button) in &tech_button_query {
        let tech = tech_button.0;

        if *interaction != Interaction::Pressed {
            continue;
        }

        if !matches!(
            tech_state_manager.0[tech],
            TechState::Available | TechState::ResearchedAndRepeatable
        ) {
            continue;
        }

        researching_tech.0 = Some(tech);
        next_state.set(ScreenState::WorldMap);
    }
}

/// 生成科技树屏幕
fn spawn_technology_screen(
    mut commands: Commands,
    map_params: Res<MapParametersRes>,
    materials: Res<GameAssets>,
    player_query: Query<
        (
            &NationComponent,
            &ResearchingTech,
            &TechProgressManager,
            &TechStateManager,
            &TechCostManager,
            &OverflowScience,
            &SciencePerTurn,
        ),
        With<Player>,
    >,
    turn_manager: Res<TurnManager>,
) {
    let ruleset = &map_params.0.ruleset;

    // 只在实际玩家回合时能打开科技树
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok((
        _player_nation,
        researching_tech,
        tech_progress,
        tech_state_manager,
        tech_cost_manager,
        overflow_science,
        science_per_turn,
    )) = player_query.get(current_entity)
    else {
        return;
    };

    let tech_and_turns: EnumMap<Technology, String> = EnumMap::from_fn(|tech| {
        turns_to_tech(
            tech,
            science_per_turn.0,
            tech_progress,
            tech_state_manager,
            tech_cost_manager,
            overflow_science,
        )
    });

    // The total number of columns which tech button will be placed in the grid layout
    // Notes: the column number starts at 0 in ruleset,
    // so the total columns which the tech tree has will be the maximum column number + 1.
    let column_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.column)
        .max()
        .unwrap() as i16
        + 1;

    // The total number of rows which tech button will be placed in the grid layout
    // Notes: the row number starts from 1,
    // so the total rows which the tech tree has will be the maximum row number.
    let row_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.row)
        .max()
        .unwrap() as i16;

    // 收集所有连接关系（前置科技 -> 后续科技）
    let mut connections: Vec<(Technology, Technology)> = Vec::new();
    for (tech, tech_info) in ruleset.technologies.iter() {
        for prereq_name in &tech_info.prerequisites {
            connections.push((Technology::from_str(prereq_name), tech));
        }
    }

    let row_height_of_tech_nodes = (100. - ERA_HEADER_PERCENT) / row_count as f32;
    // 第一行是时代头，占据 `ERA_HEADER_PERCENT` 百分比高度
    // 后续每行是科技节点，占据 `row_height_of_tech_nodes` 百分比高度
    let row_tracks = std::iter::once(GridTrack::percent(ERA_HEADER_PERCENT))
        .chain(
            std::iter::repeat(GridTrack::percent(row_height_of_tech_nodes))
                .take(row_count as usize),
        )
        .collect::<Vec<_>>();

    let column_tracks: Vec<GridTrack> = vec![GridTrack::px(COLUMN_WIDTH); column_count as usize];

    let tech_button_bg_color = |technology| {
        if Some(technology) == researching_tech.0 {
            Color::srgb(0.2, 0.4, 0.8)
        } else {
            match tech_state_manager.0[technology] {
                TechState::Available | TechState::ResearchedAndRepeatable => {
                    Color::srgb(0.2, 0.5, 0.2)
                }
                TechState::Researched => Color::srgb(0.5, 0.5, 0.5),
                TechState::Locked => Color::NONE,
            }
        }
    };

    commands
        .spawn((
            DespawnOnExit(ScreenState::TechTree),
            Node {
                width: percent(100),
                height: percent(100),
                overflow: Overflow::scroll_x(),
                ..Default::default()
            },
            ZIndex(1),
            ScrollPosition(Vec2::ZERO),
            TechTreeScrollableNode,
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .observe(
            |drag: On<Pointer<Drag>>,
             mut scroll_position_query: Query<
                (&mut ScrollPosition, &Node, &ComputedNode),
                With<TechTreeScrollableNode>,
            >| {
                if let Ok((mut scroll_position, node, computed)) =
                    scroll_position_query.single_mut()
                {
                    let max_offset = (computed.content_size() - computed.size())
                        * computed.inverse_scale_factor();
                    let delta = drag.delta;
                    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
                        let max = if delta.x > 0. {
                            scroll_position.x >= max_offset.x
                        } else {
                            scroll_position.x <= 0.
                        };

                        if !max {
                            scroll_position.x += delta.x;
                        }
                    }
                }
            },
        )
        .with_children(|builder| {
            builder
                .spawn((
                    Node {
                        display: Display::Grid,
                        height: percent(100),
                        grid_auto_rows: row_tracks,
                        grid_auto_columns: column_tracks,
                        ..default()
                    },
                    TechTree,
                ))
                .with_children(|builder| {
                    // ============ 绘制连接线 ============
                    for &(prereq_tech, tech) in &connections {
                        let (prereq_tech_info, tech_info) = (
                            &ruleset.technologies[prereq_tech],
                            &ruleset.technologies[tech],
                        );

                        let (x1, y1) =
                            get_tech_position(&prereq_tech_info, row_height_of_tech_nodes);
                        let (x2, y2) = get_tech_position(&tech_info, row_height_of_tech_nodes);

                        // 绘制从前置科技到后续科技的连线
                        // 使用正交线（直角弯折）风格，类似文明5
                        draw_tech_connection(builder, x1, y1, x2, y2);
                    }

                    // ============ 绘制时代头 ============
                    let mut era_spans: HashMap<String, (i16, i16)> = HashMap::new();
                    for technology in ruleset.technologies.values() {
                        let entry = era_spans
                            .entry(technology.era.clone())
                            .or_insert((technology.column as i16, technology.column as i16));
                        entry.0 = entry.0.min(technology.column as i16);
                        entry.1 = entry.1.max(technology.column as i16);
                    }

                    for (era_name, (min_col, max_col)) in era_spans {
                        let span = (max_col - min_col + 1) as u16;
                        builder.spawn((
                            Node {
                                grid_row: GridPlacement::start(1),
                                grid_column: GridPlacement::start(min_col + 1).set_span(span),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                            BorderColor::all(Color::WHITE),
                            children![(
                                Text::new(era_name),
                                TextFont {
                                    font_size: FontSize::Vh(3.0),
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            )],
                        ));
                    }

                    // ============ 绘制科技节点 ============
                    ruleset
                        .technologies
                        .iter()
                        .for_each(|(technology, technology_info)| {
                            let tech_turn = &tech_and_turns[technology];

                            builder.spawn((
                                Node {
                                    grid_row: GridPlacement::start(technology_info.row as i16 + 1),
                                    grid_column: GridPlacement::start(
                                        technology_info.column as i16 + 1,
                                    ),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                children![technology_button(
                                    technology,
                                    &materials,
                                    ruleset,
                                    tech_button_bg_color(technology),
                                    tech_turn
                                )],
                            ));
                        });
                });
        });

    // 关闭按钮
    commands.spawn((
        DespawnOnExit(ScreenState::TechTree),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        ZIndex(2),
        BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
        BorderColor::all(Color::WHITE),
        CloseTechTreeButton,
        Button,
        children![(
            Text::new("X"),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}

/// 绘制两个科技节点之间的连接线
/// 使用正交线风格（先水平、再垂直、再水平），类似文明5
fn draw_tech_connection(builder: &mut ChildSpawnerCommands, x1: f32, y1: f32, x2: f32, y2: f32) {
    let mid_x = (x1 + x2) / 2.0;

    // 水平线1：从起点到中点
    create_horizontal_line(builder, x1, mid_x, y1);
    // 垂直线：从中点到目标高度
    create_vertical_line(builder, mid_x, y1, y2);
    // 水平线2：从中点到目标
    create_horizontal_line(builder, mid_x, x2, y2);
}

/// 创建水平线
/// x 为像素坐标（列宽固定），y 为百分比坐标（行高为百分比）
fn create_horizontal_line(builder: &mut ChildSpawnerCommands, x1: f32, x2: f32, y: f32) {
    let start_x = x1.min(x2);
    let end_x = x1.max(x2);
    let width = (end_x - start_x).max(1.0);

    if width < 2.0 {
        return; // 忽略太短的线
    }

    builder.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(start_x),
            top: Val::Percent(y),
            width: Val::Px(width),
            height: Val::Px(LINE_WIDTH),
            ..Default::default()
        },
        BackgroundColor(LINE_COLOR),
        ZIndex(0),
    ));
}

/// 创建垂直线
/// x 为像素坐标（列宽固定），y 为百分比坐标（行高为百分比）
fn create_vertical_line(builder: &mut ChildSpawnerCommands, x: f32, y1: f32, y2: f32) {
    let start_y = y1.min(y2);
    let end_y = y1.max(y2);
    let height = (end_y - start_y).max(0.1);

    if height < 0.2 {
        return; // 忽略太短的线
    }

    builder.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x - LINE_WIDTH / 2.0),
            top: Val::Percent(start_y),
            width: Val::Px(LINE_WIDTH),
            height: Val::Percent(height),
            ..Default::default()
        },
        BackgroundColor(LINE_COLOR),
        ZIndex(0),
    ));
}

/// 创建科技按钮
fn technology_button(
    technology: Technology,
    materials: &GameAssets,
    ruleset: &Ruleset,
    bg_color: Color,
    tech_turn: &str,
) -> impl Bundle {
    (
        Node {
            width: percent(70),
            height: percent(90),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderColor::all(Color::WHITE),
        Button,
        TechButton(technology),
        children![(
            Node {
                display: Display::Grid,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                grid_template_columns: vec![
                    GridTrack::percent(20.),
                    GridTrack::fr(1.0),
                    GridTrack::px(80.0)
                ],
                grid_template_rows: vec![GridTrack::percent(25.), GridTrack::percent(75.0)],
                ..default()
            },
            children![
                (
                    Node {
                        grid_column: GridPlacement::start(1),
                        grid_row: GridPlacement::start(1).set_span(2),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Percent(5.0)),
                        ..default()
                    },
                    children![(
                        Node {
                            width: Val::Auto,
                            height: percent(100),
                            aspect_ratio: Some(1.0),
                            border: UiRect::all(Val::Percent(5.0)),
                            border_radius: BorderRadius::all(px(f32::MAX)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        ImageNode::new(materials.texture_handle(technology.as_str()))
                            .with_color(RED.into()),
                        Outline {
                            width: px(2),
                            offset: px(0),
                            color: Color::WHITE,
                        },
                    ),],
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(2),
                        grid_row: GridPlacement::start(1),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    Text::new(technology.as_str()),
                    TextFont {
                        font_size: FontSize::Vh(1.5),
                        ..Default::default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(3),
                        grid_row: GridPlacement::start(1),
                        justify_content: JustifyContent::End,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    Text::new(tech_turn),
                    TextFont {
                        font_size: FontSize::Vh(1.5),
                        ..Default::default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(2).set_span(2),
                        grid_row: GridPlacement::start(2),
                        height: Val::Percent(90.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        margin: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(Color::WHITE),
                    children![tech_unlock_item_list(technology, ruleset, materials)],
                )
            ]
        )],
    )
}

/// 创建科技解锁项目列表
/// TODO: 可能需要文明独有的单位来替换通用的建筑、单位等等，当前所有文明均使用通用项目
fn tech_unlock_item_list(
    technology: Technology,
    ruleset: &Ruleset,
    materials: &GameAssets,
) -> impl Bundle {
    let units = &ruleset.units;
    let unlock_units = units
        .values()
        .filter(|unit| unit.required_tech == technology.as_str() && unit.unique_to.is_empty());

    let buildings = &ruleset.buildings;
    let unlock_buildings: Vec<_> = buildings
        .values()
        .filter(|building| {
            building.required_tech == technology.as_str() && building.unique_to.is_empty()
        })
        .map(|building| building.name.clone())
        .collect();

    let tile_improvements = &ruleset.tile_improvements;
    let unlock_tile_improvements = tile_improvements.values().filter(|tile_improvement| {
        tile_improvement.required_tech == technology.as_str()
            && tile_improvement.unique_to.is_empty()
    });

    let unlock_uniques = ruleset.technologies[technology].uniques.clone();

    let unit_materials: Vec<_> = unlock_units
        .map(|unit| materials.texture_handle(&unit.name))
        .collect();

    let building_materials: Vec<_> = unlock_buildings
        .iter()
        .map(|building_name| materials.texture_handle(building_name))
        .collect();

    let tile_improvement_materials: Vec<_> = unlock_tile_improvements
        .map(|tile_improvement| materials.texture_handle(&tile_improvement.name))
        .collect();

    let unique_material = materials.texture_handle("Fallback");

    (
        Node {
            width: Val::Percent(90.),
            height: Val::Percent(90.),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::fr(5, 1.),
            ..default()
        },
        Children::spawn((
            SpawnIter(
                unit_materials
                    .into_iter()
                    .chain(building_materials)
                    .chain(tile_improvement_materials)
                    .map(|building_name| {
                        (
                            Node {
                                height: Val::Percent(100.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            children![unit_or_building_or_tile_improvement_item(building_name)],
                        )
                    }),
            ),
            SpawnIter(unlock_uniques.into_iter().map(move |_| {
                (
                    Node {
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![unique_item(unique_material.clone())],
                )
            })),
        )),
    )
}

/// 创建单位/建筑/地块改良图标
fn unit_or_building_or_tile_improvement_item(texture: Handle<Image>) -> impl Bundle {
    (
        Node {
            height: Val::Percent(80.0),
            aspect_ratio: Some(1.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(px(f32::MAX)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ImageNode::new(texture).with_color(BLACK.into()),
        BackgroundColor(WHITE.into()),
        Outline {
            width: px(1),
            offset: px(0),
            color: Color::WHITE,
        },
    )
}

/// 创建独特能力图标
fn unique_item(texture: Handle<Image>) -> impl Bundle {
    (
        Node {
            height: Val::Percent(80.0),
            aspect_ratio: Some(1.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(px(f32::MAX)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ImageNode::new(texture).with_color(BLACK.into()),
        BackgroundColor(WHITE.into()),
        Outline {
            width: px(1),
            offset: px(0),
            color: Color::WHITE,
        },
    )
}

/// 获取科技在网格中的坐标（中心点）
/// 返回 (x像素, y百分比)，y基于整个Screen高度的占比
fn get_tech_position(tech_info: &TechnologyInfo, row_height_of_tech_nodes: f32) -> (f32, f32) {
    let x = (tech_info.column as f32) * COLUMN_WIDTH + COLUMN_WIDTH / 2.0;
    let row_height_percent = row_height_of_tech_nodes;
    let y = ERA_HEADER_PERCENT
        + (tech_info.row as f32 - 1.) * row_height_percent
        + row_height_percent / 2.0;
    (x, y)
}

/// 计算完成指定科技还需要的剩余科学点数。
///
/// 该函数根据科技当前状态、已投入研究点数、科技总成本以及可用的溢出科学点数，
/// 计算还需要多少科学点数才能完成该科技。
///
/// # Arguments
///
/// * `tech` - 要计算剩余点数的科技。**调用前必须确保该科技不是已研究完成的状态
///   （`TechState::Researched`）**，否则函数会 panic。
/// * `tech_progress` - 管理各科技已投入研究点数的管理器。
/// * `tech_state_manager` - 管理各科技当前状态的管理器。
/// * `tech_cost_manager` - 管理各科技总成本的管理器。
/// * `overflow_science` - 当前可用的溢出科学点数。
///
/// # Returns
///
/// 返回还需要投入的科学点数（`u32`）。如果计算出的剩余点数为 0，则返回 0。
///
/// # Panics
///
/// * 如果 `tech` 的状态为 `TechState::Researched`，会直接触发 panic，
///   因为已经研究完成的科技不应该再计算剩余科学点数。
pub fn remaining_science_to_tech(
    tech: Technology,
    tech_progress: &TechProgressManager,
    tech_state_manager: &TechStateManager,
    tech_cost_manager: &TechCostManager,
    overflow_science: &OverflowScience,
) -> u32 {
    let spare_science = match tech_state_manager.0[tech] {
        TechState::Researched => panic!("Technology is already researched"),
        TechState::Available | TechState::ResearchedAndRepeatable => overflow_science.0,   
        TechState::Locked => 0,
    };

    let cost = tech_cost_manager.0.get(&tech).copied().expect("Tech cost not found");
    let researched = tech_progress.0.get(&tech).copied().unwrap_or(0);

    // 避免下溢：如果已投入 + 溢出 >= 成本，剩余为 0
    cost.saturating_sub(researched).saturating_sub(spare_science)
}

/// 计算完成科技还需要的回合数
pub fn turns_to_tech(
    tech: Technology,
    science_per_turn: u32,
    tech_progress: &TechProgressManager,
    tech_state_manager: &TechStateManager,
    tech_cost_manager: &TechCostManager,
    overflow_science: &OverflowScience,
) -> String {
    if tech_state_manager.0[tech] == TechState::Researched {
        return String::new();
    }

    let remaining_cost = remaining_science_to_tech(
        tech,
        tech_progress,
        tech_state_manager,
        tech_cost_manager,
        overflow_science,
    );

    if remaining_cost == 0 {
        return String::new();
    }

    if science_per_turn == 0 {
        return "∞ turns".to_string();
    }

    let turns = remaining_cost.div_ceil(science_per_turn).max(1);
    format!("{} turns", turns)
}
