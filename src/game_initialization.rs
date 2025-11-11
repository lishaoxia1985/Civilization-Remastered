use bevy::prelude::*;
use civ_map_generator::{nation::Nation, tile::Tile};

use crate::{RulesetResource, TileMapResource, assets::AppState, world_map::WorldTile};

#[derive(Component)]
pub enum Owner {
    Civilization(Nation),
    CityState(Nation),
}

#[derive(Component)]
pub enum Unit {
    Civilian(String),
    Military(String),
}

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

    for (&tile, &nation) in civ {
        let replace_warrior_unit = ruleset
            .units
            .values()
            .find(|&unit| unit.unique_to == nation.as_str() && unit.replaces == "Warrior");
        let military_unit = if let Some(unit) = replace_warrior_unit {
            unit.name.clone()
        } else {
            "Warrior".to_string()
        };

        // Spawn the military unit
        commands.spawn((
            Unit::Military(military_unit),
            Owner::Civilization(nation),
            WorldTile(tile),
        ));

        // Spawn the civilian unit
        commands.spawn((
            Unit::Civilian("Settler".to_string()),
            Owner::Civilization(nation),
            WorldTile(tile),
        ));
    }

    for (&tile, &nation) in city_state {
        commands.spawn((
            Unit::Civilian("Settler".to_string()),
            Owner::CityState(nation),
            WorldTile(tile),
        ));
    }

    next_state.set(AppState::GameStart);
}
