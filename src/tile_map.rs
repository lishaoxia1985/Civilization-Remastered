use std::cmp::max;
use std::collections::{BTreeMap, VecDeque};

use bevy::math::IVec2;
use bevy::utils::HashSet;
use bevy::{math::DVec2, prelude::Res, utils::HashMap};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::ruleset::{Ruleset, Unique};
pub use crate::tile_map::hex::Direction;
mod fractal;
mod map_parameters;
mod tile;

use self::fractal::{CvFractal, Flags};
use self::hex::{Hex, Offset, OffsetCoordinate};
use self::tile::Tile;
pub use hex::{HexLayout, HexOrientation};
pub use map_parameters::{MapParameters, MapSize, SeaLevel};
pub mod hex;

pub struct TileMap {
    pub map_parameters: MapParameters,
    pub random_number_generator: StdRng,
    pub tile_list: BTreeMap<[i32; 2], Tile>,
    pub river_list: HashMap<i32, Vec<([i32; 2], Direction)>>,
}

impl TileMap {
    pub fn new(map_parameters: MapParameters, ruleset: &Res<Ruleset>) -> Self {
        let random_number_generator = StdRng::seed_from_u64(map_parameters.seed);
        let tile_list = Self::rectangular_map(
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            map_parameters.hex_layout,
            map_parameters.offset,
            ruleset,
        );
        Self {
            map_parameters,
            random_number_generator,
            tile_list,
            river_list: HashMap::new(),
        }
    }

