use std::collections::HashMap;

use bevy::prelude::*;
use civ_map_generator::{nation::Nation, tile::Tile};

use crate::{RulesetResource, TileMapResource, assets::AppState};

#[derive(Resource)]
pub struct UnitListResource(pub HashMap<Tile, Vec<String>>);

struct MapUnit {
    name: String,
    owner: Nation,
    position: Tile,
    strength: u32,
    movement: u32,
    hp: u32,
    promotion: Vec<String>,
}
struct NationUnit {
    unit_list: Vec<MapUnit>,
}

const START_UNITS: [&str; 2] = ["Settler", "Warrior"];

pub fn game_initialization(
    mut commands: Commands,
    map: Option<Res<TileMapResource>>,
    ruleset: Res<RulesetResource>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if map.is_none() {
        return;
    };

    let tile_map = &map.unwrap().0;
    let ruleset = &ruleset.0;

    let civ = &tile_map.starting_tile_and_civilization;
    let city_state = &tile_map.starting_tile_and_city_state;
    let mut tile_and_units = HashMap::new();
    for (&tile, nation) in civ {
        let replace_warrior_unit = ruleset
            .units
            .values()
            .find(|&unit| unit.unique_to == nation.as_str() && unit.replaces == "Warrior");
        let military_unit = if let Some(unit) = replace_warrior_unit {
            unit.name.clone()
        } else {
            "Warrior".to_string()
        };

        tile_and_units
            .entry(tile)
            .or_insert(Vec::new())
            .push(military_unit);
        tile_and_units
            .entry(tile)
            .or_insert(Vec::new())
            .push("Settler".to_string());
    }

    for (&tile, _) in city_state {
        tile_and_units
            .entry(tile)
            .or_insert(Vec::new())
            .push("Settler".to_string());
    }

    commands.insert_resource(UnitListResource(tile_and_units));

    next_state.set(AppState::GameStart);
}
