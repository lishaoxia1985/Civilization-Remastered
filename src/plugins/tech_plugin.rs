//! 科技插件
//!
//! 管理科技树屏幕、科技按钮、AI研究选择和科技管理器注册。

use bevy::{
    color::{
        Color,
        palettes::css::{BLACK, RED, WHITE},
    },
    math::Vec2,
    picking::{
        events::{Click, Drag, Pointer},
        pointer::PointerButton,
    },
    prelude::*,
    ui::{
        BackgroundColor, BorderColor, Node, Overflow, PositionType, ScrollPosition, UiRect, Val,
        percent, widget::Text,
    },
};
use civ_map_generator::ruleset::{
    Ruleset,
    enums::{EnumStr, Technology},
};
use enum_map::EnumMap;
use std::collections::HashMap;

use crate::{
    assets::{GameAssets, ScreenState},
    components::{CloseTechTreeButton, TechButton, TechButtonState, TechTreeScrollableNode},
    resources::{CivilizationManager, GameSettings, MapParametersRes, TechManagerRegistry},
};

/// 科技插件
pub struct TechPlugin;

impl Plugin for TechPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::assets::AppState::GameStart),
            (setup_tech_button, insert_tech_manager_registry),
        )
        .add_systems(
            Update,
            ai_research_system.in_set(crate::resources::GameSystemGroup::PlayOnWorldMap),
        )
        .add_systems(
            Update,
            handle_tech_click_system.in_set(crate::resources::GameSystemGroup::PlayOnTechScreen),
        )
        .add_systems(OnEnter(ScreenState::TechTree), spawn_technology_screen);
    }
}

/// 插入科技管理器注册表资源
fn insert_tech_manager_registry(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    civ_manager: Res<CivilizationManager>,
) {
    commands.insert_resource(TechManagerRegistry::new(
        &civ_manager,
        game_settings.start_era,
    ));
}

/// 判断科技状态
fn determine_tech_state(
    technology: Technology,
    player: &crate::resources::CivData,
    map_params: &MapParametersRes,
    tech_registry: &TechManagerRegistry,
) -> TechButtonState {
    let play_nation = player.nation;
    let tech_manager = &tech_registry.0[&play_nation];

    if tech_manager.is_researched(technology) {
        return TechButtonState::Researched;
    }

    if tech_manager.current_researching_technology() == Some(technology) {
        return TechButtonState::InProgress;
    }

    if !tech_manager.can_be_researched(technology, map_params) {
        return TechButtonState::Locked;
    }

    TechButtonState::Available
}

/// AI 自动选择要研究的科技
fn ai_research_system(
    mut civ_manager: ResMut<CivilizationManager>,
    mut tech_registry: ResMut<TechManagerRegistry>,
    map_params: Res<MapParametersRes>,
) {
    for enemy_nation in &civ_manager.enemy_nations.clone() {
        if let Some(enemy) = civ_manager.civs.get_mut(enemy_nation) {
            enemy.ai_choose_research(&map_params, &mut tech_registry);
        }
    }
}

/// 处理科技按钮点击
fn handle_tech_click_system(
    mut click_events: MessageReader<Pointer<Click>>,
    tech_button_query: Query<&TechButton>,
    parent_query: Query<&ChildOf>,
    civs: Res<CivilizationManager>,
    mut tech_registry: ResMut<TechManagerRegistry>,
    map_params: Res<MapParametersRes>,
    close_tech_tree_button_query: Query<Entity, With<CloseTechTreeButton>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    for click in click_events.read() {
        let mut target = click.event_target();

        if close_tech_tree_button_query.get(target).is_ok() {
            next_state.set(ScreenState::WorldMap);
            continue;
        }

        loop {
            if let Ok(tech_button) = tech_button_query.get(target) {
                let player_nation = civs.player_nation;
                let tech_manager = &tech_registry.0[&player_nation];
                if !tech_manager.can_be_researched(tech_button.0, &map_params) {
                    break;
                }
                civs.player_data()
                    .start_research(tech_button.0, &mut tech_registry);
                next_state.set(ScreenState::WorldMap);
                break;
            }
            match parent_query.get(target) {
                Ok(parent) => target = parent.parent(),
                Err(_) => break,
            }
        }
    }
}

