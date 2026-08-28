//! 城市交互插件
//!
//! 管理玩家与城市的交互操作，包括：
//! - 城市选择（点击城市地块选择城市）
//! - 城市信息面板（显示城市名称、人口、收益等）
//! - 城市操作菜单（建造建筑、生产单位、管理市民）
//! - 城市建造界面（选择要建造的建筑/单位）
//! - 市民工作分配界面（选择市民工作的地块）

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Technology};

use crate::{
    AppState, NationComponent, Player, ScreenState, TurnManager,
    components::{
        City, CityProduction, CityYields, Owner, SelectedCity, SelectedUnit, WorldTile,
    },
    plugins::tech::{TechStateManager, is_researched},
    resources::MapParametersRes,
};

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

// ============ 城市建造界面 ============

/// 城市建造面板
#[derive(Component)]
pub struct CityBuildPanel;

/// 建造面板中的标签页按钮
#[derive(Component)]
pub enum BuildTabButton {
    /// 建筑标签页
    Buildings,
    /// 单位标签页
    Units,
}

/// 建造列表中的可建造项按钮
#[derive(Component)]
pub struct BuildItemButton {
    /// 项目名称
    pub name: String,
    /// 生产方式
    pub production: CityProduction,
}

// ============ 市民工作界面 ============

/// 城市市民管理面板
#[derive(Component)]
pub struct CityCitizenPanel;

/// 当前建造面板显示的标签页（true=建筑, false=单位）
#[derive(Default)]
struct BuildTab(bool);

/// 城市交互插件
pub struct CityInteractionPlugin;