    pub fn rectangular_map(
        width: i32,
        height: i32,
        hex_layout: HexLayout,
        offset: Offset,
        ruleset: &Res<Ruleset>,
    ) -> BTreeMap<[i32; 2], Tile> {
        let mut tile_list = BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                let offset_coordinate = OffsetCoordinate::new(x, y);
                let hex_coordinate = offset_coordinate.to_hex(offset, hex_layout.orientation);

                let tile = Tile::new(hex_coordinate.to_array(), ruleset);
                tile_list.insert(hex_coordinate.to_array(), tile);
            }
        }
        tile_list
    }

    pub const fn tile_edge_direction(&self) -> [Direction; 6] {
        self.map_parameters.hex_layout.orientation.edge_direction()
    }

    pub const fn tile_corner_direction(&self) -> [Direction; 6] {
        self.map_parameters
            .hex_layout
            .orientation
            .corner_direction()
    }

    pub fn spawn_tile_type_for_fractal(&mut self, ruleset: &Res<Ruleset>) {
        let continent_grain = 2;

        let sea_level_low = 65;
        let sea_level_normal = 72;
        let sea_level_high = 78;
        let world_age_old = 2;
        let world_age_normal = 3;
        let world_age_new = 5;

        let extra_mountains = 0;

        let tectonic_islands = false;

        let adjustment = match self.map_parameters.world_age {
            map_parameters::WorldAge::Old => world_age_old,
            map_parameters::WorldAge::Normal => world_age_normal,
            map_parameters::WorldAge::New => world_age_new,
        };

        let mountains = 97 - adjustment - extra_mountains;
        let hills_near_mountains = 91 - (adjustment * 2) - extra_mountains;
        let hills_bottom1 = 28 - adjustment;
        let hills_top1 = 28 + adjustment;
        let hills_bottom2 = 72 - adjustment;
        let hills_top2 = 72 + adjustment;
        let hills_clumps = 1 + adjustment;

        let water_percent = match self.map_parameters.sea_level {
            SeaLevel::Low => sea_level_low,
            SeaLevel::Normal => sea_level_normal,
            SeaLevel::High => sea_level_high,
            SeaLevel::Random => {
                sea_level_low
                    + self
                        .random_number_generator
                        .gen_range(0..=(sea_level_high - sea_level_low))
            }
        };

        let orientation = self.map_parameters.hex_layout.orientation;
        let offset = self.map_parameters.offset;

        let mut continents_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        continents_fractal.ridge_builder(
            &mut self.random_number_generator,
            15,
            &Flags::default(),
            1,
            2,
            orientation,
            offset,
        );

        let mut mountains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        mountains_fractal.ridge_builder(
            &mut self.random_number_generator,
            10,
            &Flags::default(),
            6,
            1,
            orientation,
            offset,
        );

        let mut hills_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        hills_fractal.ridge_builder(
            &mut self.random_number_generator,
            15,
            &Flags::default(),
            1,
            2,
            orientation,
            offset,
        );

        let [water_threshold] = continents_fractal.get_height_from_percents(&[water_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let [pass_threshold, hills_bottom1, hills_top1, hills_bottom2, hills_top2] = hills_fractal
            .get_height_from_percents(&[
                hills_near_mountains,
                hills_bottom1,
                hills_top1,
                hills_bottom2,
                hills_top2,
            ])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let [mountain_threshold, hills_near_mountains, hills_clumps, mountain_100, mountain_99, mountain_98, mountain_97, mountain_95] =
            mountains_fractal.get_height_from_percents(&[
                mountains,
                hills_near_mountains,
                hills_clumps,
                100,
                99,
                98,
                97,
                95,
            ])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        self.tile_list.values_mut().for_each(|tile| {
            let hex_coord = Hex::from(tile.hex_position);
            let offset_coord = hex_coord.to_offset_coordinate(
                self.map_parameters.offset,
                self.map_parameters.hex_layout.orientation,
            );

            let [x, y] = offset_coord.to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);

            if height <= water_threshold {
                tile.base_terrain = ruleset.terrains["Ocean"].clone();
                if tectonic_islands {
                    if mountain_height == mountain_100 {
                        tile.base_terrain = ruleset.terrains["Mountain"].clone();
                    } else if mountain_height == mountain_99 {
                        tile.terrain_features.push(ruleset.terrains["Hill"].clone());
                        tile.base_terrain = ruleset.terrains["Grassland"].clone();
                    } else if (mountain_height == mountain_97) || (mountain_height == mountain_95) {
                        tile.base_terrain = ruleset.terrains["Grassland"].clone();
                    }
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    tile.terrain_features.push(ruleset.terrains["Hill"].clone());
                    tile.base_terrain = ruleset.terrains["Grassland"].clone();
                } else {
                    tile.base_terrain = ruleset.terrains["Mountain"].clone();
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                tile.terrain_features.push(ruleset.terrains["Hill"].clone());
                tile.base_terrain = ruleset.terrains["Grassland"].clone();
            } else {
                tile.base_terrain = ruleset.terrains["Grassland"].clone();
            };
        });
    }

    pub fn spawn_tile_type_for_pangaea(&mut self, ruleset: &Res<Ruleset>) {
        let continent_grain = 2;

        let sea_level_low = 71;
        let sea_level_normal = 78;
        let sea_level_high = 84;
        let world_age_old = 2;
        let world_age_normal = 3;
        let world_age_new = 5;

        let extra_mountains = 0;

        let adjustment = match self.map_parameters.world_age {
            map_parameters::WorldAge::Old => world_age_old,
            map_parameters::WorldAge::Normal => world_age_normal,
            map_parameters::WorldAge::New => world_age_new,
        };

        let mountains = 97 - adjustment - extra_mountains;
        let hills_near_mountains = 91 - (adjustment * 2) - extra_mountains;
        let hills_bottom1 = 28 - adjustment;
        let hills_top1 = 28 + adjustment;
        let hills_bottom2 = 72 - adjustment;
        let hills_top2 = 72 + adjustment;
        let hills_clumps = 1 + adjustment;

        let water_percent = match self.map_parameters.sea_level {
            SeaLevel::Low => sea_level_low,
            SeaLevel::Normal => sea_level_normal,
            SeaLevel::High => sea_level_high,
            SeaLevel::Random => {
                sea_level_low
                    + self
                        .random_number_generator
                        .gen_range(0..=(sea_level_high - sea_level_low))
            }
        };

        let orientation = self.map_parameters.hex_layout.orientation;
        let offset = self.map_parameters.offset;

        let mut continents_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        continents_fractal.ridge_builder(
            &mut self.random_number_generator,
            15,
            &Flags::default(),
            1,
            2,
            orientation,
            offset,
        );

        let mut mountains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        mountains_fractal.ridge_builder(
            &mut self.random_number_generator,
            10,
            &Flags::default(),
            6,
            1,
            orientation,
            offset,
        );

        let mut hills_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            2,
            Flags::default(),
            7,
            6,
        );

        hills_fractal.ridge_builder(
            &mut self.random_number_generator,
            15,
            &Flags::default(),
            1,
            2,
            orientation,
            offset,
        );

        let [water_threshold] = continents_fractal.get_height_from_percents(&[water_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let [pass_threshold, hills_bottom1, hills_top1, hills_bottom2, hills_top2] = hills_fractal
            .get_height_from_percents(&[
                hills_near_mountains,
                hills_bottom1,
                hills_top1,
                hills_bottom2,
                hills_top2,
            ])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let [mountain_threshold, hills_near_mountains, hills_clumps, mountain_100, mountain_99, mountain_98, mountain_97, mountain_95] =
            mountains_fractal.get_height_from_percents(&[
                mountains,
                hills_near_mountains,
                hills_clumps,
                100,
                99,
                98,
                97,
                95,
            ])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let width = self.map_parameters.map_size.width;
        let height = self.map_parameters.map_size.height;
        let center_position = DVec2::new(width as f64 / 2., height as f64 / 2.);

        let axis = center_position * 3. / 5.;

        self.tile_list.values_mut().for_each(|tile| {
            let hex_coord = Hex::from(tile.hex_position);
            let offset_coord = hex_coord.to_offset_coordinate(
                self.map_parameters.offset,
                self.map_parameters.hex_layout.orientation,
            );
            let [x, y] = offset_coord.to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);

            let mut h = water_threshold as f64;

            let delta = IVec2::from([x, y]).as_dvec2() - center_position;
            let d = (delta / axis).length_squared();

            if d <= 1. {
                h = h + (h * 0.125)
            } else {
                h = h - (h * 0.125)
            }

            let height = ((height as f64 + h + h) * 0.33) as i32;

            if height <= water_threshold {
                tile.base_terrain = ruleset.terrains["Ocean"].clone();
                if height == mountain_100 {
                    tile.base_terrain = ruleset.terrains["Mountain"].clone()
                } else if height == mountain_99 {
                    tile.base_terrain = ruleset.terrains["Hill"].clone()
                } else if height == mountain_97 || height == mountain_95 {
                    tile.base_terrain = ruleset.terrains["Grassland"].clone()
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    tile.terrain_features.push(ruleset.terrains["Hill"].clone());
                    tile.base_terrain = ruleset.terrains["Grassland"].clone();
                } else {
                    tile.base_terrain = ruleset.terrains["Mountain"].clone();
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                tile.terrain_features.push(ruleset.terrains["Hill"].clone());
                tile.base_terrain = ruleset.terrains["Grassland"].clone();
            } else {
                tile.base_terrain = ruleset.terrains["Grassland"].clone();
            };
        });
    }

    pub fn generate_coasts(&mut self, ruleset: &Res<Ruleset>) {
        let tile_list_position: Vec<_> = self.tile_list.keys().copied().collect();
        for hex_position in tile_list_position.iter() {
            let tile = &self.tile_list[hex_position];
            if tile.base_terrain.name == "Ocean"
                && tile
                    .tiles_neighbors(self)
                    .iter()
                    .any(|tile| tile.base_terrain.r#type == "Land")
            {
                self.tile_list.get_mut(hex_position).unwrap().base_terrain =
                    ruleset.terrains["Coast"].clone();
            }
        }

        for chance in &self.map_parameters.coast_expansion_chance {
            for hex_position in tile_list_position.iter() {
                let tile = &self.tile_list[hex_position];
                if tile.base_terrain.name == "Ocean"
                    && tile
                        .tiles_neighbors(self)
                        .iter()
                        .any(|tile| tile.base_terrain.name == "Coast")
                    && self.random_number_generator.gen_bool(*chance)
                {
                    self.tile_list.get_mut(hex_position).unwrap().base_terrain =
                        ruleset.terrains["Coast"].clone();
                }
            }
        }
    }

    /// This fun is used when we create the world and some water areas surrounded by land.
    /// In original Civ, the random world create by Voronoi Noise doesn't have situation, so the fun
    /// is only used when we create the world by ourselves. But in our code, we should tackle with this
    /// situation.
    pub fn generate_lakes(&mut self, ruleset: &Res<Ruleset>) {
        self.recalculate_areas();
        let max_area_id = self
            .tile_list
            .values()
            .map(|tile| tile.area_id)
            .max()
            .unwrap();
        for area_id in -1..=max_area_id {
            let water_tiles: Vec<_> = self
                .tile_list
                .values_mut()
                .filter(|tile| tile.area_id == area_id && tile.is_water())
                .collect();

            let water_area_size = water_tiles.len() as i32;

            if (1..=self.map_parameters.lake_max_area_size).contains(&water_area_size) {
                for tile in water_tiles {
                    tile.base_terrain = ruleset.terrains["Lakes"].clone();
                }
            }
        }
    }

    pub fn generate_terrain(&mut self, ruleset: &Res<Ruleset>) {
        let temperature_shift = 0.1;
        let desert_shift = 16;
        let mut desert_percent = 32;
        let plains_percent = 50;
        let mut snow_latitude = 0.75;
        let mut tundra_latitude = 0.6;
        let mut grass_latitude = 0.1;
        let desert_bottom_latitude = 0.2;
        let mut desert_top_latitude = 0.5;

        match self.map_parameters.temperature {
            map_parameters::Temperature::Cool => {
                desert_percent -= desert_shift;
                tundra_latitude -= temperature_shift * 1.5;
                desert_top_latitude -= temperature_shift;
                grass_latitude -= temperature_shift * 0.5;
            }
            map_parameters::Temperature::Normal => {}
            map_parameters::Temperature::Hot => {
                desert_percent += desert_shift;
                snow_latitude += temperature_shift * 0.5;
                tundra_latitude += temperature_shift;
                desert_top_latitude += temperature_shift;
                grass_latitude -= temperature_shift * 0.5;
            }
        }

        let desert_top_percent = 100;
        let desert_bottom_percent = max(0, 100 - desert_percent);
        let plains_top_percent = 100;
        let plains_bottom_percent = max(0, 100 - plains_percent);

        //let (seed, seed2, seed3) = self.random_number_generator.gen();
        let variation_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            3,
            Flags::default(),
            -1,
            -1,
        );
        let deserts_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            3,
            Flags::default(),
            -1,
            -1,
        );
        let plains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            self.map_parameters.map_size.width,
            self.map_parameters.map_size.height,
            3,
            Flags::default(),
            -1,
            -1,
        );

        let [desert_top, plains_top] =
            deserts_fractal.get_height_from_percents(&[desert_top_percent, plains_top_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };
        let [desert_bottom, plains_bottom] = plains_fractal
            .get_height_from_percents(&[desert_bottom_percent, plains_bottom_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        let width = self.map_parameters.map_size.width;
        let height = self.map_parameters.map_size.height;

        self.tile_list
            .values_mut()
            .filter(|tile| {
                tile.base_terrain.name != "Ocean" && tile.base_terrain.name != "Mountain"
            })
            .for_each(|tile| {
                let hex_coord = Hex::from(tile.hex_position);
                let offset_coord = hex_coord.to_offset_coordinate(
                    self.map_parameters.offset,
                    self.map_parameters.hex_layout.orientation,
                );
                let [x, y] = offset_coord.to_array();

                let deserts_height = deserts_fractal.get_height(x, y);
                let plains_height = plains_fractal.get_height(x, y);

                let mut latitude = ((height as f64 / 2. - y as f64) / (height as f64 / 2.)).abs();
                latitude += (128 - variation_fractal.get_height(x, y)) as f64 / (255.0 * 5.0);
                latitude = latitude.clamp(0., 1.);

                if latitude >= snow_latitude {
                    tile.base_terrain = ruleset.terrains["Snow"].clone()
                } else if latitude >= tundra_latitude {
                    tile.base_terrain = ruleset.terrains["Tundra"].clone()
                } else if latitude < grass_latitude {
                    tile.base_terrain = ruleset.terrains["Grassland"].clone()
                } else if deserts_height >= desert_bottom
                    && deserts_height <= desert_top
                    && latitude >= desert_bottom_latitude
                    && latitude < desert_top_latitude
                {
                    tile.base_terrain = ruleset.terrains["Desert"].clone()
                } else if plains_height >= plains_bottom && plains_height <= plains_top {
                    tile.base_terrain = ruleset.terrains["Plains"].clone()
                }
            });
    }

    pub fn recalculate_areas(&mut self) {
        // area id of all the tiles is set to default value (-1)
        self.tile_list
            .values_mut()
            .for_each(|tile| tile.area_id = -1);
        // water area, excluding impassable tile ( e.g. ice, natural-wonder in water)
        self.bfs(|tile| tile.is_water() && !tile.base_terrain.impassable);
        // mountain area
        self.bfs(|tile| tile.is_mountain());
        // other land area (including hill, excluding natural-wonder and mountain)
        self.bfs(|tile| tile.is_land() && !tile.base_terrain.impassable);
        // impassable area (including ice and natural-wonder, excluding mountain)
        self.bfs(|tile| tile.base_terrain.impassable && !tile.is_mountain());
    }

    fn bfs(&mut self, filter_condition: impl Fn(&Tile) -> bool) {
        let mut area_tiles_positions: HashSet<_> = self
            .tile_list
            .values()
            .filter_map(|tile| filter_condition(tile).then_some(tile.hex_position))
            .collect();
        let mut current_area_id = self
            .tile_list
            .values()
            .map(|tile| tile.area_id)
            .max()
            .unwrap()
            + 1;
        while let Some(&initial_area_tile_position) = area_tiles_positions.iter().next() {
            area_tiles_positions.remove(&initial_area_tile_position);
            let mut tiles_in_current_area_positions = HashSet::new();
            tiles_in_current_area_positions.insert(initial_area_tile_position);
            self.tile_list
                .get_mut(&initial_area_tile_position)
                .unwrap()
                .area_id = current_area_id;
            let mut tiles_to_check_positions = VecDeque::new();
            tiles_to_check_positions.push_back(initial_area_tile_position);
            while let Some(tile_we_are_checking_position) = tiles_to_check_positions.pop_front() {
                let neighbors_tiles_positions: Vec<_> = self.tile_list
                    [&tile_we_are_checking_position]
                    .tiles_neighbors(self)
                    .iter()
                    .filter_map(|tile| {
                        {
                            !tiles_in_current_area_positions.contains(&tile.hex_position)
                                && filter_condition(tile)
                        }
                        .then_some(tile.hex_position)
                    })
                    .collect();
                for position in neighbors_tiles_positions.iter() {
                    tiles_in_current_area_positions.insert(*position);
                    self.tile_list.get_mut(position).unwrap().area_id = current_area_id;
                    tiles_to_check_positions.push_back(*position);
                    area_tiles_positions.remove(position);
                }
            }
            current_area_id += 1;
        }
    }

    pub fn add_rivers(&mut self, ruleset: &Res<Ruleset>) {
        let river_source_range_default = 4;
        let sea_water_range_default = 3;
        const plots_per_river_edge: i32 = 12;

        fn pass_conditions(
            tile: &Tile,
            tile_map: &TileMap,
            random_number_generator: &mut StdRng,
        ) -> [bool; 4] {
            let num_tiles = tile_map
                .tile_list
                .values()
                .filter(|x| x.area_id == tile.area_id)
                .count() as i32;
            let num_river_edges = num_river_edges(tile, tile_map);
            [
                tile.is_hill() || tile.is_mountain(),
                tile.is_coastal_land(tile_map) && random_number_generator.gen_range(0..8) == 0,
                (tile.is_hill() || tile.is_mountain())
                    && (num_river_edges < num_tiles / plots_per_river_edge + 1),
                num_river_edges < num_tiles / plots_per_river_edge + 1,
            ]
        }

        // Returns the number of river edges in the area where the tile is
        // 1. Get the area where the tile is
        // 2. Get the number of rivers edge which the tile in the area own
        fn num_river_edges(tile: &Tile, tile_map: &TileMap) -> i32 {
            let mut num_river_edges = 0;
            let area_tile_list: Vec<_> = tile_map
                .tile_list
                .values()
                .filter(|x| x.area_id == tile.area_id)
                .collect();
            area_tile_list.iter().for_each(|tile| {
                tile_map.river_list.values().for_each(|river_plot| {
                    num_river_edges = river_plot
                        .iter()
                        .filter(|(hex_position, _)| hex_position == &tile.hex_position)
                        .count();
                });
            });
            num_river_edges as i32
        }

        let mut random_number_generator = self.random_number_generator.clone();

        let candidate_start_tile_positions: Vec<_> = self
            .tile_list
            .iter()
            .filter_map(|(hex_position, tile)| {
                {
                    !tile.is_water()
                        && self.tile_edge_direction()[0..3].iter().all(|&direction| {
                            if let Some(neighbor_tile) = tile.tile_neighbor(self, direction) {
                                neighbor_tile.is_land()
                            } else {
                                false
                            }
                        })
                }
                .then_some(*hex_position)
            })
            .collect();
        let mut river_id = 0;

        (0..4).for_each(|index| {
            let (river_source_range, sea_water_range) = if index <= 1 {
                (river_source_range_default, sea_water_range_default)
            } else {
                (
                    (river_source_range_default / 2),
                    (sea_water_range_default / 2),
                )
            };

            for hex_position in candidate_start_tile_positions.iter() {
                let tile = &self.tile_list[hex_position];
                if pass_conditions(tile, self, &mut random_number_generator)[index]
                    && !tile
                        .tiles_in_distance(river_source_range, self)
                        .iter()
                        .any(|tile| tile.is_freshwater(self))
                    && !tile
                        .tiles_in_distance(sea_water_range, self)
                        .iter()
                        .any(|tile| tile.is_water())
                {
                    self.do_river(
                        hex_position,
                        Direction::NoDirection,
                        Direction::NoDirection,
                        river_id,
                    );
                    river_id += 1;
                }
            }
        });
        self.random_number_generator = random_number_generator;
    }

    fn do_river(
        &mut self,
        start_plot: &[i32; 2],
        this_flow_direction: Direction,
        original_flow_direction: Direction,
        river_id: i32,
    ) {
        // if the start plot have a river, exit the function
        if self.river_list.values().any(|river| {
            river
                .iter()
                .any(|(hex_position, _)| hex_position == start_plot)
        }) && original_flow_direction == Direction::NoDirection
        {
            return;
        }

        let mut original_flow_direction = original_flow_direction;

        let mut river_plot;
        let mut best_flow_direction = Direction::NoDirection;
        match self.map_parameters.hex_layout.orientation {
            HexOrientation::Pointy => match this_flow_direction {
                Direction::North => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::SouthEast, self)
                            || neighbor_tile.has_river(Direction::SouthWest, self)
                        {
                            return;
                        } else {
                            river_plot = neighbor_tile.hex_position;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthEast => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::East)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::East, self)
                            || neighbor_tile.has_river(Direction::SouthWest, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::East => unreachable!(),
                Direction::SouthEast => {
                    let start_tile = &self.tile_list[start_plot];
                    if let Some(neighbor_tile) = start_tile.tile_neighbor(self, Direction::East) {
                        river_plot = neighbor_tile.hex_position
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::East, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::South => {
                    let start_tile = &self.tile_list[start_plot];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::SouthWest)
                    {
                        river_plot = neighbor_tile.hex_position
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::East)
                    {
                        if neighbor_tile2.has_river(Direction::SouthWest, self) {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthWest => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::East, self)
                            || river_plot_tile.has_river(Direction::SouthWest, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::West => unreachable!(),
                Direction::NorthWest => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::West)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::East, self)
                            || neighbor_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        } else {
                            river_plot = neighbor_tile.hex_position;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NoDirection => {
                    river_plot = *start_plot;
                }
            },
            HexOrientation::Flat => match this_flow_direction {
                Direction::North => unreachable!(),
                Direction::NorthEast => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::NorthEast, self)
                            || neighbor_tile.has_river(Direction::South, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::East => {
                    let start_tile = &self.tile_list[start_plot];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::NorthEast)
                    {
                        river_plot = neighbor_tile.hex_position
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::South)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::NorthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthEast => {
                    let start_tile = &self.tile_list[start_plot];
                    if let Some(neighbor_tile) = start_tile.tile_neighbor(self, Direction::South) {
                        river_plot = neighbor_tile.hex_position
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::South, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::South => unreachable!(),
                Direction::SouthWest => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::South)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::South, self)
                            || neighbor_tile.has_river(Direction::NorthEast, self)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::West => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::NorthEast, self)
                            || neighbor_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        } else {
                            river_plot = neighbor_tile.hex_position;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthWest => {
                    river_plot = *start_plot;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot, this_flow_direction));
                    let river_plot_tile = &self.tile_list[&river_plot];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::North)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::South, self)
                            || neighbor_tile.has_river(Direction::SouthEast, self)
                        {
                            return;
                        } else {
                            river_plot = neighbor_tile.hex_position;
                        }
                    } else {
                        return;
                    }
                }
                Direction::NoDirection => {
                    river_plot = *start_plot;
                }
            },
        }

        let river_plot_tile = &self.tile_list[&river_plot];
        if river_plot_tile.is_water() {
            return;
        }

        // In this tuple, The first element is next possible flow, the second element is the direction of the special plot relative to current plot
        // We evaluate the weight value of the special plot using a certain algorithm and select the minimum one to determine the next direction of the river flow
        let adjacent_plot_directions = match self.map_parameters.hex_layout.orientation {
            HexOrientation::Pointy => [
                (Direction::North, Direction::NorthWest),
                (Direction::NorthEast, Direction::NorthEast),
                (Direction::SouthEast, Direction::East),
                (Direction::South, Direction::SouthWest),
                (Direction::SouthWest, Direction::West),
                (Direction::NorthWest, Direction::NorthWest),
            ],
            HexOrientation::Flat => [
                (Direction::East, Direction::NorthEast),
                (Direction::SouthEast, Direction::South),
                (Direction::SouthWest, Direction::SouthWest),
                (Direction::West, Direction::NorthWest),
                (Direction::NorthWest, Direction::NorthWest),
                (Direction::NorthEast, Direction::North),
            ],
        };

        fn next_flow_directions(flow_direction: Direction, tile_map: &TileMap) -> [Direction; 2] {
            let direction_array = tile_map.tile_corner_direction();
            let flow_direction_index = tile_map
                .map_parameters
                .hex_layout
                .orientation
                .corner_index(flow_direction);
            [
                direction_array[(flow_direction_index + 1) % 6], // turn_right_flow_direction
                direction_array[(flow_direction_index + 5) % 6], // turn_left_flow_direction
            ]
        }

        fn river_value_at_plot(plot: [i32; 2], tile_map: &mut TileMap) -> i32 {
            fn plot_elevation(tile: &Tile) -> i32 {
                if tile.is_mountain() {
                    4
                } else if tile.is_hill() {
                    3
                } else if tile.is_water() {
                    2
                } else {
                    1
                }
            }
            let tile = &tile_map.tile_list[&plot];
            let mut sum = plot_elevation(tile) * 20;
            let direction_array = tile_map.tile_edge_direction();
            direction_array.iter().for_each(|&direction| {
                if let Some(adjacent_tile) = tile.tile_neighbor(tile_map, direction) {
                    sum += plot_elevation(adjacent_tile);
                    if adjacent_tile.base_terrain.name == "Desert" {
                        sum += 4;
                    }
                } else {
                    sum += 40;
                }
            });
            sum += tile_map.random_number_generator.gen_range(0..10);
            sum
        }

        let adjacent_plot_list = adjacent_plot_directions
            .into_iter()
            .filter_map(|(flow_direction, direction)| {
                river_plot_tile
                    .tile_neighbor(self, direction)
                    .and_then(|neighbor_tile| Some((flow_direction, neighbor_tile.hex_position)))
            })
            .collect::<Vec<_>>();

        if best_flow_direction == Direction::NoDirection {
            let mut best_value = i32::MAX;
            for (flow_direction, adjacent_plot) in adjacent_plot_list.into_iter() {
                if flow_direction.opposite_direction() != original_flow_direction
                    && (this_flow_direction == Direction::NoDirection
                        || next_flow_directions(this_flow_direction, self)
                            .contains(&flow_direction))
                {
                    let mut value = river_value_at_plot(adjacent_plot, self);
                    if flow_direction == original_flow_direction {
                        value = (value * 3) / 4;
                    }
                    if value < best_value {
                        best_value = value;
                        best_flow_direction = flow_direction;
                    }
                }
            }
        }

        if best_flow_direction != Direction::NoDirection {
            if original_flow_direction == Direction::NoDirection {
                original_flow_direction = best_flow_direction;
            }
            self.do_river(
                &river_plot,
                best_flow_direction,
                original_flow_direction,
                river_id,
            )
        }
    }

    pub fn add_lakes(&mut self, ruleset: &Res<Ruleset>) {
        let large_lake_num = self.map_parameters.large_lake_num;

        let mut num_lakes_added = 0;
        let mut num_large_lakes_added = 0;
        let lake_plot_rand = 25;
        let direction_array = self.tile_edge_direction();

        let tile_list_position: Vec<_> = self.tile_list.keys().copied().collect();
        for hex_position in tile_list_position.iter() {
            let tile = &self.tile_list[hex_position];
            if !tile.is_water()
                && !tile.is_coastal_land(self)
                && !direction_array
                    .iter()
                    .any(|&direction| tile.has_river(direction, self))
                && self.random_number_generator.gen_range(0..lake_plot_rand) == 0
            {
                num_lakes_added += 1;
                if num_large_lakes_added < large_lake_num {
                    let add_more_lakes = self.add_more_lake(hex_position, ruleset);
                    if add_more_lakes {
                        num_large_lakes_added += 1;
                    }
                }
                let tile = self.tile_list.get_mut(hex_position).unwrap();
                tile.base_terrain = ruleset.terrains["Lakes"].clone();
                tile.terrain_features.clear();
            }
        }
        if num_lakes_added > 0 {
            self.recalculate_areas()
        }
    }

    fn add_more_lake(&mut self, hex_position: &[i32; 2], ruleset: &Res<Ruleset>) -> bool {
        let mut large_lake = 0;
        let mut lake_plots = Vec::new();
        let tile = &self.tile_list[hex_position];
        for &direction in self.tile_edge_direction().iter() {
            let neighbor_tile = tile.tile_neighbor(self, direction);
            if let Some(neighbor_tile) = neighbor_tile {
                if !neighbor_tile.is_water()
                    && !neighbor_tile.is_coastal_land(self)
                    && !self
                        .tile_edge_direction()
                        .iter()
                        .any(|&direction| neighbor_tile.has_river(direction, self))
                {
                    let hex_position = neighbor_tile.hex_position;
                    if self.random_number_generator.gen_range(0..(large_lake + 4)) < 3 {
                        lake_plots.push(hex_position);
                        large_lake += 1;
                    }
                }
            }
        }

        for lake_plot in lake_plots.iter() {
            let tile = self.tile_list.get_mut(lake_plot).unwrap();
            tile.base_terrain = ruleset.terrains["Lakes"].clone();
            tile.terrain_features.clear();
        }

        large_lake > 2
    }

    pub fn add_features(&mut self, ruleset: &Res<Ruleset>) {
        let rainfall = match self.map_parameters.rainfall {
            map_parameters::Rainfall::Arid => -4,
            map_parameters::Rainfall::Normal => 0,
            map_parameters::Rainfall::Wet => 4,
            map_parameters::Rainfall::Random => self.random_number_generator.gen_range(0..11) - 5,
        };

        let equator_adjustment = 0;
        let mut jungle_percent = 12;
        let mut forest_percent = 18;
        let mut marsh_percent = 3;
        let mut oasis_percent = 1;

        jungle_percent += rainfall;
        forest_percent += rainfall;
        marsh_percent += rainfall / 2;
        oasis_percent += rainfall / 4;

        // The variable is used to describe the relative position of the equator
        // equator_position = max_latitude * equator / 100.
        let equator = equator_adjustment;

        let jungle_max_percent = jungle_percent;
        let forest_max_percent = forest_percent;
        let marsh_max_percent = marsh_percent;
        let oasis_max_percent = oasis_percent;

        let mut forest_count = 0;
        let mut jungle_count = 0;
        let mut marsh_count = 0;
        let mut oasis_count = 0;
        let mut num_land_plots = 0;
        let jungle_bottom = equator - (jungle_percent as f64 * 0.5).ceil() as i32;
        let jungle_top = equator + (jungle_percent as f64 * 0.5).ceil() as i32;

        let tile_list_position: Vec<_> = self.tile_list.keys().copied().collect();

        let height = self.map_parameters.map_size.height;

        for hex_position in tile_list_position.iter() {
            let tile = &self.tile_list[hex_position];

            /* **********start to add ice********** */
            if !tile.base_terrain.impassable
                && tile.is_water()
                && !self
                    .tile_edge_direction()
                    .iter()
                    .any(|&direction| tile.has_river(direction, self))
                && ruleset.terrains["Ice"]
                    .occurs_on
                    .contains(&tile.base_terrain.name)
            {
                let hex_coord = Hex::from(tile.hex_position);
                let offset_coord = hex_coord.to_offset_coordinate(
                    self.map_parameters.offset,
                    self.map_parameters.hex_layout.orientation,
                );
                let [_x, y] = offset_coord.to_array();

                let latitude = ((height as f64 / 2. - y as f64) / (height as f64 / 2.)).abs();

                if latitude > 0.78 {
                    let mut score = self.random_number_generator.gen_range(0..100) as f64;
                    score += latitude * 100.;
                    let tile_neighbors = tile.tiles_neighbors(self);
                    if tile_neighbors.iter().any(|x| x.is_land()) {
                        score /= 2.0;
                    }
                    let a = tile_neighbors
                        .iter()
                        .filter(|x| {
                            x.terrain_features
                                .iter()
                                .any(|terrain| terrain.name == "Ice")
                        })
                        .count();
                    score += 10. * a as f64;
                    if score > 130. {
                        let tile = self.tile_list.get_mut(hex_position).unwrap();
                        tile.terrain_features.push(ruleset.terrains["Ice"].clone());
                    }
                }
            }
            /* **********the end of add ice********** */
            /* **********start to add flood plains********** */
            else {
                num_land_plots += 1;
                if self
                    .tile_edge_direction()
                    .iter()
                    .any(|&direction| tile.has_river(direction, self))
                    && ruleset.terrains["Flood plains"]
                        .occurs_on
                        .contains(&tile.base_terrain.name)
                {
                    let tile = self.tile_list.get_mut(hex_position).unwrap();
                    tile.terrain_features
                        .push(ruleset.terrains["Flood plains"].clone());
                }
                /* **********the end of add flood plains********** */
                /* **********start to add oasis********** */
                else if ruleset.terrains["Oasis"]
                    .occurs_on
                    .contains(&tile.base_terrain.name)
                    && (oasis_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= oasis_max_percent
                    && self.random_number_generator.gen_range(0..4) == 1
                {
                    let tile = self.tile_list.get_mut(hex_position).unwrap();
                    tile.terrain_features
                        .push(ruleset.terrains["Oasis"].clone());
                    oasis_count += 1;
                }
            }
            /* **********the end of add oasis********** */

            let mut marsh = false;
            let mut jungle = false;

            /* **********start to add march********** */
            let tile = &self.tile_list[hex_position];
            if ruleset.terrains["Marsh"]
                .occurs_on
                .contains(&tile.base_terrain.name)
                && (marsh_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                    <= marsh_max_percent
            {
                let mut score = 300;

                let tile_neighbors = tile.tiles_neighbors(self);

                let a = tile_neighbors
                    .iter()
                    .filter(|x| {
                        x.terrain_features
                            .iter()
                            .any(|terrain| terrain.name == "Marsh")
                    })
                    .count();
                match a {
                    0 => (),
                    1 => score += 50,
                    2 | 3 => score += 150,
                    4 => score -= 50,
                    _ => score -= 200,
                };
                if self.random_number_generator.gen_range(0..300) <= score {
                    let tile = self.tile_list.get_mut(hex_position).unwrap();
                    tile.terrain_features
                        .push(ruleset.terrains["Marsh"].clone());
                    marsh_count += 1;
                    marsh = true;
                }
            };
            /* **********the end of add march********** */
            /* **********start to add jungle********** */
            if !marsh {
                let tile = &self.tile_list[hex_position];
                let hex_coord = Hex::from(tile.hex_position);
                let offset_coord = hex_coord.to_offset_coordinate(
                    self.map_parameters.offset,
                    self.map_parameters.hex_layout.orientation,
                );
                let [_x, y] = offset_coord.to_array();

                let latitude = ((height as f64 / 2. - y as f64) / (height as f64 / 2.)).abs();
                if ruleset.terrains["Jungle"]
                    .occurs_on
                    .contains(&tile.base_terrain.name)
                    && (jungle_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= jungle_max_percent
                    && (latitude >= jungle_bottom as f64 / 100.
                        && latitude <= jungle_top as f64 / 100.)
                {
                    let mut score = 300;

                    let tile_neighbors = tile.tiles_neighbors(self);

                    let a = tile_neighbors
                        .iter()
                        .filter(|x| {
                            x.terrain_features
                                .iter()
                                .any(|terrain| terrain.name == "Jungle")
                        })
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        let tile = self.tile_list.get_mut(hex_position).unwrap();
                        tile.terrain_features
                            .push(ruleset.terrains["Jungle"].clone());

                        if tile.terrain_features.iter().any(|x| x.name == "Hill")
                            && (tile.base_terrain.name == "Grassland"
                                || tile.base_terrain.name == "Plains")
                        {
                            tile.base_terrain = ruleset.terrains["Plains"].clone()
                        } else {
                            tile.base_terrain = ruleset.terrains["Plains"].clone();
                            tile.terrain_features.retain(|x| x.name != "Hill");
                        }

                        jungle_count += 1;
                        jungle = true;
                    }
                }
            }
            /* **********the end of add jungle********** */
            /* **********start to add forest********** */
            if !marsh && !jungle {
                let tile = &self.tile_list[hex_position];
                if ruleset.terrains["Forest"]
                    .occurs_on
                    .contains(&tile.base_terrain.name)
                    && (forest_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= forest_max_percent
                {
                    let mut score = 300;

                    let tile_neighbors = tile.tiles_neighbors(self);

                    let a = tile_neighbors
                        .iter()
                        .filter(|x| {
                            x.terrain_features
                                .iter()
                                .any(|terrain| terrain.name == "Forest")
                        })
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        let tile = self.tile_list.get_mut(hex_position).unwrap();
                        tile.terrain_features
                            .push(ruleset.terrains["Forest"].clone());
                        forest_count += 1;
                    }
                }
            }
            /* **********the end of add jungle********** */
        }
    }

    pub fn natural_wonder_generator(&mut self, ruleset: &Res<Ruleset>) {
        let mut natural_wonder_list: Vec<_> = ruleset
            .terrains
            .iter()
            .filter_map(|(name, x)| { x.r#type == "NaturalWonder" }.then_some(name))
            .collect();

        /* The order of natural_wonder_list is random, so we should arrange this list in order
        to ensure that the obtained Vec is the same every time. */
        natural_wonder_list.sort_unstable();

        natural_wonder_list.shuffle(&mut self.random_number_generator);

        let mut random_number_generator = self.random_number_generator.clone();

        let mut natural_wonder_position_and_score = HashMap::new();

        let mut land_id_and_tile_count: Vec<_> = self
            .tile_list
            .values()
            .filter(|tile| tile.is_land() && !tile.base_terrain.impassable)
            .fold(HashMap::new(), |mut map, tile| {
                *map.entry(tile.area_id).or_insert(0) += 1;
                map
            })
            .into_iter()
            .collect();

        land_id_and_tile_count.sort_by_key(|&(_, v)| std::cmp::Reverse(v));

        fn matches_wonder_filter(tile: &Tile, filter: &str) -> bool {
            match filter {
                "Elevated" => tile.is_mountain() || tile.is_hill(),
                "Water" => tile.is_water(),
                "Land" => tile.is_land(),
                "Hill" => tile.is_hill(),
                //naturalWonder -> true
                //in allTerrainFeatures -> lastTerrain.name == filter
                _ => tile.base_terrain.name == filter,
            }
        }

        for (&hex_position, tile) in &self.tile_list {
            for &natural_wonder_name in &natural_wonder_list {
                let possible_natural_wonder = &ruleset.terrains[natural_wonder_name];

                // check unique conditions, this code is the same as the below, need refactoring
                let check_unique_conditions =
                    possible_natural_wonder.uniques.iter().all(|unique| {
                        let unique = Unique::new(unique);
                        match unique.placeholder_text.as_str() {
                            "Must be adjacent to [] [] tiles" => {
                                let count = tile
                                    .tiles_neighbors(self)
                                    .iter()
                                    .filter(|x| matches_wonder_filter(x, unique.params[1].as_str()))
                                    .count();
                                count == unique.params[0].parse::<usize>().unwrap()
                            }
                            "Must be adjacent to [] to [] [] tiles" => {
                                let count = tile
                                    .tiles_neighbors(self)
                                    .iter()
                                    .filter(|x| matches_wonder_filter(x, unique.params[2].as_str()))
                                    .count();
                                count >= unique.params[0].parse::<usize>().unwrap()
                                    && count <= unique.params[1].parse::<usize>().unwrap()
                            }
                            "Must not be on [] largest landmasses" => {
                                let index = unique.params[0].parse::<usize>().unwrap();
                                !land_id_and_tile_count
                                    .iter()
                                    .take(index)
                                    .any(|(id, _)| tile.area_id == *id)
                            }
                            "Must be on [] largest landmasses" => {
                                let index = unique.params[0].parse::<usize>().unwrap();
                                land_id_and_tile_count
                                    .iter()
                                    .take(index)
                                    .any(|(id, _)| tile.area_id == *id)
                            }
                            /* "Occurs on latitudes from [] to [] percent of distance equator to pole"=>{

                            } */
                            /* "Occurs in groups of [] to [] tiles"=>{

                            } */
                            _ => true,
                        }
                    });
                // end check unique conditions

                if possible_natural_wonder
                    .occurs_on
                    .contains(&tile.base_terrain.name)
                    && check_unique_conditions
                {
                    natural_wonder_position_and_score
                        .entry(natural_wonder_name)
                        .or_insert_with(Vec::new)
                        .push((hex_position, 1));
                }
            }
        }

        let mut j = 0;

        let selected_natural_wonder_list: Vec<_> =
            natural_wonder_position_and_score.keys().cloned().collect();

        let mut placed_natural_wonder_position = Vec::new();

        // start to place wonder
        for &natural_wonder_name in &selected_natural_wonder_list {
            if j <= self.map_parameters.natural_wonder_num {
                // for every natural wonder, give a score to the position where the natural wonder can place
                // the score is related to the min value of the distance from the position to all the placed natural wonders
                // if no natural wonder has placed, we choose the random place where the current natural wonder can place for the current natural wonder
                // the score method start
                let position_and_score = natural_wonder_position_and_score
                    .get_mut(natural_wonder_name)
                    .unwrap();
                for (x, score) in position_and_score.iter_mut() {
                    let closest_natural_wonder_dist = if placed_natural_wonder_position.is_empty() {
                        1000000
                    } else {
                        placed_natural_wonder_position
                            .iter()
                            .map(|y| Hex::hex_distance(Hex::from(*x), Hex::from(*y)))
                            .min()
                            .unwrap()
                    };
                    *score = if closest_natural_wonder_dist <= 10 {
                        100 * closest_natural_wonder_dist
                    } else {
                        1000 + (closest_natural_wonder_dist - 10)
                    } + random_number_generator.gen_range(0..100);
                }
                // the score method end

                // choose the max score position as the candidate position for the current natural wonder
                let max_score_position = position_and_score
                    .iter()
                    .max_by_key(|&(_, score)| score)
                    .map(|&(position, _)| position)
                    .unwrap();
                let tile = &self.tile_list[&max_score_position];
                let natural_wonder = &ruleset.terrains[natural_wonder_name];

                // check unique conditions, this code is the same as the above, need refactoring
                let check_unique_conditions = natural_wonder.uniques.iter().all(|unique| {
                    let unique = Unique::new(unique);
                    match unique.placeholder_text.as_str() {
                        "Must be adjacent to [] [] tiles" => {
                            let count = tile
                                .tiles_neighbors(self)
                                .iter()
                                .filter(|x| matches_wonder_filter(x, unique.params[1].as_str()))
                                .count();
                            count == unique.params[0].parse::<usize>().unwrap()
                        }
                        "Must be adjacent to [] to [] [] tiles" => {
                            let count = tile
                                .tiles_neighbors(self)
                                .iter()
                                .filter(|x| matches_wonder_filter(x, unique.params[2].as_str()))
                                .count();
                            count >= unique.params[0].parse::<usize>().unwrap()
                                && count <= unique.params[1].parse::<usize>().unwrap()
                        }
                        "Must not be on [] largest landmasses" => {
                            let index = unique.params[0].parse::<usize>().unwrap();
                            !land_id_and_tile_count
                                .iter()
                                .take(index)
                                .any(|(id, _)| tile.area_id == *id)
                        }
                        "Must be on [] largest landmasses" => {
                            let index = unique.params[0].parse::<usize>().unwrap();
                            land_id_and_tile_count
                                .iter()
                                .take(index)
                                .any(|(id, _)| tile.area_id == *id)
                        }
                        /* "Occurs on latitudes from [] to [] percent of distance equator to pole"=>{

                        } */
                        /* "Occurs in groups of [] to [] tiles"=>{

                        } */
                        _ => true,
                    }
                });
                // end check unique conditions

                if natural_wonder.occurs_on.contains(&tile.base_terrain.name)
                    && check_unique_conditions
                {
                    let tile = self.tile_list.get_mut(&max_score_position).unwrap();
                    tile.terrain_features.clear();
                    tile.terrain_features.push(natural_wonder.clone());
                    let turn_into_terrain_name = &natural_wonder.turns_into;
                    tile.base_terrain = ruleset.terrains[turn_into_terrain_name].clone();
                    placed_natural_wonder_position.push(max_score_position);
                    j += 1;
                }
            }
        }
        self.random_number_generator = random_number_generator;
    }
}
