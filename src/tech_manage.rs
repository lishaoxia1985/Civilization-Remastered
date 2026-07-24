use crate::{
    GameSetting, MapSetting,
    game_state::{CivData, CivilizationStates},
};
use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{EnumStr, Era, Nation, Technology};
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
};

#[derive(Resource)]
pub struct TechManagerMap(pub HashMap<Nation, TechManager>);

impl TechManagerMap {
    pub fn new(civilization_states: &CivilizationStates, era: Era) -> Self {
        let tech_manager_map: HashMap<_, _> = civilization_states
            .civs
            .keys()
            .map(|&nation| (nation.clone(), TechManager::new(era)))
            .collect();
        Self(tech_manager_map)
    }
}

/// Technology Manager
pub struct TechManager {
    pub era: Era,
    pub researched_technologies: Vec<Technology>,
    pub tech_uniques: HashMap<String, Vec<String>>,

    // Unit movement related boolean flags
    pub units_can_embark: bool,
    pub embarked_units_can_enter_ocean: bool,
    pub all_units_can_enter_ocean: bool,
    pub specific_units_can_enter_ocean: bool,

    // Movement speed related
    pub movement_speed_on_roads: f32,
    pub roads_connect_across_rivers: bool,
    pub all_techs_are_researched: bool,

    /// Technology count
    pub free_techs: i32,
    pub repeating_techs_researched: i32,

    /// Science values from last 8 turns
    pub science_of_last_8_turns: [i32; 8],
    pub science_from_research_agreements: i32,

    /// Set of researched technology
    pub techs_researched: HashSet<Technology>,

    /// Queue of technologies to research.
    /// Current researching technology is always at the first of the queue.
    /// TODO: we have not implemented the queue yet. Now it always contains only current researching technology.
    pub techs_to_research: Vec<Technology>,

    pub overflow_science: i32,

    /// Technologies in progress, only the tech which is being worked but not yet complete is stored here.
    /// The value is the amount of science spent on the tech, it can't be `0`.
    pub techs_in_progress: HashMap<Technology, i32>,

    // Gold to science conversion ratio
    pub gold_percent_converted_to_science: f32,
}

impl TechManager {
    /// Create a new technology manager
    pub fn new(era: Era) -> Self {
        Self {
            era,
            researched_technologies: Vec::new(),
            tech_uniques: HashMap::new(),
            units_can_embark: false,
            embarked_units_can_enter_ocean: false,
            all_units_can_enter_ocean: false,
            specific_units_can_enter_ocean: false,
            movement_speed_on_roads: 1.0,
            roads_connect_across_rivers: false,
            all_techs_are_researched: false,
            free_techs: 0,
            repeating_techs_researched: 0,
            science_of_last_8_turns: [0; 8],
            science_from_research_agreements: 0,
            techs_researched: HashSet::new(),
            techs_to_research: Vec::new(),
            overflow_science: 0,
            techs_in_progress: HashMap::new(),
            gold_percent_converted_to_science: 0.6,
        }
    }

    /// Get the number of researched technologies
    pub fn get_number_of_techs_researched(&self) -> i32 {
        self.techs_researched.len() as i32
    }

    /// Get overflow science value
    pub fn get_overflow_science(&self) -> i32 {
        self.overflow_science
    }

    /// Calculate the cost of a technology
    pub fn cost_of_tech(
        &self,
        tech: Technology,
        civ_data: &CivData,
        game_setting: &GameSetting,
        map_setting: &MapSetting,
    ) -> i32 {
        let ruleset = &map_setting.0.ruleset;
        let tech_info = &ruleset.technologies[tech];
        let difficulty_info = &ruleset.difficulties[game_setting.difficulty];
        let speed_info = &ruleset.speeds[game_setting.speed];

        let mut tech_cost = tech_info.cost as f32;

        // Difficulty Modifier when civ is player
        if civ_data.is_human {
            tech_cost *= difficulty_info.research_cost_modifier;
        }

        // Speed Modifier
        tech_cost *= speed_info.science_cost_modifier;

        let science_modifier = self.get_science_modifier(tech, civ_data, map_setting);
        tech_cost /= science_modifier;

        // Map size Modifier
        // TODO: Need to get map size adjustment value
        // let map_size_predef = map_parameters.map_size.get_predefined_or_next_smaller();
        // tech_cost *= map_size_predef.tech_cost_multiplier;

        // Number of cities Modifier
        // TODO: Need to get number of cities adjustment
        // let city_modifier = (cities_count - 1) * map_size_predef.tech_cost_per_city_modifier;

        // Unique ability Modifier
        // TODO: Need to implement unique ability system

        tech_cost as i32
    }

