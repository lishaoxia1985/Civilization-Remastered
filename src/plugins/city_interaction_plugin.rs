//! 城市交互插件
//!
//! 管理玩家与城市的交互操作，包括：
//! - 城市选择（点击城市地块选择城市）
//! - 城市信息面板（显示城市名称、人口、收益等）
//! - 城市操作菜单（建造建筑、生产单位、管理市民）
//! - 城市建造界面（选择要建造的建筑/单位）
//! - 市民工作分配界面（选择市民工作的地块）

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::EnumStr;

use crate::{
    AppState, ScreenState,
    components::{CitizenAssignActive, City, CityProduction, CityYields, SelectedCity},
    resources::MapParametersRes,
};

// ============ 城市按钮样式 ============

/// 城市按钮基础颜色（与单位按钮一致）
pub const CITY_BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.6);
/// 城市按钮激活颜色（分配市民时高亮）
pub const CITY_BUTTON_ACTIVE_COLOR: Color = Color::srgb(0.1, 0.8, 0.1);

/// 城市信息面板容器
#[derive(Component)]
pub struct CityInfoPanel;

/// 城市信息字段类型
#[derive(Component)]
pub enum CityInfoField {
    /// 城市名称
    Name,
    /// 人口
    Population,
    /// 粮食
    Food,
    /// 产能
    Production,
    /// 科研
    Science,
    /// 金币
    Gold,
    /// 文化
    Culture,
    /// 信仰
    Faith,
    /// 当前生产
    CurrentProduction,
    /// 已建造建筑
    Buildings,
    /// 建筑加成
    BuildingBonuses,
}

/// 城市操作菜单面板
#[derive(Component)]
pub struct CityActionMenu;

/// 城市操作按钮
#[derive(Component)]
pub enum CityActionButton {
    /// 建造（打开建造面板）
    Build,
    /// 分配市民（直接点击地块分配）
    AssignCitizens,
    /// 关闭城市面板
    Close,
}

// ============ 市民工作界面 ============

/// 城市市民管理面板
#[derive(Component)]
pub struct CityCitizenPanel;

/// 城市交互插件
pub struct CityInteractionPlugin;

impl Plugin for CityInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (
                setup_city_info_panel,
                setup_city_action_menu,
                setup_city_citizen_panel,
            ),
        )
        .add_systems(
            Update,
            update_city_info_panel.run_if(in_state(ScreenState::WorldMap)),
        )
        .add_systems(
            Update,
            (
                show_city_action_menu,
                handle_city_action_click,
                handle_assign_toggle,
            )
                .run_if(
                    in_state(ScreenState::WorldMap).or_else(in_state(ScreenState::CitizenAssign)),
                ),
        )
        .add_systems(
            OnExit(ScreenState::CitizenAssign),
            reset_assign_button_on_exit,
        );
    }
}

// ============ 城市信息面板 ============

/// 设置城市信息面板（左下角）
fn setup_city_info_panel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(60.0),
            width: Val::Px(300.0),
            height: Val::Auto,
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(5.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        BorderColor::all(Color::WHITE),
        Visibility::Hidden,
        CityInfoPanel,
        children![
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                CityInfoField::Name,
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                CityInfoField::Population
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                CityInfoField::Food
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.7, 0.2)),
                CityInfoField::Production
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.0, 0.5, 1.0)),
                CityInfoField::Science
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                CityInfoField::Gold
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.8, 0.4, 0.8)),
                CityInfoField::Culture
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
                CityInfoField::Faith
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.3, 0.8, 1.0)),
                CityInfoField::CurrentProduction
            ),
            // 已建造建筑列表
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                CityInfoField::Buildings,
            ),
            // 建筑加成
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..Default::default()
                },
                TextColor(Color::srgb(0.6, 1.0, 0.6)),
                CityInfoField::BuildingBonuses,
            ),
        ],
    ));
}

