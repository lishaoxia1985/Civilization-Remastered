//! 城市建造屏幕插件
//!
//! 当玩家在城市操作菜单点击"Build"时进入 [`ScreenState::CityConstruction`] 状态。
//! 进入该状态时显示一个全屏建造界面：
//! - 左侧为竖向可建造列表（仿文明5：先列出可建造的"Unit"，再列出可建造的"Building"）；
//! - 右侧为当前城市已经拥有的建筑列表。
//! 点击建造项会设置城市的 `current_production`；通过关闭按钮或 `Esc` 返回世界地图。
//! 退出该状态时，所有界面节点通过 `DespawnOnExit` 自动销毁。

use bevy::prelude::*;
use civ_map_generator::ruleset::{
    Ruleset,
    enums::{EnumStr, Technology},
};

use crate::{
    NationComponent, ScreenState, TurnManager,
    assets::GameAssets,
    components::{City, CityProduction, CityYields, SelectedCity},
    plugins::tech::{TechStateManager, is_researched},
    resources::MapParametersRes,
};

/// 城市建造屏幕根节点
#[derive(Component)]
pub struct CityConstructionScreen;

/// 可重建的内容节点（可建造列表 + 已拥有建筑列表）
///
/// 当选中城市的 `City` 数据变化（例如设定了当前生产）时会被重建。
#[derive(Component)]
pub struct CityConstructionContent;

/// 建造列表中的可建造项按钮
#[derive(Component)]
pub struct CityConstructionItemButton {
    /// 项目标签
    pub name: String,
    /// 生产方式
    pub production: CityProduction,
}

/// 关闭建造屏幕按钮
#[derive(Component)]
pub struct CloseCityConstructionButton;

/// 建造项按钮基础颜色
const ITEM_BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.6);
/// 分组大标题颜色（Unit / Building）
const HEADER_COLOR: Color = Color::srgb(1.0, 0.84, 0.0);

/// 城市建造屏幕插件
pub struct CityConstructionScreenPlugin;

impl Plugin for CityConstructionScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(ScreenState::CityConstruction),
            spawn_city_construction_screen,
        )
        .add_systems(
            Update,
            (
                handle_city_construction_item_click,
                close_city_construction_screen,
                refresh_city_construction_screen,
            )
                .run_if(in_state(ScreenState::CityConstruction)),
        );
    }
}
/// 进入建造屏幕时生成界面
fn spawn_city_construction_screen(
    mut commands: Commands,
    selected_city_query: Query<(&City, &CityYields), With<SelectedCity>>,
    materials: Res<GameAssets>,
    map_params: Res<MapParametersRes>,
    tech_state_query: Query<&TechStateManager, With<NationComponent>>,
    turn_manager: Res<TurnManager>,
) {
    let Ok((city, yields)) = selected_city_query.single() else {
        return;
    };
    let ruleset = &map_params.0.ruleset;
    let current_nation_entity = turn_manager.current_nation_entity();
    let tech_state_manager = tech_state_query.get(current_nation_entity).ok();

    commands
        .spawn((
            DespawnOnExit(ScreenState::CityConstruction),
            CityConstructionScreen,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(10.0),
                top: Val::Percent(6.0),
                width: Val::Percent(80.0),
                height: Val::Percent(88.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::hidden(),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.92)),
            BorderColor::all(Color::WHITE),
        ))
        .with_children(|builder| {
            spawn_content(
                builder,
                &materials,
                city,
                yields.production.max(1),
                ruleset,
                tech_state_manager,
            );

            // 关闭按钮（右上角）
            builder.spawn((
                DespawnOnExit(ScreenState::CityConstruction),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(5.0),
                    right: Val::Px(5.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.7, 0.2, 0.2)),
                BorderColor::all(Color::WHITE),
                Text::new("Close (Esc)"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Button,
                CloseCityConstructionButton,
            ));
        });
}