    /// Get the modifier for a given technology according to the remaining civilization's research
    fn get_science_modifier(
        &self,
        tech: Technology,
        civ_data: &CivData,
        map_setting: &MapSetting,
    ) -> f32 {
        // TODO: need to implement get all remaining civilizations and their research status
        // let number_of_civs_researched_this_tech = 0;
        // let number_of_civs_remaining = 0;
        // 1 + number_of_civs_researched_this_tech / number_of_civs_remaining as f32 * 0.3
        1.0
    }

    /// Get current researching technology.
    pub fn current_researching_technology(&self) -> Option<Technology> {
        self.techs_to_research.first().map(|&s| s)
    }

    /// Get research progress of a technology, returns 0 if not researched.
    /// That return value is the science points we have spent on it.
    pub fn research_progress(&self, tech: Technology) -> i32 {
        self.techs_in_progress.get(&tech).copied().unwrap_or(0)
    }

    /// Calculates the remaining science points required to complete this technology.
    pub fn remaining_science_to_tech(
        &self,
        tech: Technology,
        civ_data: &CivData,
        game_setting: &GameSetting,
        map_setting: &MapSetting,
    ) -> i32 {
        let spare_science = if self.can_be_researched(tech, map_setting) {
            self.overflow_science
        } else {
            0
        };

        let cost = self.cost_of_tech(tech, civ_data, game_setting, map_setting);
        let researched = self.research_progress(tech);

        cost - researched - spare_science
    }

    /// Calculates the number of turns required to complete this technology.
    pub fn turns_to_tech(
        &self,
        tech: Technology,
        science_per_turn: i32,
        civ_data: &CivData,
        game_setting: &GameSetting,
        map_setting: &MapSetting,
    ) -> String {
        if self.is_researched(tech) && tech != Technology::FutureTech {
            return "".to_string();
        }

        // Calculate remaining science cost for the technology
        let remaining_cost =
            self.remaining_science_to_tech(tech, civ_data, game_setting, map_setting) as f32;

        // Return empty string if technology is already completed (no remaining cost)
        if remaining_cost <= 0.0 {
            return "".to_string();
        }

        // Handle case where science production is insufficient (infinite turns)
        if science_per_turn <= 0 {
            return "∞ turns".to_string();
        }

        // Calculate required turns (ceiling of division) and ensure minimum of 1 turn
        let turns = (remaining_cost / science_per_turn as f32).ceil() as i32;
        format!("{} turns", turns.max(1))
    }

    /// Check if technology has been researched
    pub fn is_researched(&self, tech: Technology) -> bool {
        self.techs_researched.contains(&tech)
    }

    /// Check if technology can be researched
    pub fn can_be_researched(&self, tech: Technology, map_setting: &MapSetting) -> bool {
        let ruleset = &map_setting.0.ruleset;
        let tech_info = &ruleset.technologies[tech];

        let is_continually_researchable = tech_info
            .uniques
            .contains(&"Can be continually researched".to_string());

        // TODO: Check if technology is not researchable
        // if self.is_unresearchable(tech, map_setting) {
        //     return false;
        // }

        // If already researched and not repeatable, cannot research again
        if self.is_researched(tech) && !is_continually_researchable {
            return false;
        }

        // Check if all prerequisite technologies have been researched
        tech_info
            .prerequisites
            .iter()
            .all(|prereq| self.is_researched(Technology::from_str(prereq)))
    }

    /// Check if technology is not researchable
    fn is_unresearchable(&self, tech: &Technology, map_setting: &MapSetting) -> bool {
        // TODO: Need to implement unique ability check
        // if (tech.getMatchingUniques(UniqueType.OnlyAvailable, GameContext.IgnoreConditionals).any { !it.conditionalsApply(civInfo.state) })
        //     return true
        // if (tech.hasUnique(UniqueType.Unavailable, civInfo.state)) return true
        false
    }

    /// Check if all technologies have been researched
    pub fn all_techs_are_researched(&self) -> bool {
        self.all_techs_are_researched
    }

