mod component;
mod system;
mod tile_query;

use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};
pub use component::*;
use rand::rngs::StdRng;
pub use system::*;
pub use tile_query::*;

use crate::tile_map::Direction;

#[derive(Resource)]
pub struct River(pub HashMap<i32, Vec<(Entity, Direction)>>);

#[derive(Resource)]
pub struct RandomNumberGenerator {
    pub rng: StdRng,
}