/// 设置打开科技树的按钮
fn setup_tech_button(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Auto,
                height: Val::Auto,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::WHITE),
            Text::new("Open Tech Tree"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
        ))
        .observe(open_tech_tree);
}

/// 打开科技树
fn open_tech_tree(drag: On<Pointer<Click>>, mut next_state: ResMut<NextState<ScreenState>>) {
    if matches!(drag.button, PointerButton::Primary) {
        next_state.set(ScreenState::TechTree);
    }
}

/// 生成科技树屏幕
fn spawn_technology_screen(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
    materials: Res<GameAssets>,
    civs: Res<CivilizationManager>,
    tech_registry: Res<TechManagerRegistry>,
) {
    let ruleset = &map_params.0.ruleset;
    let player_nation = civs.player_nation;
    let player_data = civs.player_data();
    let tech_manager = &tech_registry.0[&player_nation];

    let tech_and_turns: EnumMap<Technology, String> = EnumMap::from_fn(|tech| {
        tech_manager.turns_to_tech(
            tech,
            player_data.science_per_turn,
            player_data,
            &game_settings,
            &map_params,
        )
    });

    let column_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.column)
        .max()
        .unwrap() as i16
        + 1;

    let row_count = ruleset
        .technologies
        .values()
        .map(|technology| technology.row)
        .max()
        .unwrap() as i16
        + 1;

    let mut row_tracks: Vec<GridTrack> = Vec::new();
    row_tracks.push(GridTrack::percent(5.0));

    for _ in 0..row_count {
        row_tracks.push(GridTrack::fr(1.0));
    }

    let column_tracks: Vec<GridTrack> = vec![GridTrack::px(400.0); column_count as usize];

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
                .spawn(Node {
                    display: Display::Grid,
                    grid_auto_rows: row_tracks,
                    grid_auto_columns: column_tracks,
                    ..default()
                })
                .with_children(|builder| {
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
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                            BorderColor::all(Color::WHITE),
                            children![(
                                Text::new(era_name),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..Default::default()
                                },
                                TextColor(Color::WHITE),
                            )],
                        ));
                    }

                    ruleset
                        .technologies
                        .iter()
                        .for_each(|(technology, technology_info)| {
                            let player = civs.player_data();
                            let tech_state = determine_tech_state(
                                technology,
                                player,
                                &map_params,
                                &tech_registry,
                            );
                            let tech_turn = &tech_and_turns[technology];

                            builder.spawn((
                                Node {
                                    grid_row: GridPlacement::start(technology_info.row as i16 + 1),
                                    grid_column: GridPlacement::start(
                                        technology_info.column as i16 + 1,
                                    ),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                Pickable {
                                    should_block_lower: false,
                                    is_hoverable: true,
                                },
                                tech_state.clone(),
                                children![technology_button(
                                    technology, &materials, ruleset, tech_state, tech_turn
                                )],
                            ));
                        });
                });
        });

    commands.spawn((
        DespawnOnExit(ScreenState::TechTree),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        ZIndex(2),
        BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
        BorderColor::all(Color::WHITE),
        CloseTechTreeButton,
        Pickable::default(),
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

/// 创建科技按钮
fn technology_button(
    technology: Technology,
    materials: &GameAssets,
    ruleset: &Ruleset,
    tech_state: TechButtonState,
    tech_turn: &str,
) -> impl Bundle {
    let bg_color = match tech_state {
        TechButtonState::Available => Color::srgb(0.2, 0.5, 0.2),
        TechButtonState::InProgress => Color::srgb(0.2, 0.4, 0.8),
        TechButtonState::Researched => Color::srgb(0.5, 0.5, 0.5),
        TechButtonState::Locked => Color::NONE,
    };

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
        Pickable::default(),
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
                        font_size: FontSize::Px(12.0),
                        ..Default::default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Node {
                        grid_column: GridPlacement::start(3),
                        grid_row: GridPlacement::start(1),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    Text::new(tech_turn),
                    TextFont {
                        font_size: FontSize::Px(12.0),
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

/// 创建科技解锁物品列表
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
            width: px(25),
            height: px(25),
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
            width: px(25),
            height: px(25),
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
