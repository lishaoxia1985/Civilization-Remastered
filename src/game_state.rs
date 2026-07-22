use std::collections::HashMap;

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Nation};

use crate::MapSetting;

/// 单个文明的数据
#[derive(Clone)]
pub struct CivData {
    pub nation: Nation,
    pub gold: i32,
    pub gold_per_turn: i32,
    pub science: i32,
    pub science_per_turn: i32,
    pub culture: i32,
    pub culture_per_turn: i32,
    pub current_research: Option<String>,
    pub researched_technologies: Vec<String>,
    pub accumulated_science: i32,
    pub is_human: bool,
}

impl CivData {
    pub fn new(nation: Nation, is_human: bool) -> Self {
        Self {
            nation,
            gold: if is_human { 500 } else { 500 },
            gold_per_turn: 0,
            science: 0,
            science_per_turn: 3,
            culture: 0,
            culture_per_turn: 0,
            current_research: None,
            researched_technologies: Vec::new(),
            accumulated_science: 0,
            is_human,
        }
    }

    pub fn end_turn(&mut self) {
        self.gold += self.gold_per_turn;
        self.science += self.science_per_turn;
        self.culture += self.culture_per_turn;
    }

    pub fn advance_research(&mut self, map_setting: &MapSetting) -> Option<String> {
        if let Some(ref tech_name) = self.current_research.clone() {
            self.accumulated_science += self.science_per_turn;
            if let Some(technology) = map_setting.0.ruleset.technologies.get(tech_name) {
                if self.accumulated_science >= technology.cost as i32 {
                    self.researched_technologies.push(tech_name.clone());
                    let completed = Some(tech_name.clone());
                    self.current_research = None;
                    self.accumulated_science = 0;
                    return completed;
                }
            }
        }
        None
    }

    pub fn start_research(&mut self, tech_name: String) -> bool {
        if self.researched_technologies.contains(&tech_name) {
            return false;
        }
        self.current_research = Some(tech_name);
        self.accumulated_science = 0;
        true
    }

    /// AI自动选择可研究的科技（找最便宜的未研究科技）
    pub fn ai_choose_research(&mut self, map_setting: &MapSetting) {
        if self.current_research.is_some() {
            return; // 已经在研究中
        }
        let mut available: Vec<(&String, i16)> = map_setting
            .0
            .ruleset
            .technologies
            .iter()
            .filter(|(name, _)| !self.researched_technologies.contains(*name))
            .map(|(name, tech)| (name, tech.cost))
            .collect();
        available.sort_by_key(|(_, cost)| *cost);
        if let Some((tech_name, _)) = available.first() {
            self.start_research(tech_name.to_string());
        }
    }
}

/// 所有文明的状态管理
#[derive(Resource)]
pub struct Civilizations {
    pub civs: HashMap<Nation, CivData>,
    pub turn: u32,
    pub player_nation: Nation,
    pub enemy_nations: Vec<Nation>,
}

impl Civilizations {
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

    pub fn player(&self) -> &CivData {
        &self.civs[&self.player_nation]
    }

    pub fn player_mut(&mut self) -> &mut CivData {
        self.civs.get_mut(&self.player_nation).unwrap()
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

// ============ UI组件 ============

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
    civs: Res<Civilizations>,
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
    map_setting: Res<MapSetting>,
) {
    let player = civs.player();
    turn_text.0 = format!("Turn: {} (Player: {})", civs.turn, player.nation.as_str());
    gold_text.0 = format!("Gold: {} ({:+})", player.gold, player.gold_per_turn);
    science_text.0 = format!("Science: {}/turn", player.science_per_turn);

    if let Some(ref tech_name) = player.current_research {
        if let Some(technology) = map_setting.0.ruleset.technologies.get(tech_name) {
            let progress =
                (player.accumulated_science as f32 / technology.cost as f32 * 100.0) as i32;
            research_text.0 = format!("Researching: {} ({}%)", tech_name, progress.min(100));
        } else {
            research_text.0 = format!("Researching: {}", tech_name);
        }
    } else {
        research_text.0 = "Research: None - Click a tech to start".to_string();
    }
}

// ============ 结束回合按钮 ============

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

fn end_turn_click(_click: On<Pointer<Click>>, mut civs: ResMut<Civilizations>) {
    civs.end_turn();
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum TurnPhase {
    #[default]
    PlayerTurn,
    EnemyTurn,
}