impl Plugin for CityInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (
                setup_city_info_panel,
                setup_city_action_menu,
                setup_city_build_panel,
                setup_city_citizen_panel,
            ),
        )
        .add_systems(
            Update,
            (
                handle_city_selection,
                update_city_info_panel,
                show_city_action_menu,
                handle_city_action_click,
                update_city_build_panel,
                handle_build_item_click,
            )
                .run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

// ============ 城市选择系统 ============

/// 处理城市选择 - 点击城市地块选择城市
fn handle_city_selection(
    mut click_events: MessageReader<Pointer<Click>>,
    mut commands: Commands,
    city_tile_query: Query<(Entity, &City, &Owner), With<WorldTile>>,
    children_query: Query<Option<&Children>, With<WorldTile>>,
    player_query: Query<&NationComponent, With<Player>>,
    selected_city_query: Query<Entity, With<SelectedCity>>,
    selected_unit_query: Query<Entity, With<SelectedUnit>>,
    turn_manager: Res<TurnManager>,
) {
    // 仅在玩家回合处理选择
    let current_entity = turn_manager.current_nation_entity();
    let Ok(nation_component) = player_query.get(current_entity) else {
        return;
    };
    let player_nation = nation_component.0;

    for click in click_events.read() {
        let click_target = click.event_target();

        // 检查点击的是否是某个城市地块（或该地块的子节点）
        let clicked_city = city_tile_query.iter().find(|(city_entity, _, _)| {
            if *city_entity == click_target {
                return true;
            }
            if let Ok(Some(children)) = children_query.get(*city_entity) {
                return children.contains(&click_target);
            }
            false
        });

        if let Some((city_tile_entity, city, owner)) = clicked_city {
            if owner.0 != player_nation {
                continue;
            }

            for entity in selected_unit_query.iter() {
                commands.entity(entity).remove::<SelectedUnit>();
            }
            for entity in selected_city_query.iter() {
                commands.entity(entity).remove::<SelectedCity>();
            }
            commands.entity(city_tile_entity).insert(SelectedCity);
            info!(
                "City selected: {} (Pop: {}, Food: {}/{})",
                city.name, city.population, city.food, city.food_needed
            );
            continue;
        }

        if selected_city_query.single().is_ok() {
            if children_query.get(click_target).is_ok() {
                for entity in selected_city_query.iter() {
                    commands.entity(entity).remove::<SelectedCity>();
                }
            }
        }
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
                BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
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
                BackgroundColor(Color::srgb(0.6, 0.5, 0.2)),
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
                BackgroundColor(Color::srgb(0.5, 0.2, 0.2)),
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

/// 处理城市操作按钮点击
fn handle_city_action_click(
    button_query: Query<(&Interaction, &CityActionButton), Changed<Interaction>>,
    mut commands: Commands,
    selected_city_query: Query<Entity, With<SelectedCity>>,
    build_panel_query: Query<Entity, With<CityBuildPanel>>,
    citizen_panel_query: Query<Entity, With<CityCitizenPanel>>,
    mut visibility_query: Query<&mut Visibility>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    for (interaction, action) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // 获取建造面板和市民面板的可见性
        let mut show_build = false;
        let mut show_citizen = false;

        match action {
            CityActionButton::Build => {
                show_build = true;
            }
            CityActionButton::AssignCitizens => {
                // 隐藏其他面板
                if let Ok(build_panel) = build_panel_query.single() {
                    if let Ok(mut v) = visibility_query.get_mut(build_panel) {
                        *v = Visibility::Hidden;
                    }
                }
                if let Ok(citizen_panel) = citizen_panel_query.single() {
                    if let Ok(mut v) = visibility_query.get_mut(citizen_panel) {
                        *v = Visibility::Hidden;
                    }
                }
                // 进入市民分配屏幕（由 CitizenAssignScreenPlugin 处理显示与点击）
                next_state.set(ScreenState::CitizenAssign);
            }
            CityActionButton::Close => {
                for entity in selected_city_query.iter() {
                    commands.entity(entity).remove::<SelectedCity>();
                }
            }
        }

        // 更新面板可见性
        if let Ok(build_panel) = build_panel_query.single() {
            if let Ok(mut v) = visibility_query.get_mut(build_panel) {
                *v = if show_build {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
        if let Ok(citizen_panel) = citizen_panel_query.single() {
            if let Ok(mut v) = visibility_query.get_mut(citizen_panel) {
                *v = if show_citizen {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

// ============ 城市建造面板 ============

/// 设置城市建造面板（居中大面板）
fn setup_city_build_panel(mut commands: Commands) {
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
        CityBuildPanel,
    ));
}

/// 更新城市建造面板（显示建筑/单位列表）
///
/// 根据科技解锁状态过滤可建造列表：
/// - 建筑：需要 `required_tech` 已研究才能建造
/// - 单位：需要 `required_tech` 已研究才能建造，`obsolete_tech` 已研究则过时不可建造
fn update_city_build_panel(
    build_panel: Single<(Entity, &mut Visibility, Option<&mut Children>), With<CityBuildPanel>>,
    selected_city_query: Query<(&City, &CityYields), With<SelectedCity>>,
    map_params: Res<MapParametersRes>,
    tech_state_query: Query<&TechStateManager, With<NationComponent>>,
    turn_manager: Res<TurnManager>,
    mut commands: Commands,
    mut show_buildings: Local<bool>,
    mut last_selected: Local<Option<Entity>>,
) {
    let (panel_entity, mut visibility, children_option) = build_panel.into_inner();

    let Ok((city, yields)) = selected_city_query.single() else {
        *visibility = Visibility::Hidden;
        *last_selected = None;
        return;
    };

    if *visibility == Visibility::Hidden {
        return;
    }

    let ruleset = &map_params.0.ruleset;

    if *last_selected == Some(panel_entity) {
        return;
    }
    *last_selected = Some(panel_entity);

    if let Some(children) = children_option {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    // 获取当前回合文明的科技状态
    let current_nation_entity = turn_manager.current_nation_entity();
    let tech_state_manager = tech_state_query.get(current_nation_entity).ok();

    let production_per_turn = yields.production.max(1);

    commands.entity(panel_entity).with_children(|builder| {
        // 标题
        builder.spawn((
            Text::new(format!("{} - Production", city.name)),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
        ));

        // 简单标签行（文字显示当前标签）
        let tab_label = if *show_buildings {
            "Showing: Buildings"
        } else {
            "Showing: Units"
        };
        builder.spawn((
            Text::new(tab_label),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..Default::default()
            },
            TextColor(Color::srgb(0.5, 1.0, 0.5)),
        ));

        // 切换按钮
        builder.spawn((
            Node {
                width: Val::Auto,
                height: Val::Auto,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(3.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.3, 0.4, 0.5)),
            BorderColor::all(Color::WHITE),
            Text::new("Switch Tab"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
            Button,
            BuildTabButton::Buildings, // 复用作为切换按钮
        ));

        // 列表
        if *show_buildings {
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
                        .as_ref()
                        .map(|manager| is_researched(required_tech, manager))
                        .unwrap_or(false);
                    if !tech_unlocked {
                        continue;
                    }
                }

                let turns = turns_estimate(cost, production_per_turn);
                let label = format!("{} - {} ({} turns)", building.as_str(), cost, turns);
                spawn_build_item_button(builder, label, CityProduction::Building(building));
            }
        } else {
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
                        .as_ref()
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
                        .as_ref()
                        .map(|manager| is_researched(obsolete_tech, manager))
                        .unwrap_or(false);
                    if is_obsolete {
                        continue;
                    }
                }

                let turns = turns_estimate(cost, production_per_turn);
                let label = format!("{} - {} ({} turns)", unit.as_str(), cost, turns);
                spawn_build_item_button(builder, label, CityProduction::Unit(unit));
            }
        }
    });

    // 检查切换按钮点击（通过命令）
    // 注意：这里通过 button_query 在下一帧处理
    let _ = show_buildings;
}

/// 生成建造项目按钮
fn spawn_build_item_button(
    builder: &mut ChildSpawnerCommands,
    label: String,
    production: CityProduction,
) {
    builder.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            border: UiRect::all(Val::Px(1.0)),
            padding: UiRect::all(Val::Px(4.0)),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.2, 0.3, 0.4)),
        BorderColor::all(Color::WHITE),
        Text::new(label.as_str()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..Default::default()
        },
        TextColor(Color::WHITE),
        Button,
        BuildItemButton {
            name: label,
            production,
        },
    ));
}

/// 处理建造项目点击
fn handle_build_item_click(
    button_query: Query<(&Interaction, &BuildItemButton)>,
    mut selected_city_query: Query<(Entity, &mut City), (With<SelectedCity>, With<City>)>,
) {
    for (interaction, item) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        // 修改城市的 current_production 字段
        for (entity, mut city) in selected_city_query.iter_mut() {
            city.current_production = Some(item.production.clone());

            info!("City {} started producing: {}", entity, item.name);
        }
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

