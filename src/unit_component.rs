use bevy::prelude::*;
use civ_map_generator::ruleset::enums::{Nation, Unit};

#[derive(Component, Clone, Copy)]
pub enum Owner {
    Civilization(Nation),
    CityState(Nation),
}

#[derive(Component)]
pub enum UnitComponent {
    Civilian(Unit),
    Military(Unit),
}

#[derive(Component)]
pub struct Strength(pub u32);

#[derive(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

#[derive(Component)]
pub struct Movement {
    pub current: u32,
    pub max: u32,
}

#[derive(Component)]
pub struct Promotion(Vec<String>);

const START_UNITS: [&str; 2] = ["Settler", "Warrior"];
