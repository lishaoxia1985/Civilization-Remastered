use bevy::prelude::*;
use civ_map_generator::nation::Nation;

#[derive(Component, Clone, Copy)]
pub enum Owner {
    Civilization(Nation),
    CityState(Nation),
}

#[derive(Component)]
pub enum Unit {
    Civilian(String),
    Military(String),
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