/// 更新城市信息面板
fn update_city_info_panel(
    panel: Single<&mut Visibility, With<CityInfoPanel>>,
    mut text_fields: Query<(&mut Text, &CityInfoField)>,
    selected_city_query: Query<(&City, &CityYields), With<SelectedCity>>,
    map_params: Res<MapParametersRes>,
) {
    if let Ok((city, yields)) = selected_city_query.single() {
        *panel.into_inner() = Visibility::Visible;
        let ruleset = &map_params.0.ruleset;

        for (mut text, field) in text_fields.iter_mut() {
            match field {
                CityInfoField::Name => text.0 = format!("🏙️ {}", city.name),
                CityInfoField::Population => text.0 = format!("Population: {}", city.population),
                CityInfoField::Food => {
                    text.0 = format!(
                        "Food: {} (+{}) / {} to grow",
                        city.food, yields.food, city.food_needed
                    )
                }
                CityInfoField::Production => {
                    text.0 = format!("Production: +{}/turn", yields.production)
                }
                CityInfoField::Science => text.0 = format!("Science: +{}/turn", yields.science),
                CityInfoField::Gold => text.0 = format!("Gold: +{}/turn", yields.gold),
                CityInfoField::Culture => {
                    text.0 = format!(
                        "Culture: +{}/turn ({} to expand)",
                        yields.culture, city.culture_to_expand
                    )
                }
                CityInfoField::Faith => text.0 = format!("Faith: +{}/turn", yields.faith),
                CityInfoField::CurrentProduction => {
                    text.0 = match &city.current_production {
                        Some(CityProduction::Building(building)) => {
                            format!(
                                "Building: {} ({} turns)",
                                building.as_str(),
                                turns_estimate(
                                    ruleset.buildings[*building].cost.max(0) as u32,
                                    yields.production
                                )
                            )
                        }
                        Some(CityProduction::Unit(unit)) => {
                            format!(
                                "Producing: {} ({} turns)",
                                unit.as_str(),
                                turns_estimate(
                                    ruleset.units[*unit].cost.max(0) as u32,
                                    yields.production
                                )
                            )
                        }
                        None => "Production: None - Click Build".to_string(),
                    };
                }
                CityInfoField::Buildings => {
                    // 显示已建造建筑列表
                    let buildings_str = if city.buildings.is_empty() {
                        "Buildings: None".to_string()
                    } else {
                        let names: Vec<String> = city
                            .buildings
                            .iter()
                            .map(|building| ruleset.buildings[*building].name.clone())
                            .collect();
                        format!("Buildings: {}", names.join(", "))
                    };
                    text.0 = buildings_str;
                }
                CityInfoField::BuildingBonuses => {
                    // 计算并显示建筑加成
                    let mut bonuses: Vec<String> = Vec::new();

                    for &building in &city.buildings {
                        let building_info = &ruleset.buildings[building];
                        let yields = &building_info.yields;

                        if yields.food > 0 {
                            bonuses.push(format!("+{} Food", yields.food));
                        }
                        if yields.production > 0 {
                            bonuses.push(format!("+{} Prod", yields.production));
                        }
                        if yields.science > 0 {
                            bonuses.push(format!("+{} Sci", yields.science));
                        }
                        if yields.gold > 0 {
                            bonuses.push(format!("+{} Gold", yields.gold));
                        }
                        if yields.culture > 0 {
                            bonuses.push(format!("+{} Culture", yields.culture));
                        }
                        if yields.faith > 0 {
                            bonuses.push(format!("+{} Faith", yields.faith));
                        }
                    }

                    text.0 = if bonuses.is_empty() {
                        "Bonuses: None".to_string()
                    } else {
                        format!("Bonuses: {}", bonuses.join(", "))
                    };
                }
            }
        }
    } else {
        *panel.into_inner() = Visibility::Hidden;
    }
}

/// 估算生产所需回合数
fn turns_estimate(cost: u32, production_per_turn: u32) -> u32 {
    if production_per_turn == 0 {
        0
    } else {
        (cost as f32 / production_per_turn as f32).ceil() as u32
    }
}

// ============ 城市操作菜单 ============

/// 设置城市操作菜单（右上角）
fn setup_city_action_menu(mut commands: Commands) {
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
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        Visibility::Hidden,
        CityActionMenu,
    ));
}

