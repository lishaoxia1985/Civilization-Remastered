//! 游戏状态插件
//!
//! 管理游戏状态 UI、文明状态、结束回合等功能。

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::EnumStr;

use crate::{
    AppState, ScreenState,
    components::{EndTurnButton, GoldText, ResearchStatusText, ScienceText, TurnCounterText},
    resources::{CivilizationManager, GameSettings, MapParametersRes, TechManagerRegistry},
};

/// 游戏状态插件
pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::GameStart),
            (setup_game_state_ui, setup_end_turn_button),
        )
        .add_systems(
            Update,
            update_game_state_ui.run_if(in_state(ScreenState::WorldMap)),
        );
    }
}

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
    civ_manager: Res<CivilizationManager>,
    tech_registry: Res<TechManagerRegistry>,
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
) {
    let player_nation = civ_manager.player_nation;
    let tech_manager = &tech_registry.0[&player_nation];
    let player_data = civ_manager.player_data();
    turn_text.0 = format!(
        "Turn: {} (Player: {})",
        civ_manager.turn,
        player_data.nation.as_str()
    );
    gold_text.0 = format!(
        "Gold: {} ({:+})",
        player_data.gold, player_data.gold_per_turn
    );
    science_text.0 = format!("Science: {}/turn", player_data.science_per_turn);

    if let Some(tech) = tech_manager.current_researching_technology() {
        let research_progress = tech_manager.research_progress(tech);
        let cost_of_tech =
            tech_manager.cost_of_tech(tech, player_data, &game_settings, &map_params);
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

/// 结束回合点击处理
fn end_turn_click(
    _click: On<Pointer<Click>>,
    mut civ_manager: ResMut<CivilizationManager>,
    mut tech_registry: ResMut<TechManagerRegistry>,
    game_settings: Res<GameSettings>,
    map_params: Res<MapParametersRes>,
) {
    let player_nation = civ_manager.player_nation;
    if tech_registry.0[&player_nation]
        .current_researching_technology()
        .is_none()
    {
        println!("当前没有正在研究的科技，请选择一项科技进行研究。");
        return;
    }
    civ_manager.end_turn();
    tech_registry
        .0
        .iter_mut()
        .for_each(|(nation, tech_manager)| {
            let civ = &civ_manager.civs[nation];
            tech_manager.end_turn(
                civ.science_per_turn,
                civ,
                civ_manager.turn,
                &game_settings,
                &map_params,
            );
        });
}
