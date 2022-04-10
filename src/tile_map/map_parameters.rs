use std::time::{SystemTime, UNIX_EPOCH};

use bevy::math::DVec2;

use super::{
    hex::{HexOrientation, Offset},
    HexLayout,
};

pub struct MapParameters {
    pub name: String,
    //it.type = type
    //it.mapResources = mapResources
    pub map_size: MapSize,
    pub hex_layout: HexLayout,
    /// the map use which type of offset coordinate
    pub offset: Offset,
    pub no_ruins: bool,
    //it.worldWrap = worldWrap
    //it.mods = LinkedHashSet(mods)
    //it.baseRuleset = baseRuleset
    pub seed: u64,
    pub large_lake_num: i32,
    pub lake_max_area_size: i32,
    /// Its 'length' is the number of iterations,
    /// its 'element' is the chance for each eligible plot to become an expansion coast in each iteration. \
    /// If it is empty the coast will not expand.
    pub coast_expansion_chance: Vec<f64>,
    pub sea_level: SeaLevel,
    pub world_age: WorldAge,
    pub temperature: Temperature,
    pub rainfall: Rainfall,
    pub natural_wonder_num: i32, // In fact, it is related to map size, don't need to set singlely.
}

pub struct MapSize {
    pub width: i32,
    pub height: i32,
}

/* pub enum MapSize {
    Duel,
    Tiny,
    Small,
    Standard,
    Large,
    Huge,
} */

pub enum SeaLevel {
    Low,
    Normal,
    High,
    Random,
}

pub enum WorldAge {
    Old,
    Normal,
    New,
}

pub enum Temperature {
    Cool,
    Normal,
    Hot,
}

pub enum Rainfall {
    Arid,
    Normal,
    Wet,
    Random,
}

impl Default for MapParameters {
    fn default() -> Self {
        Self {
            name: "perlin map".to_owned(),
            map_size: MapSize {
                width: 100,
                height: 50,
            },
            hex_layout: HexLayout {
                orientation: HexOrientation::Flat,
                size: DVec2::new(8., 8.),
                origin: DVec2::new(0., 0.),
            },
            offset: Offset::Odd,
            no_ruins: false,
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .try_into()
                .unwrap(),
            large_lake_num: 2,
            lake_max_area_size: 9,
            coast_expansion_chance: vec![0.25, 0.25],
            sea_level: SeaLevel::Normal,
            world_age: WorldAge::Normal,
            temperature: Temperature::Normal,
            rainfall: Rainfall::Normal,
            natural_wonder_num: 6,
        }
    }
}