/// 显示城市操作菜单
fn show_city_action_menu(
    action_menu: Single<(Entity, &mut Visibility, Option<&mut Children>), With<CityActionMenu>>,
    selected_city_query: Query<(), With<SelectedCity>>,
    mut commands: Commands,
    mut last_selected: Local<Option<Entity>>,
) {
    let (menu_entity, mut visibility, children_option) = action_menu.into_inner();

    if selected_city_query.single().is_ok() {
        *visibility = Visibility::Visible;

        if *last_selected == Some(menu_entity) {
            return;
        }
        *last_selected = Some(menu_entity);

        if let Some(children) = children_option {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        // 城市操作菜单/建造面板统一使用与单位按钮一致的颜色
        commands.entity(menu_entity).with_children(|builder| {
            // Build 按钮
            builder.spawn((
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                BackgroundColor(CITY_BUTTON_COLOR),
                BorderColor::all(Color::WHITE),
                Text::new("Build"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Button,
                CityActionButton::Build,
            ));

            // Assign 按钮（直接点击地块分配市民）
            builder.spawn((
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                BackgroundColor(CITY_BUTTON_COLOR),
                BorderColor::all(Color::WHITE),
                Text::new("Assign"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Button,
                CityActionButton::AssignCitizens,
            ));

            // Close 按钮
            builder.spawn((
                Node {
                    width: Val::Auto,
                    height: Val::Auto,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                BackgroundColor(CITY_BUTTON_COLOR),
                BorderColor::all(Color::WHITE),
                Text::new("Close"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Button,
                CityActionButton::Close,
            ));
        });
    } else {
        *visibility = Visibility::Hidden;
        *last_selected = None;
    }
}

/// 处理城市操作按钮点击（Build 进入建造屏幕；Assign 由 handle_assign_toggle 单独处理；Close 取消选择）
fn handle_city_action_click(
    button_query: Query<(&Interaction, &CityActionButton), Changed<Interaction>>,
    mut commands: Commands,
    selected_city_query: Query<Entity, With<SelectedCity>>,
    screen: Res<State<ScreenState>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    // 分配模式下只允许通过 Close 退出，不允许进入建造
    let in_assign = matches!(screen.get(), ScreenState::CitizenAssign);

    for (interaction, action) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if in_assign && !matches!(action, CityActionButton::Close) {
            continue;
        }

        match action {
            CityActionButton::Build => {
                // 进入城市建造屏幕（使用 ScreenState 切换）
                next_state.set(ScreenState::CityConstruction);
            }
            CityActionButton::Close => {
                for entity in selected_city_query.iter() {
                    commands.entity(entity).remove::<SelectedCity>();
                }
                // 若当前处于分配屏幕，Close 同时退出分配模式
                if in_assign {
                    next_state.set(ScreenState::WorldMap);
                }
            }
            CityActionButton::AssignCitizens => {
                // 由 handle_assign_toggle 处理
            }
        }
    }
}

/// 处理 Assign（分配市民）按钮点击 - 进入/退出市民分配屏幕（toggle）
///
/// 点击一下进入 CitizenAssign 屏幕（Assign 按钮变绿高亮，类似单位 Move 模式）；
/// 再点击一下返回 WorldMap。退出时按钮颜色由
/// [`reset_assign_button_on_exit`] 恢复。
fn handle_assign_toggle(
    button_query: Query<(&Interaction, &CityActionButton, Entity), Changed<Interaction>>,
    mut commands: Commands,
    screen: Res<State<ScreenState>>,
    mut next_state: ResMut<NextState<ScreenState>>,
    citizen_panel_query: Query<Entity, With<CityCitizenPanel>>,
    mut visibility_query: Query<&mut Visibility>,
) {
    for (interaction, action, button_entity) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !matches!(action, &CityActionButton::AssignCitizens) {
            continue;
        }

        match screen.get() {
            ScreenState::WorldMap => {
                // 隐藏市民面板
                if let Ok(citizen_panel) = citizen_panel_query.single() {
                    if let Ok(mut v) = visibility_query.get_mut(citizen_panel) {
                        *v = Visibility::Hidden;
                    }
                }
                // 进入市民分配屏幕，Assign 按钮变绿激活（同单位 Move 模式）
                commands
                    .entity(button_entity)
                    .insert(CitizenAssignActive)
                    .insert(BackgroundColor(CITY_BUTTON_ACTIVE_COLOR));
                next_state.set(ScreenState::CitizenAssign);
            }
            ScreenState::CitizenAssign => {
                // 再点一次退出分配屏幕（颜色恢复由 OnExit 处理）
                next_state.set(ScreenState::WorldMap);
            }
            _ => {}
        }
    }
}

/// 退出市民分配屏幕时，恢复 Assign 按钮的基础颜色并清除激活标记
/// （覆盖 Esc 退出和再次点击退出两种方式）
fn reset_assign_button_on_exit(
    mut commands: Commands,
    active_query: Query<Entity, (With<CityActionButton>, With<CitizenAssignActive>)>,
) {
    for entity in active_query.iter() {
        commands
            .entity(entity)
            .remove::<CitizenAssignActive>()
            .insert(BackgroundColor(CITY_BUTTON_COLOR));
    }
}

// ============ 市民管理面板 ============

/// 设置城市市民管理面板（居中大面板）
fn setup_city_citizen_panel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(20.0),
            top: Val::Percent(15.0),
            width: Val::Percent(60.0),
            height: Val::Percent(70.0),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(5.0),
            overflow: Overflow::scroll_y(),
            ..Default::default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
        BorderColor::all(Color::WHITE),
        Visibility::Hidden,
        CityCitizenPanel,
    ));
}
