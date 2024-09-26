mod component;

use std::collections::BTreeMap;

use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};
pub use component::*;
use rand::rngs::StdRng;

use crate::grid::Direction;

#[derive(Resource)]
pub struct River(pub HashMap<i32, Vec<(Entity, Direction)>>);

#[derive(Resource)]
pub struct RandomNumberGenerator {
    pub rng: StdRng,
}

#[derive(Resource)]
pub struct TileStorage {
    pub tiles: Vec<Entity>,
}

#[derive(Resource)]
/// Store all the area ids and their sizes
pub struct AreaIdAndSize(pub BTreeMap<i32, u32>);