    /// Add science points
    pub fn add_science(
        &mut self,
        science: i32,
        current_tech: Technology,
        civ_data: &CivData,
        game_setting: &GameSetting,
        map_setting: &MapSetting,
    ) {
        let cost = self.cost_of_tech(current_tech, civ_data, game_setting, map_setting);
        let current = self.techs_in_progress.entry(current_tech).or_insert(0);
        *current += science;

        if *current >= cost {
            // Complete technology research
            let extra_science = *current - cost;
            self.overflow_science +=
                self.limit_overflow_science(extra_science, current_tech, civ_data, map_setting);
            self.add_technology(current_tech);
        }
    }

    /// Limits overflow science points to prevent excessive carryover to next technology.
    ///
    /// Ensures overflow does not exceed:
    /// 1. The current technology's base cost, OR
    /// 2. Five turns' worth of science production
    fn limit_overflow_science(
        &self,
        overflow: i32,
        current_tech: Technology,
        civ_data: &CivData,
        map_setting: &MapSetting,
    ) -> i32 {
        let ruleset = &map_setting.0.ruleset;
        let tech_cost = ruleset.technologies[current_tech].cost;
        // Limit overflow science value
        min(overflow, max(civ_data.science_per_turn * 5, tech_cost))
    }

    /// Add technology when comleting a technology research
    pub fn add_technology(&mut self, tech: Technology) {
        let is_new = self.techs_researched.insert(tech);

        // Remove from research queue
        self.techs_to_research.retain(|t| t != &tech);

        // Remove from in-progress
        self.techs_in_progress.remove(&tech);

        // Add to researched list
        // TODO: Need to get Technology object from ruleset
        // self.researched_technologies.push(tech);

        // Update transient boolean values
        self.update_transient_booleans();

        if is_new {
            // TODO: Add popup notification
            // civ_info.popup_alerts.add(PopupAlert(AlertType.TechResearched, tech_name))
        }
    }

    /// Update transient boolean values
    fn update_transient_booleans(&mut self) {
        // TODO: Need to implement unique ability check
        // self.units_can_embark = civ_info.hasUnique(UniqueType.LandUnitEmbarkation);
        // self.all_units_can_enter_ocean = ...;
        // self.embarked_units_can_enter_ocean = ...;
        // self.specific_units_can_enter_ocean = ...;
        // self.movement_speed_on_roads = ...;
        // self.roads_connect_across_rivers = ...;
        // self.all_techs_are_researched = ...;
    }

    /// Update at end of turn
    pub fn end_turn(
        &mut self,
        science_for_new_turn: i32,
        civ_data: &CivData,
        turn: u32,
        game_setting: &GameSetting,
        map_setting: &MapSetting,
    ) {
        // Update science values from last 8 turns
        self.science_of_last_8_turns[turn as usize % 8] = science_for_new_turn;

        let current_tech = match self.current_researching_technology() {
            Some(tech) => tech,
            None => panic!("No technology is being researched"),
        };

        let mut final_science = science_for_new_turn;

        // Research agreement bonus
        if self.science_from_research_agreements != 0 {
            let boost = self.science_from_research_agreements / 3;
            final_science += boost;
            self.science_from_research_agreements = 0;
        }

        // Overflow science bonus
        if self.overflow_science != 0 {
            final_science += self.overflow_science;
            self.overflow_science = 0;
        }

        self.add_science(
            final_science,
            current_tech,
            civ_data,
            game_setting,
            map_setting,
        );
    }

    /// Set transient data
    pub fn set_transients(&mut self, map_setting: &MapSetting) {
        // TODO: Need to populate researched_technologies from techs_researched
        // self.researched_technologies = techs_researched.map { getRuleset().technologies[it]!! };

        self.update_era(map_setting);
        self.update_transient_booleans();
    }

    /// Update era
    fn update_era(&mut self, map_setting: &MapSetting) {
        let ruleset = &map_setting.0.ruleset;

        if self.techs_researched.is_empty() {
            return;
        }

        // TODO: Need to implement era update logic
        // Find the highest era among researched technologies
        // Find the lowest era among unresearched technologies
        // Take the later era between the two
    }
}

impl Default for TechManager {
    fn default() -> Self {
        Self::new(Era::AncientEra)
    }
}

pub fn insert_tech_manager_map(
    mut commands: Commands,
    game_setting: Res<GameSetting>,
    civilization_states: Res<CivilizationStates>,
) {
    commands.insert_resource(TechManagerMap::new(
        &civilization_states,
        game_setting.start_era,
    ));
}
