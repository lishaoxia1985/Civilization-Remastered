use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::ruleset::Ruleset;

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug, Component)]
pub enum Feature {
    Forest,
    Jungle,
    Marsh,
    Floodplain,
    Oasis,
    Ice,
    Atoll,
    Fallout,
}

impl Feature {
    pub fn name(&self) -> &str {
        match self {
            Feature::Forest => "Forest",
            Feature::Jungle => "Jungle",
            Feature::Marsh => "Marsh",
            Feature::Floodplain => "Floodplain",
            Feature::Oasis => "Oasis",
            Feature::Ice => "Ice",
            Feature::Atoll => "Atoll",
            Feature::Fallout => "Fallout",
        }
    }

    pub fn impassable(&self, ruleset: &Ruleset) -> bool {
        ruleset.features[self.name()].impassable
    }
}
