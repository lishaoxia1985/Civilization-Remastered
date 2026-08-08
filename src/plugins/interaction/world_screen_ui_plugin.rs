//! 游戏状态插件
//!
//! 管理游戏状态 UI、文明状态、结束回合等功能。

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::EnumStr;

use crate::{
    AppState, GoldIncome, GoldPerTurn, NationComponent, Player, SciencePerTurn, ScreenState,
    TurnManager, TurnPhase, TurnState,
    plugins::tech::{ResearchingTech, TechCostManager, TechProgressManager},
};

/// 回合计数器文本
#[derive(Component)]
struct TurnCounterText;

/// 金币文本
#[derive(Component)]
struct GoldText;

/// 科技点数文本
#[derive(Component)]
struct ScienceText;

/// 研究状态文本
#[derive(Component)]
struct ResearchStatusText;

/// 游戏状态插件
pub struct WorldScreenUiPlugin;

impl Plugin for WorldScreenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (
                setup_game_state_ui,
                setup_end_turn_button,
                setup_tech_tree_button,
            ),
        )
        .add_systems(
            Update,
            (
                update_turn_counter,
                update_gold_display,
                update_science_display,
                update_research_status,
                update_end_turn_button,
                update_tech_tree_button,
            )
                .run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

/// 结束回合按钮
#[derive(Component)]
pub struct EndTurnButton;

#[derive(Component)]
pub struct TechTreeButton;

/// 设置游戏状态 UI
fn setup_game_state_ui(mut commands: Commands) {
    let default_font = TextFont {
        font_size: FontSize::Px(16.0),
        ..Default::default()
    };
    let small_font = TextFont {
        font_size: FontSize::Px(14.0),
        ..Default::default()
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(50.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(5.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Turn: 0"),
                default_font.clone(),
                TextColor(Color::WHITE),
                TurnCounterText,
            ));
            parent.spawn((
                Text::new("Gold: 0"),
                default_font.clone(),
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                GoldText,
            ));
            parent.spawn((
                Text::new("Science: 0/turn"),
                default_font.clone(),
                TextColor(Color::srgb(0.0, 0.5, 1.0)),
                ScienceText,
            ));
            parent.spawn((
                Text::new("Research: None"),
                small_font.clone(),
                TextColor(Color::srgb(0.0, 0.8, 0.8)),
                ResearchStatusText,
            ));
        });
}

/// 更新回合计数器
fn update_turn_counter(
    mut text: Single<&mut Text, With<TurnCounterText>>,
    turn_manager: Res<TurnManager>,
    player_query: Query<&NationComponent, With<Player>>,
) {
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok(nation_component) = player_query.get(current_entity) else {
        return;
    };

    text.0 = format!(
        "Turn: {} (Player: {})",
        turn_manager.turn_number,
        nation_component.0.as_str()
    );
}

/// 更新金币显示
/// TODO: 需通过查询获得每回合金
fn update_gold_display(
    mut text: Single<&mut Text, With<GoldText>>,
    turn_manager: Res<TurnManager>,
    gold_query: Query<(&GoldIncome, &GoldPerTurn), With<Player>>,
) {
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();
    let Ok((gold_income, gold_per_turn)) = gold_query.get(current_entity) else {
        return;
    };

    text.0 = format!("Gold: {} ({:+})", gold_income.0, gold_per_turn.0);
}

/// 更新科技点数显示
fn update_science_display(
    mut text: Single<&mut Text, With<ScienceText>>,
    turn_manager: Res<TurnManager>,
    science_per_turn_query: Query<&SciencePerTurn, With<Player>>,
) {
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok(science_per_turn) = science_per_turn_query.get(current_entity) else {
        return;
    };

    text.0 = format!("Science: {}/turn", science_per_turn.0);
}

/// 更新研究状态
fn update_research_status(
    mut text: Single<&mut Text, With<ResearchStatusText>>,
    turn_manager: Res<TurnManager>,
    player_query: Query<(&ResearchingTech, &TechProgressManager, &TechCostManager), With<Player>>,
) {
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok((researching_tech, tech_progress_manager, tech_cost_manager)) =
        player_query.get(current_entity)
    else {
        return;
    };

    if let Some(tech) = researching_tech.0 {
        let research_progress = tech_progress_manager.0.get(&tech).copied().unwrap_or(0);
        let cost_of_tech = tech_cost_manager.0.get(&tech).copied().unwrap_or(0);
        let progress = (research_progress as f32 / cost_of_tech as f32 * 100.0) as i32;
        text.0 = format!("Researching: {} ({}%)", tech.as_str(), progress.min(100));
    } else {
        text.0 = "Research: None - Click a tech to start".to_string();
    }
}

// ============ 结束回合按钮 ============

/// 设置结束回合按钮
fn setup_end_turn_button(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(50.0),
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            Visibility::Hidden,
            BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
            BorderColor::all(Color::WHITE),
            Text::new("End Turn"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
            EndTurnButton,
        ))
        .observe(end_turn_click);
}

/// 结束回合点击处理,只针对Player
fn end_turn_click(
    _click: On<Pointer<Click>>,
    player_query: Query<(Entity, &ResearchingTech), With<Player>>,
    turn_manager: Res<TurnManager>,
    mut next_turn_state: ResMut<NextState<TurnState>>,
) {
    // 获取当前回合的nation实体
    let current_entity = turn_manager.current_nation_entity();

    let Ok((entity, researching_tech)) = player_query.get(current_entity) else {
        return;
    };

    if researching_tech.0.is_none() {
        println!("当前没有正在研究的科技，请选择一项科技进行研究。");
        return;
    }
    next_turn_state.set(TurnState::End);
    info!("Ending player turn for {}", entity);
}

fn update_end_turn_button(
    visibility: Single<&mut Visibility, With<EndTurnButton>>,
    turn_phase: Res<State<TurnPhase>>,
) {
    let mut visibility = visibility.into_inner();
    if turn_phase.get() == &TurnPhase::Player {
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}

/// 设置打开科技树的按钮
fn setup_tech_tree_button(mut commands: Commands) {
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
            Visibility::Hidden,
            BackgroundColor(Color::BLACK),
            BorderColor::all(Color::WHITE),
            Text::new("Open Tech Tree"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..Default::default()
            },
            TextColor(Color::WHITE),
            TechTreeButton,
        ))
        .observe(open_tech_tree);
}

/// 打开科技树
fn open_tech_tree(drag: On<Pointer<Click>>, mut next_state: ResMut<NextState<ScreenState>>) {
    if matches!(drag.button, PointerButton::Primary) {
        next_state.set(ScreenState::TechTree);
    }
}

fn update_tech_tree_button(
    visibility: Single<&mut Visibility, With<TechTreeButton>>,
    turn_phase: Res<State<TurnPhase>>,
) {
    let mut visibility = visibility.into_inner();
    if turn_phase.get() == &TurnPhase::Player {
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}
