//! 游戏状态插件
//!
//! 管理游戏状态 UI、文明状态、结束回合等功能。

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::EnumStr;

use crate::{
    AppState, NationComponent, Player, SciencePerTurn, ScreenState, TurnEndMessage, TurnManager,
    TurnPhase,
    resources::{GameSettings, MapParametersRes, TechManager},
};

/// 游戏状态插件
pub struct UiPlugin;

impl Plugin for UiPlugin {
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
                update_game_state_ui,
                update_end_turn_button,
                update_tech_tree_button,
            )
                .run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

/// 回合计数器文本
#[derive(Component)]
pub struct TurnCounterText;

/// 金币文本
#[derive(Component)]
pub struct GoldText;

/// 科技点数文本
#[derive(Component)]
pub struct ScienceText;

/// 研究状态文本
#[derive(Component)]
pub struct ResearchStatusText;

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
                Text::new("Turn: 1"),
                default_font.clone(),
                TextColor(Color::WHITE),
                TurnCounterText,
            ));
            parent.spawn((
                Text::new("Gold: 500"),
                default_font.clone(),
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                GoldText,
            ));
            parent.spawn((
                Text::new("Science: 3/turn"),
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

/// 更新游戏状态 UI
fn update_game_state_ui(
    mut turn_text: Single<
        &mut Text,
        (
            With<TurnCounterText>,
            Without<GoldText>,
            Without<ScienceText>,
            Without<ResearchStatusText>,
        ),
    >,
    mut gold_text: Single<
        &mut Text,
        (
            With<GoldText>,
            Without<TurnCounterText>,
            Without<ResearchStatusText>,
            Without<ScienceText>,
        ),
    >,
    mut science_text: Single<
        &mut Text,
        (
            With<ScienceText>,
            Without<TurnCounterText>,
            Without<GoldText>,
            Without<ResearchStatusText>,
        ),
    >,
    mut research_text: Single<
        &mut Text,
        (
            With<ResearchStatusText>,
            Without<TurnCounterText>,
            Without<GoldText>,
            Without<ScienceText>,
        ),
    >,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
    turn_manager: Res<TurnManager>,
    query_player: Single<(&NationComponent, &mut TechManager, &SciencePerTurn), With<Player>>,
) {
    let (nation_component, tech_manager, science_per_turn) = query_player.into_inner();
    turn_text.0 = format!(
        "Turn: {} (Player: {})",
        turn_manager.turn_number,
        nation_component.0.as_str()
    );
    gold_text.0 = format!("Gold: {} ({:+})", 3, 3);
    science_text.0 = format!("Science: {}/turn", science_per_turn.0);

    if let Some(tech) = tech_manager.current_researching_technology() {
        let research_progress = tech_manager.research_progress(tech);
        let cost_of_tech = tech_manager.cost_of_tech(tech, true, &game_settings, &map_params);
        let progress = (research_progress as f32 / cost_of_tech as f32 * 100.0) as i32;
        research_text.0 = format!("Researching: {} ({}%)", tech.as_str(), progress.min(100));
    } else {
        research_text.0 = "Research: None - Click a tech to start".to_string();
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
/// TODO: 当前按钮全程有效，实际应当只针对Player有效
fn end_turn_click(
    _click: On<Pointer<Click>>,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
    query_player: Single<(Entity, &mut TechManager, &SciencePerTurn), With<Player>>,
    turn_manager: Res<TurnManager>,
    mut turn_end_messages: MessageWriter<TurnEndMessage>,
) {
    let (entity, mut tech_manager, science_per_turn) = query_player.into_inner();
    if turn_manager.turn_queue[turn_manager.current_index] != entity {
        return;
    }
    if tech_manager.current_researching_technology().is_none() {
        println!("当前没有正在研究的科技，请选择一项科技进行研究。");
        return;
    }
    // TODO: Should edit `turn` and `is_player`
    turn_end_messages.write(TurnEndMessage { entity });

    tech_manager.end_turn(
        science_per_turn.0,
        true,
        turn_manager.turn_number,
        &game_settings,
        &map_params,
    );
}

fn update_end_turn_button(
    visibility: Single<&mut Visibility, With<EndTurnButton>>,
    turn_phase: Res<State<TurnPhase>>,
) {
    let mut visibility = visibility.into_inner();
    if turn_phase.get() == &TurnPhase::PlayTurn {
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
    if turn_phase.get() == &TurnPhase::PlayTurn {
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}
