use std::time::{SystemTime, UNIX_EPOCH};

use bevy::{math::DVec2, prelude::Resource};

use crate::grid::hex::{Direction, HexLayout, HexOrientation, Offset, OffsetCoordinate};

#[derive(Resource)]
pub struct MapParameters {
    pub name: String,
    //it.type = type
    //it.mapResources = mapResources
    pub map_size: MapSize,
    pub hex_layout: HexLayout,
    pub wrap_x: bool,
    pub wrap_y: bool,
    /// the map use which type of offset coordinate
    pub offset: Offset,
    pub no_ruins: bool,
    //it.worldWrap = worldWrap
    //it.mods = LinkedHashSet(mods)
    //it.baseRuleset = baseRuleset
    pub seed: u64,
    pub large_lake_num: u32,
    /// The max area size of a lake.
    pub lake_max_area_size: u32,
    /// Its 'length' is the number of iterations,
    /// its 'element' is the chance for each eligible plot to become an expansion coast in each iteration. \
    /// If it is empty the coast will not expand, and then only the water tile is adjacent to the land tile can become coast.
    pub coast_expand_chance: Vec<f64>,
    pub sea_level: SeaLevel,
    pub world_age: WorldAge,
    pub temperature: Temperature,
    pub rainfall: Rainfall,
    /// In fact, it is related to map size, we don't need to set singlely.
    pub natural_wonder_num: u32,
}

#[derive(Clone, Copy)]
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
        let mut map_parameters = Self {
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
            wrap_x: true,
            wrap_y: false,
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
            coast_expand_chance: vec![0.25, 0.25],
            sea_level: SeaLevel::Normal,
            world_age: WorldAge::Normal,
            temperature: Temperature::Normal,
            rainfall: Rainfall::Normal,
            natural_wonder_num: 6,
        };
        map_parameters.update_origin();
        map_parameters
    }
}

impl MapParameters {
    pub fn update_origin(&mut self) {
        let width = self.map_size.width;
        let height = self.map_size.height;

        let (min_offset_x, min_offset_y) = [0, 1, width].into_iter().fold(
            (0.0_f64, 0.0_f64),
            |(min_offset_x, min_offset_y), index| {
                let offset_coordinate = self.index_to_offset_coordinate(index as usize);
                let hex = offset_coordinate.to_hex(self.offset, self.hex_layout.orientation);

                let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
                (min_offset_x.min(offset_x), min_offset_y.min(offset_y))
            },
        );

        let (max_offset_x, max_offset_y) = [
            width * (height - 1) - 1,
            width * height - 2,
            width * height - 1,
        ]
        .into_iter()
        .fold((0.0_f64, 0.0_f64), |(max_offset_x, max_offset_y), index| {
            let offset_coordinate = self.index_to_offset_coordinate(index as usize);
            let hex = offset_coordinate.to_hex(self.offset, self.hex_layout.orientation);

            let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
            (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
        });

        self.hex_layout.origin =
            -(DVec2::new(min_offset_x, min_offset_y) + DVec2::new(max_offset_x, max_offset_y)) / 2.;
    }

    pub const fn index_to_offset_coordinate(&self, index: usize) -> OffsetCoordinate {
        let map_size = self.map_size;
        assert!(index < (map_size.width * map_size.height) as usize);
        let x = index as i32 % map_size.width;
        let y = index as i32 / map_size.width;
        OffsetCoordinate::new(x, y)
    }

    pub const fn offset_coordinate_to_index(&self, offset_coordinate: OffsetCoordinate) -> usize {
        let map_size = self.map_size;
        let width = self.map_size.width as i32;
        let height = self.map_size.height as i32;
        // Check if the offset coordinate is inside the map
        let [mut x, mut y] = offset_coordinate.to_array();

        if self.wrap_x {
            x = (x % width + width) % width
        };
        if self.wrap_y {
            y = (y % height + height) % height
        };

        assert!((x >= 0) && (x < map_size.width) && (y >= 0) && (y < map_size.height));

        (x + y * map_size.width) as usize
    }

    pub const fn edge_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.edge_direction()
    }

    pub const fn corner_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.corner_direction()
    }

    /// Calculates the latitude of the tile on the tile map.
    ///
    /// Define that the latitude of the equator is `0.0` and the latitudes of the poles are `1.0`.
    /// The closer the latitude is to `0.0`, the closer the tile is to the equator; the closer the latitude is to `1.0`, the closer the tile is to the poles.
    pub fn latitude(&self, index: usize) -> f64 {
        let y = Self::index_to_offset_coordinate(self, index).0.y;
        let half_height = self.map_size.height as f64 / 2.0;
        ((half_height - y as f64) / half_height).abs()
    }
}