/// 生成建造屏幕内容（左：可建造列表；右：已拥有建筑列表）
fn spawn_content(
    builder: &mut ChildSpawnerCommands,
    materials: &GameAssets,
    city: &City,
    production_per_turn: u32,
    ruleset: &Ruleset,
    tech_state_manager: Option<&TechStateManager>,
) {
    builder
        .spawn((
            DespawnOnExit(ScreenState::CityConstruction),
            CityConstructionContent,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                overflow: Overflow::hidden(),
                ..Default::default()
            },
        ))
        .with_children(|content| {
            // ============ 左侧：竖向可建造列表 ============
            content
                .spawn((
                    DespawnOnExit(ScreenState::CityConstruction),
                    Node {
                        width: Val::Percent(58.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        overflow: Overflow::scroll_y(),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.6)),
                ))
                .with_children(|left| {
                    // 标题
                    left.spawn((
                        Text::new(format!("{} - Production", city.name)),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // 当前生产
                    let current = match &city.current_production {
                        Some(CityProduction::Building(building)) => {
                            format!("Building: {}", building.as_str())
                        }
                        Some(CityProduction::Unit(unit)) => format!("Producing: {}", unit.as_str()),
                        None => "Nothing selected - click an item below".to_string(),
                    };
                    left.spawn((
                        Text::new(current),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..Default::default()
                        },
                        TextColor(Color::srgb(0.5, 1.0, 0.5)),
                    ));

                    // ---- Unit 分组大标题 ----
                    left.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            border: UiRect::bottom(Val::Px(1.0)),
                            margin: UiRect::top(Val::Px(6.0)),
                            ..Default::default()
                        },
                        BorderColor::all(Color::WHITE),
                        Text::new("Unit"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..Default::default()
                        },
                        TextColor(HEADER_COLOR),
                    ));
// 可建造单位列表
                    for (unit, unit_info) in ruleset.units.iter() {
                        if unit_info.strength <= 0 {
                            continue;
                        }
                        let cost = unit_info.cost.max(0) as u32;
                        if cost <= 0 {
                            continue;
                        }

                        // 检查科技是否已解锁
                        if !unit_info.required_tech.is_empty() {
                            let required_tech = Technology::from_str(&unit_info.required_tech);
                            let tech_unlocked = tech_state_manager
                                .map(|manager| is_researched(required_tech, manager))
                                .unwrap_or(false);
                            if !tech_unlocked {
                                continue;
                            }
                        }

                        // 检查单位是否已过时（过时科技已研究则不可建造）
                        if !unit_info.obsolete_tech.is_empty() {
                            let obsolete_tech = Technology::from_str(&unit_info.obsolete_tech);
                            let is_obsolete = tech_state_manager
                                .map(|manager| is_researched(obsolete_tech, manager))
                                .unwrap_or(false);
                            if is_obsolete {
                                continue;
                            }
                        }

                        let turns = turns_estimate(cost, production_per_turn);
                        let label = format!("{} - {} ({} turns)", unit_info.name, cost, turns);
                        let icon = materials.texture_handle(&unit_info.name);
                        spawn_item_button(
                            left,
                            icon,
                            label,
                            CityProduction::Unit(unit),
                        );
                    }

                    // ---- Building 分组大标题 ----
                    left.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            border: UiRect::bottom(Val::Px(1.0)),
                            margin: UiRect::top(Val::Px(12.0)),
                            ..Default::default()
                        },
                        BorderColor::all(Color::WHITE),
                        Text::new("Building"),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..Default::default()
                        },
                        TextColor(HEADER_COLOR),
                    ));

                    // 可建造建筑列表
                    for (building, building_info) in ruleset.buildings.iter() {
                        if building_info.is_wonder || building_info.is_national_wonder {
                            continue;
                        }
                        if city.buildings.contains(&building) {
                            continue;
                        }
                        let cost = building_info.cost.max(0) as u32;
                        if cost <= 0 {
                            continue;
                        }

                        // 检查科技是否已解锁
                        if !building_info.required_tech.is_empty() {
                            let required_tech = Technology::from_str(&building_info.required_tech);
                            let tech_unlocked = tech_state_manager
                                .map(|manager| is_researched(required_tech, manager))
                                .unwrap_or(false);
                            if !tech_unlocked {
                                continue;
                            }
                        }

                        let turns = turns_estimate(cost, production_per_turn);
                        let label = format!("{} - {} ({} turns)", building_info.name, cost, turns);
                        let icon = materials.texture_handle(&building_info.name);
                        spawn_item_button(
                            left,
                            icon,
                            label,
                            CityProduction::Building(building),
                        );
                    }
                });
