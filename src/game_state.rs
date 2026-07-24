use std::collections::HashMap;

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Nation, Technology};

use crate::{GameSetting, MapSetting, tech_manage::TechManagerMap};

pub struct CivData {
    pub nation: Nation,
    pub gold: i32,
    pub gold_per_turn: i32,
    pub science_per_turn: i32,
    pub culture: i32,
    pub culture_per_turn: i32,
    pub is_human: bool,
}

impl CivData {
    pub fn new(nation: Nation, is_human: bool) -> Self {
        Self {
            nation,
            gold: if is_human { 500 } else { 500 },
            gold_per_turn: 0,
            science_per_turn: 3,
            culture: 0,
            culture_per_turn: 0,
            is_human,
        }
    }

    pub fn end_turn(&mut self) {
        self.gold += self.gold_per_turn;
        self.culture += self.culture_per_turn;
    }

    pub fn start_research(&self, tech: Technology, tech_manager_map: &mut TechManagerMap) -> bool {
        tech_manager_map.0.entry(self.nation).and_modify(|tm| {
            // TODO: We have not implemented the queue yet.
            // Now it always has only current researching tech in the queue.
            tm.techs_to_research.clear();
            tm.techs_to_research.push(tech)
        });
        true
    }

    /// AI automatically selects technology to research (find cheapest unresearched technology)
    pub fn ai_choose_research(
        &mut self,
        map_setting: &MapSetting,
        tech_manager_map: &mut TechManagerMap,
    ) {
        let tech_manager = &tech_manager_map.0[&self.nation];

        if tech_manager.current_researching_technology().is_some() {
            return; // Already researching
        }
        let mut available: Vec<(Technology, i32)> = map_setting
            .0
            .ruleset
            .technologies
            .iter()
            .filter(|(tech, _)| tech_manager.can_be_researched(*tech, map_setting))
            .map(|(tech, info)| (tech, info.cost))
            .collect();
        available.sort_by_key(|(_, cost)| *cost);
        if let Some(&(tech, _)) = available.first() {
            self.start_research(tech, tech_manager_map);
        }
    }
}

/// Manager states for all civilizations
#[derive(Resource)]
pub struct CivilizationStates {
    pub civs: HashMap<Nation, CivData>,
    pub turn: u32,
    pub player_nation: Nation,
    pub enemy_nations: Vec<Nation>,
}

impl CivilizationStates {
    pub fn new(player: Nation, enemies: Vec<Nation>) -> Self {
        let mut civs = HashMap::new();
        civs.insert(player, CivData::new(player, true));
        for &enemy in &enemies {
            civs.insert(enemy, CivData::new(enemy, false));
        }
        Self {
            civs,
            turn: 1,
            player_nation: player,
            enemy_nations: enemies,
        }
    }

    pub fn player_data(&self) -> &CivData {
        &self.civs[&self.player_nation]
    }

    pub fn is_enemy(&self, nation: Nation) -> bool {
        self.enemy_nations.contains(&nation)
    }

    pub fn end_turn(&mut self) {
        self.turn += 1;
        for civ in self.civs.values_mut() {
            civ.end_turn();
        }
    }
}

#[derive(Component)]
pub struct TurnCounterText;

#[derive(Component)]
pub struct GoldText;

#[derive(Component)]
pub struct ScienceText;

#[derive(Component)]
pub struct ResearchStatusText;

pub fn setup_game_state_ui(mut commands: Commands) {
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

pub fn update_game_state_ui(
    civ_states: Res<CivilizationStates>,
    tech_manager_map: Res<TechManagerMap>,
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
    game_settings: Res<GameSetting>,
    map_setting: Res<MapSetting>,
) {
    let player_nation = civ_states.player_nation;
    let tech_manager = &tech_manager_map.0[&player_nation];
    let player_data = civ_states.player_data();
    turn_text.0 = format!(
        "Turn: {} (Player: {})",
        civ_states.turn,
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
            tech_manager.cost_of_tech(tech, player_data, &game_settings, &map_setting);
        let progress = (research_progress as f32 / cost_of_tech as f32 * 100.0) as i32;
        research_text.0 = format!("Researching: {} ({}%)", tech.as_str(), progress.min(100));
    } else {
        research_text.0 = "Research: None - Click a tech to start".to_string();
    }
}

// ============ End Turn Button ============

#[derive(Component)]
pub struct EndTurnButton;

pub fn setup_end_turn_button(mut commands: Commands) {
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
            Pickable::default(),
        ))
        .observe(end_turn_click);
}

fn end_turn_click(
    _click: On<Pointer<Click>>,
    mut civ_states: ResMut<CivilizationStates>,
    mut tech_manager_map: ResMut<TechManagerMap>,
    game_settings: Res<GameSetting>,
    map_setting: Res<MapSetting>,
) {
    let player_nation = civ_states.player_nation;
    if tech_manager_map.0[&player_nation]
        .current_researching_technology()
        .is_none()
    {
        println!("Ccurrently no technology being researched. Please choose one to research.");
        return;
    }
    civ_states.end_turn();
    // TODO: the turn is not true because turn has +1 when civ_states.end_turn() is called,
    // the turn +1 should be at last when the turn is ended
    tech_manager_map
        .0
        .iter_mut()
        .for_each(|(nation, tech_manager)| {
            let civ = &civ_states.civs[nation];
            tech_manager.end_turn(
                civ.science_per_turn,
                civ,
                civ_states.turn,
                &game_settings,
                &map_setting,
            );
        });
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum TurnPhase {
    #[default]
    PlayerTurn,
    EnemyTurn,
}