// ============ 右侧：已拥有建筑列表 ============
            content
                .spawn((
                    DespawnOnExit(ScreenState::CityConstruction),
                    Node {
                        width: Val::Percent(42.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        overflow: Overflow::scroll_y(),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.6)),
                ))
                .with_children(|right| {
                    // 标题
                    right.spawn((
                        Text::new(format!("{} - Owned Buildings", city.name)),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    if city.buildings.is_empty() {
                        right.spawn((
                            Text::new("None"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..Default::default()
                            },
                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        ));
                    } else {
                        for &building in &city.buildings {
                            let building_info = &ruleset.buildings[building];
                            let name = building_info.name.clone();
                            let icon = materials.texture_handle(&building_info.name);
                            right
                                .spawn((
                                    DespawnOnExit(ScreenState::CityConstruction),
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Auto,
                                        border: UiRect::all(Val::Px(1.0)),
                                        padding: UiRect::all(Val::Px(4.0)),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(8.0),
                                        ..Default::default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.6)),
                                    BorderColor::all(Color::WHITE),
                                ))
                                .with_children(|item| {
                                    item.spawn((
                                        DespawnOnExit(ScreenState::CityConstruction),
                                        Node {
                                            width: Val::Px(32.0),
                                            height: Val::Px(32.0),
                                            flex_shrink: 0.0,
                                            ..Default::default()
                                        },
                                        ImageNode::new(icon)
                                            .with_mode(bevy::ui::widget::NodeImageMode::Stretch),
                                    ));
                                    item.spawn((
                                        DespawnOnExit(ScreenState::CityConstruction),
                                        Text::new(name),
                                        TextFont {
                                            font_size: FontSize::Px(13.0),
                                            ..Default::default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        }
                    }
                });
        });
}

/// 生成一个可建造项目按钮（横排：图标 + 文字）
fn spawn_item_button(
    builder: &mut ChildSpawnerCommands,
    icon: Handle<Image>,
    label: String,
    production: CityProduction,
) {
    builder
        .spawn((
            DespawnOnExit(ScreenState::CityConstruction),
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(4.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..Default::default()
            },
            BackgroundColor(ITEM_BUTTON_COLOR),
            BorderColor::all(Color::WHITE),
            Button,
            CityConstructionItemButton { name: label.clone(), production },
        ))
        .with_children(|btn| {
            btn.spawn((
                DespawnOnExit(ScreenState::CityConstruction),
                Node {
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                },
                ImageNode::new(icon).with_mode(bevy::ui::widget::NodeImageMode::Stretch),
            ));
            btn.spawn((
                DespawnOnExit(ScreenState::CityConstruction),
                Text::new(label.as_str()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
            ));
        });
}
/// 处理建造项目点击：设置城市的当前生产
fn handle_city_construction_item_click(
    button_query: Query<(&Interaction, &CityConstructionItemButton), Changed<Interaction>>,
    mut selected_city_query: Query<&mut City, With<SelectedCity>>,
) {
    for (interaction, item) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for mut city in selected_city_query.iter_mut() {
            city.current_production = Some(item.production.clone());
            info!("City {} started producing: {}", city.name, item.name);
        }
    }
}

/// 处理关闭按钮 / Esc 退出建造屏幕
fn close_city_construction_screen(
    button_query: Query<(&Interaction, &CloseCityConstructionButton)>,
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    for (interaction, _) in &button_query {
        if *interaction == Interaction::Pressed {
            next_state.set(ScreenState::WorldMap);
            return;
        }
    }
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(ScreenState::WorldMap);
    }
}

/// 当选中城市的 `City` 数据变化（如设定当前生产）时重建列表内容
fn refresh_city_construction_screen(
    mut commands: Commands,
    root_query: Query<Entity, With<CityConstructionScreen>>,
    content_query: Query<Entity, With<CityConstructionContent>>,
    changed_city: Query<(), (With<SelectedCity>, Changed<City>)>,
    selected_city_query: Query<(&City, &CityYields), With<SelectedCity>>,
    materials: Res<GameAssets>,
    map_params: Res<MapParametersRes>,
    tech_state_query: Query<&TechStateManager, With<NationComponent>>,
    turn_manager: Res<TurnManager>,
) {
    if changed_city.single().is_err() {
        return;
    }
    let Ok(root) = root_query.single() else {
        return;
    };
    // 删除旧内容并重建（延迟执行）
    if let Ok(content_entity) = content_query.single() {
        commands.entity(content_entity).despawn();
    }
    let Ok((city, yields)) = selected_city_query.single() else {
        return;
    };
    let ruleset = &map_params.0.ruleset;
    let tech_state_manager = tech_state_query.get(turn_manager.current_nation_entity()).ok();
    commands
        .entity(root)
        .with_children(|builder| {
            spawn_content(
                builder,
                &materials,
                city,
                yields.production.max(1),
                ruleset,
                tech_state_manager,
            );
        });
}

/// 估算生产所需回合数
fn turns_estimate(cost: u32, production_per_turn: u32) -> u32 {
    if production_per_turn == 0 {
        0
    } else {
        (cost as f32 / production_per_turn as f32).ceil() as u32
    }
}