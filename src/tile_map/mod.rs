use std::cmp::max;
use std::collections::VecDeque;

use bevy::math::IVec2;
use bevy::utils::HashSet;
use bevy::{math::DVec2, prelude::Res, utils::HashMap};
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::grid::hex::{Hex, HexLayout, HexOrientation, Offset, OffsetCoordinate};
use crate::grid::Direction;
use crate::map::base_terrain::BaseTerrain;
use crate::map::feature::Feature;
use crate::map::natural_wonder::NaturalWonder;
use crate::map::terrain_type::TerrainType;
use crate::ruleset::{self, Ruleset, Unique};
mod fractal;
mod map_parameters;
mod tile;

pub use self::fractal::{CvFractal, Flags};
pub use self::tile::Tile;
pub use map_parameters::*;

pub struct TileMap {
    //pub map_parameters: MapParameters,
    pub random_number_generator: StdRng,
    pub tile_list: Vec<Tile>,
    pub river_list: HashMap<i32, Vec<(usize, Direction)>>,
}

impl TileMap {
    pub fn new(map_parameters: &Res<MapParameters>) -> Self {
        let random_number_generator = StdRng::seed_from_u64(map_parameters.seed);
        let tile_list = Self::rectangular_map(
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            map_parameters.hex_layout,
            map_parameters.offset,
        );
        Self {
            //map_parameters,
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
    ) -> Vec<Tile> {
        let mut tile_list = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let offset_coordinate = OffsetCoordinate::new(x, y);
                let hex_coordinate = offset_coordinate.to_hex(offset, hex_layout.orientation);

                let tile = Tile::new(hex_coordinate.to_array());
                tile_list.push(tile);
            }
        }
        tile_list
    }

    pub const fn index_to_offset_coordinate(map_size: MapSize, index: usize) -> OffsetCoordinate {
        assert!(index < (map_size.width * map_size.height) as usize);
        let x = index as i32 % map_size.width;
        let y = index as i32 / map_size.width;
        OffsetCoordinate::new(x, y)
    }

    pub const fn offset_coordinate_to_index(
        map_size: MapSize,
        offset_coordinate: OffsetCoordinate,
    ) -> usize {
        let [x, y] = offset_coordinate.to_array();
        assert!((x >= 0) && (x < map_size.width) && (y >= 0) && (y < map_size.height));
        (x + y * map_size.width) as usize
    }

    /// Calculates the latitude of the tile on the tile map.
    ///
    /// Define that the latitude of the equator is `0.0` and the latitudes of the poles are `1.0`.
    /// The closer the latitude is to `0.0`, the closer the tile is to the equator; the closer the latitude is to `1.0`, the closer the tile is to the poles.
    pub fn tile_latitude(map_size: MapSize, index: usize) -> f64 {
        let [_x, y] = Self::index_to_offset_coordinate(map_size, index).to_array();
        ((map_size.height as f64 / 2. - y as f64) / (map_size.height as f64 / 2.)).abs()
    }

    pub fn tile_edge_direction(&self, map_parameters: &MapParameters) -> [Direction; 6] {
        map_parameters.hex_layout.orientation.edge_direction()
    }

    pub fn tile_corner_direction(&self, map_parameters: &MapParameters) -> [Direction; 6] {
        map_parameters.hex_layout.orientation.corner_direction()
    }

    pub fn spawn_tile_type_for_fractal(&mut self, map_parameters: &Res<MapParameters>) {
        let continent_grain = 2;

        let sea_level_low = 65;
        let sea_level_normal = 72;
        let sea_level_high = 78;
        let world_age_old = 2;
        let world_age_normal = 3;
        let world_age_new = 5;

        let extra_mountains = 0;

        let tectonic_islands = false;

        let adjustment = match map_parameters.world_age {
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

        let water_percent = match map_parameters.sea_level {
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

        let orientation = map_parameters.hex_layout.orientation;
        let offset = map_parameters.offset;

        let mut continents_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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

        self.tile_list
            .iter_mut()
            .enumerate()
            .for_each(|(index, tile)| {
                let [x, y] =
                    Self::index_to_offset_coordinate(map_parameters.map_size, index).to_array();
                let height = continents_fractal.get_height(x, y);

                let mountain_height = mountains_fractal.get_height(x, y);
                let hill_height = hills_fractal.get_height(x, y);

                if height <= water_threshold {
                    tile.terrain_type = TerrainType::Water;
                    if tectonic_islands {
                        if mountain_height == mountain_100 {
                            tile.terrain_type = TerrainType::Mountain;
                        } else if mountain_height == mountain_99 {
                            tile.terrain_type = TerrainType::Hill;
                        } else if (mountain_height == mountain_97)
                            || (mountain_height == mountain_95)
                        {
                            tile.terrain_type = TerrainType::Flatland;
                        }
                    }
                } else if mountain_height >= mountain_threshold {
                    if hill_height >= pass_threshold {
                        tile.terrain_type = TerrainType::Hill;
                    } else {
                        tile.terrain_type = TerrainType::Mountain;
                    }
                } else if mountain_height >= hills_near_mountains
                    || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                    || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
                {
                    tile.terrain_type = TerrainType::Hill;
                } else {
                    tile.terrain_type = TerrainType::Flatland;
                };
            });
    }

    pub fn spawn_tile_type_for_pangaea(&mut self, map_parameters: &Res<MapParameters>) {
        let continent_grain = 2;

        let sea_level_low = 71;
        let sea_level_normal = 78;
        let sea_level_high = 84;
        let world_age_old = 2;
        let world_age_normal = 3;
        let world_age_new = 5;

        let extra_mountains = 0;

        let adjustment = match map_parameters.world_age {
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

        let water_percent = match map_parameters.sea_level {
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

        let orientation = map_parameters.hex_layout.orientation;
        let offset = map_parameters.offset;

        let mut continents_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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

        let width = map_parameters.map_size.width;
        let height = map_parameters.map_size.height;
        let center_position = DVec2::new(width as f64 / 2., height as f64 / 2.);

        let axis = center_position * 3. / 5.;

        self.tile_list
            .iter_mut()
            .enumerate()
            .for_each(|(index, tile)| {
                let [x, y] =
                    Self::index_to_offset_coordinate(map_parameters.map_size, index).to_array();
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
                    tile.terrain_type = TerrainType::Water;
                    if height == mountain_100 {
                        tile.terrain_type = TerrainType::Mountain;
                    } else if height == mountain_99 {
                        tile.terrain_type = TerrainType::Hill;
                    } else if height == mountain_97 || height == mountain_95 {
                        tile.terrain_type = TerrainType::Flatland;
                    }
                } else if mountain_height >= mountain_threshold {
                    if hill_height >= pass_threshold {
                        tile.terrain_type = TerrainType::Hill;
                    } else {
                        tile.terrain_type = TerrainType::Mountain;
                    }
                } else if mountain_height >= hills_near_mountains
                    || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                    || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
                {
                    tile.terrain_type = TerrainType::Hill;
                } else {
                    tile.terrain_type = TerrainType::Flatland;
                };
            });
    }

    pub fn generate_coasts(&mut self, ruleset: &Res<Ruleset>, map_parameters: &Res<MapParameters>) {
        for index in 0..self.tile_list.len() {
            let tile = &self.tile_list[index];
            if tile.is_water()
                && tile
                    .tile_neighbors(self, map_parameters)
                    .iter()
                    .any(|neigbor_tile| !neigbor_tile.is_water())
            {
                self.tile_list[index].base_terrain = BaseTerrain::Coast;
            }
        }

        for chance in &map_parameters.coast_expand_chance {
            let mut expansion_index = Vec::new();
            /* Don't update the base_terrain of the tile in the iteration.
            Because if we update the base_terrain of the tile in the iteration,
            the tile will be used in the next iteration(e.g. tile.tile_neighbors().iter().any()),
            which will cause the result to be wrong. */
            for index in 0..self.tile_list.len() {
                let tile = &self.tile_list[index];
                if tile.is_water()
                    && tile.base_terrain != BaseTerrain::Coast
                    && tile
                        .tile_neighbors(self, map_parameters)
                        .iter()
                        .any(|tile| tile.base_terrain == BaseTerrain::Coast)
                    && self.random_number_generator.gen_bool(*chance)
                {
                    expansion_index.push(index);
                }
            }

            for index in expansion_index {
                self.tile_list[index].base_terrain = BaseTerrain::Coast;
            }
        }
    }

    /// This fun is used when we create the world and some water areas surrounded by land.
    /// In original Civ, the random world create by Voronoi Noise doesn't have situation, so the fun
    /// is only used when we create the world by ourselves. But in our code, we should tackle with this
    /// situation.
    pub fn generate_lakes(&mut self, ruleset: &Res<Ruleset>, map_parameters: &Res<MapParameters>) {
        self.recalculate_areas(ruleset, map_parameters);

        // Get the Vec of area_id when water_area_size is smaller than lake_max_area_size
        let candidate_water_area_ids: Vec<i32> = self
            .tile_list
            .iter()
            .filter(|tile| tile.is_water())
            .fold(HashMap::new(), |mut water_area_ids_and_size, tile| {
                // Get a HashMap of water area id and its size
                *water_area_ids_and_size.entry(tile.area_id).or_insert(0) += 1;
                water_area_ids_and_size
            })
            .into_iter()
            .filter_map(|(area_id, water_area_size)| {
                // Get area_id when water_area_size is smaller than lake_max_area_size
                (water_area_size <= map_parameters.lake_max_area_size).then_some(area_id)
            })
            .collect();

        for tile in self
            .tile_list
            .iter_mut()
            .filter(|tile| candidate_water_area_ids.contains(&tile.area_id))
        {
            tile.base_terrain = BaseTerrain::Lake;
        }
    }

    pub fn generate_terrain(
        &mut self,
        ruleset: &Res<Ruleset>,
        map_parameters: &Res<MapParameters>,
    ) {
        let temperature_shift = 0.1;
        let desert_shift = 16;
        let mut desert_percent = 32;
        let plains_percent = 50;
        let mut snow_latitude = 0.75;
        let mut tundra_latitude = 0.6;
        let mut grass_latitude = 0.1;
        let desert_bottom_latitude = 0.2;
        let mut desert_top_latitude = 0.5;

        match map_parameters.temperature {
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
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            3,
            Flags::default(),
            -1,
            -1,
        );
        let deserts_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            3,
            Flags::default(),
            -1,
            -1,
        );
        let plains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
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

        self.tile_list
            .iter_mut()
            .enumerate()
            .filter(|(_, tile)| tile.terrain_type != TerrainType::Water)
            .for_each(|(index, tile)| {
                let [x, y] =
                    Self::index_to_offset_coordinate(map_parameters.map_size, index).to_array();

                tile.base_terrain = BaseTerrain::Grassland;

                let deserts_height = deserts_fractal.get_height(x, y);
                let plains_height = plains_fractal.get_height(x, y);

                let mut latitude = Self::tile_latitude(map_parameters.map_size, index);
                latitude += (128 - variation_fractal.get_height(x, y)) as f64 / (255.0 * 5.0);
                latitude = latitude.clamp(0., 1.);

                if latitude >= snow_latitude {
                    tile.base_terrain = BaseTerrain::Snow
                } else if latitude >= tundra_latitude {
                    tile.base_terrain = BaseTerrain::Tundra;
                } else if latitude < grass_latitude {
                    tile.base_terrain = BaseTerrain::Grassland;
                } else if deserts_height >= desert_bottom
                    && deserts_height <= desert_top
                    && latitude >= desert_bottom_latitude
                    && latitude < desert_top_latitude
                {
                    tile.base_terrain = BaseTerrain::Desert;
                } else if plains_height >= plains_bottom && plains_height <= plains_top {
                    tile.base_terrain = BaseTerrain::Plain;
                }
            });
    }

    pub fn recalculate_areas(&mut self, ruleset: &Res<Ruleset>, map_parameters: &MapParameters) {
        // area id of all the tiles is set to default value (-1)
        self.tile_list.iter_mut().for_each(|tile| tile.area_id = -1);
        // water area, excluding impassable tile ( e.g. ice, natural-wonder in water)
        self.bfs(
            |tile| tile.is_water() && !tile.is_impassable(ruleset),
            map_parameters,
        );
        // mountain area
        self.bfs(|tile| tile.is_mountain(), map_parameters);
        // other land area (including flatland and hill, excluding natural-wonder and mountain)
        self.bfs(
            |tile| (tile.is_flatland() || tile.is_hill()) && tile.natural_wonder.is_none(),
            map_parameters,
        );
        // impassable area (including ice and natural-wonder, excluding mountain)
        self.bfs(
            |tile| tile.feature == Some(Feature::Ice) || tile.natural_wonder.is_some(),
            map_parameters,
        );
    }

    fn bfs(&mut self, filter_condition: impl Fn(&Tile) -> bool, map_parameters: &MapParameters) {
        let mut area_tiles_indices: HashSet<_> = self
            .tile_list
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| filter_condition(tile).then_some(index))
            .collect();
        let mut current_area_id = self
            .tile_list
            .iter()
            .map(|tile| tile.area_id)
            .max()
            .unwrap()
            + 1;
        while let Some(&initial_area_tile_index) = area_tiles_indices.iter().next() {
            area_tiles_indices.remove(&initial_area_tile_index);
            let mut tiles_in_current_area_indices = HashSet::new();
            tiles_in_current_area_indices.insert(initial_area_tile_index);
            self.tile_list[initial_area_tile_index].area_id = current_area_id;
            let mut tiles_to_check_indices = VecDeque::new();
            tiles_to_check_indices.push_back(initial_area_tile_index);
            while let Some(tile_we_are_checking_position) = tiles_to_check_indices.pop_front() {
                let neighbors_tiles_indices: Vec<_> = self.tile_list[tile_we_are_checking_position]
                    .tile_neighbors(self, map_parameters)
                    .iter()
                    .filter_map(|tile| {
                        {
                            !tiles_in_current_area_indices.contains(&tile.index(map_parameters))
                                && filter_condition(tile)
                        }
                        .then_some(tile.index(map_parameters))
                    })
                    .collect();
                for &index in neighbors_tiles_indices.iter() {
                    tiles_in_current_area_indices.insert(index);
                    self.tile_list[index].area_id = current_area_id;
                    tiles_to_check_indices.push_back(index);
                    area_tiles_indices.remove(&index);
                }
            }
            current_area_id += 1;
        }
    }

    pub fn add_rivers(&mut self, ruleset: &Res<Ruleset>, map_parameters: &Res<MapParameters>) {
        let river_source_range_default = 4;
        let sea_water_range_default = 3;
        const plots_per_river_edge: i32 = 12;

        fn pass_conditions(
            tile: &Tile,
            tile_map: &TileMap,
            random_number_generator: &mut StdRng,
            map_parameters: &MapParameters,
        ) -> [bool; 4] {
            let num_tiles = tile_map
                .tile_list
                .iter()
                .filter(|x| x.area_id == tile.area_id)
                .count() as i32;
            let num_river_edges = num_river_edges(tile, tile_map, map_parameters);
            [
                tile.is_hill() || tile.is_mountain(),
                tile.is_coastal_land(tile_map, map_parameters)
                    && random_number_generator.gen_range(0..8) == 0,
                (tile.is_hill() || tile.is_mountain())
                    && (num_river_edges < num_tiles / plots_per_river_edge + 1),
                num_river_edges < num_tiles / plots_per_river_edge + 1,
            ]
        }

        // Returns the number of river edges in the area where the tile is
        // 1. Get the area where the tile is
        // 2. Get the number of rivers edge which the area (where the tile is) own
        fn num_river_edges(tile: &Tile, tile_map: &TileMap, map_parameters: &MapParameters) -> i32 {
            let mut num_river_edges = 0;
            tile_map
                .tile_list
                .iter()
                .filter(|x| x.area_id == tile.area_id)
                .for_each(|tile| {
                    tile_map.river_list.values().for_each(|river_plot| {
                        num_river_edges = river_plot
                            .iter()
                            .filter(|(tile_index, _)| tile_index == &tile.index(map_parameters))
                            .count();
                    });
                });
            num_river_edges as i32
        }

        let mut random_number_generator = self.random_number_generator.clone();

        // The tile where the river will start shoult meet these conditions:
        // 1. It should be not a water tile
        // 2. It should be not a natural wonder
        // 3. It should be not a tile which is neighbor to a natural wonder
        // 4. Its edge directions in [0..3] should be not water because the river edge uses (tile_index, river_flow_direction) for storage.
        //    tile_index is current tile index and river_flow_direction should be one of the edge directions in [0..3].
        let candidate_start_tile_indices: Vec<_> = self
            .tile_list
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                {
                    !tile.is_water()
                        && !tile.is_natural_wonder()
                        && !tile
                            .tile_neighbors(self, map_parameters)
                            .iter()
                            .any(|neighbor_tile| neighbor_tile.is_natural_wonder())
                        && self.tile_edge_direction(map_parameters)[0..3]
                            .iter()
                            .all(|&direction| {
                                if let Some(neighbor_tile) =
                                    tile.tile_neighbor(self, direction, map_parameters)
                                {
                                    !neighbor_tile.is_water()
                                        && !neighbor_tile.is_natural_wonder()
                                        && !neighbor_tile
                                            .tile_neighbors(self, map_parameters)
                                            .iter()
                                            .any(|neighbor_tile| neighbor_tile.is_natural_wonder())
                                } else {
                                    false
                                }
                            })
                }
                .then_some(index)
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

            for &tile_index in candidate_start_tile_indices.iter() {
                let tile = &self.tile_list[tile_index];
                if pass_conditions(tile, self, &mut random_number_generator, map_parameters)[index]
                    && !tile
                        .tiles_in_distance(river_source_range, self, map_parameters)
                        .iter()
                        .any(|tile| tile.is_freshwater(self, map_parameters))
                    && !tile
                        .tiles_in_distance(sea_water_range, self, map_parameters)
                        .iter()
                        .any(|tile| tile.is_water())
                {
                    self.do_river(
                        tile_index,
                        Direction::None,
                        Direction::None,
                        river_id,
                        &ruleset,
                        &map_parameters,
                    );
                    river_id += 1;
                }
            }
        });
        self.random_number_generator = random_number_generator;
    }

    fn do_river(
        &mut self,
        start_plot_index: usize,
        this_flow_direction: Direction,
        original_flow_direction: Direction,
        river_id: i32,
        ruleset: &Ruleset,
        map_parameters: &MapParameters,
    ) {
        // if the start plot have a river, exit the function
        if self.river_list.values().any(|river| {
            river
                .iter()
                .any(|(tile_index, _)| *tile_index == start_plot_index)
        }) && original_flow_direction == Direction::None
        {
            return;
        }

        let mut original_flow_direction = original_flow_direction;

        let mut river_plot_index;
        let mut best_flow_direction = Direction::None;
        match map_parameters.hex_layout.orientation {
            HexOrientation::Pointy => match this_flow_direction {
                Direction::North => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::SouthEast, self, map_parameters)
                            || neighbor_tile.has_river(Direction::SouthWest, self, map_parameters)
                        {
                            return;
                        } else {
                            river_plot_index = neighbor_tile.index(map_parameters);
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthEast => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::East, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::East, self, map_parameters)
                            || neighbor_tile.has_river(Direction::SouthWest, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::East => unreachable!(),
                Direction::SouthEast => {
                    let start_tile = &self.tile_list[start_plot_index];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::East, map_parameters)
                    {
                        river_plot_index = neighbor_tile.index(map_parameters)
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest, map_parameters)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::East, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::South => {
                    let start_tile = &self.tile_list[start_plot_index];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::SouthWest, map_parameters)
                    {
                        river_plot_index = neighbor_tile.index(map_parameters)
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::East, map_parameters)
                    {
                        if neighbor_tile2.has_river(Direction::SouthWest, self, map_parameters) {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthWest => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::East, self, map_parameters)
                            || river_plot_tile.has_river(Direction::SouthWest, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::West => unreachable!(),
                Direction::NorthWest => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::West, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::East, self, map_parameters)
                            || neighbor_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        } else {
                            river_plot_index = neighbor_tile.index(map_parameters);
                        }
                    } else {
                        return;
                    }
                }
                Direction::None => {
                    river_plot_index = start_plot_index;
                }
            },
            HexOrientation::Flat => match this_flow_direction {
                Direction::North => unreachable!(),
                Direction::NorthEast => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::NorthEast, self, map_parameters)
                            || neighbor_tile.has_river(Direction::South, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::East => {
                    let start_tile = &self.tile_list[start_plot_index];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::NorthEast, map_parameters)
                    {
                        river_plot_index = neighbor_tile.index(map_parameters)
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::South, map_parameters)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::NorthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::SouthEast => {
                    let start_tile = &self.tile_list[start_plot_index];
                    if let Some(neighbor_tile) =
                        start_tile.tile_neighbor(self, Direction::South, map_parameters)
                    {
                        river_plot_index = neighbor_tile.index(map_parameters)
                    } else {
                        return;
                    };
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthEast, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                    if let Some(neighbor_tile2) =
                        river_plot_tile.tile_neighbor(self, Direction::NorthEast, map_parameters)
                    {
                        if neighbor_tile2.is_water()
                            || neighbor_tile2.has_river(Direction::South, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::South => unreachable!(),
                Direction::SouthWest => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::South, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || river_plot_tile.has_river(Direction::South, self, map_parameters)
                            || neighbor_tile.has_river(Direction::NorthEast, self, map_parameters)
                        {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                Direction::West => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::SouthWest, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::NorthEast, self, map_parameters)
                            || neighbor_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        } else {
                            river_plot_index = neighbor_tile.index(map_parameters);
                        }
                    } else {
                        return;
                    }
                }
                Direction::NorthWest => {
                    river_plot_index = start_plot_index;
                    self.river_list
                        .entry(river_id)
                        .or_default()
                        .push((river_plot_index, this_flow_direction));
                    let river_plot_tile = &self.tile_list[river_plot_index];
                    if let Some(neighbor_tile) =
                        river_plot_tile.tile_neighbor(self, Direction::North, map_parameters)
                    {
                        if neighbor_tile.is_water()
                            || neighbor_tile.has_river(Direction::South, self, map_parameters)
                            || neighbor_tile.has_river(Direction::SouthEast, self, map_parameters)
                        {
                            return;
                        } else {
                            river_plot_index = neighbor_tile.index(map_parameters);
                        }
                    } else {
                        return;
                    }
                }
                Direction::None => {
                    river_plot_index = start_plot_index;
                }
            },
        }

        let river_plot_tile = &self.tile_list[river_plot_index];
        if river_plot_tile.is_water() {
            return;
        }

        // In this tuple, The first element is next possible flow, the second element is the direction of the special plot relative to current plot
        // We evaluate the weight value of the special plot using a certain algorithm and select the minimum one to determine the next direction of the river flow
        let adjacent_plot_directions = match map_parameters.hex_layout.orientation {
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

        fn next_flow_directions(
            flow_direction: Direction,
            tile_map: &TileMap,
            map_parameters: &MapParameters,
        ) -> [Direction; 2] {
            let direction_array = tile_map.tile_corner_direction(map_parameters);
            let flow_direction_index = map_parameters
                .hex_layout
                .orientation
                .corner_index(flow_direction);
            [
                direction_array[(flow_direction_index + 1) % 6], // turn_right_flow_direction
                direction_array[(flow_direction_index + 5) % 6], // turn_left_flow_direction
            ]
        }

        fn river_value_at_plot(
            plot_index: usize,
            tile_map: &mut TileMap,
            map_parameters: &MapParameters,
        ) -> i32 {
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

            let tile = &tile_map.tile_list[plot_index];

            if tile.is_natural_wonder()
                || tile
                    .tile_neighbors(tile_map, map_parameters)
                    .iter()
                    .any(|neighbor_tile| neighbor_tile.is_natural_wonder())
            {
                return -1;
            }

            let mut sum = plot_elevation(tile) * 20;
            let direction_array = tile_map.tile_edge_direction(map_parameters);
            direction_array.iter().for_each(|&direction| {
                if let Some(adjacent_tile) = tile.tile_neighbor(tile_map, direction, map_parameters)
                {
                    sum += plot_elevation(adjacent_tile);
                    if adjacent_tile.base_terrain == BaseTerrain::Desert {
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
                    .tile_neighbor(self, direction, map_parameters)
                    .map(|neighbor_tile| (flow_direction, neighbor_tile.index(map_parameters)))
            })
            .collect::<Vec<_>>();

        if best_flow_direction == Direction::None {
            let mut best_value = i32::MAX;
            for (flow_direction, adjacent_plot) in adjacent_plot_list.into_iter() {
                if flow_direction.opposite_direction() != original_flow_direction
                    && (this_flow_direction == Direction::None
                        || next_flow_directions(this_flow_direction, self, map_parameters)
                            .contains(&flow_direction))
                {
                    let mut value = river_value_at_plot(adjacent_plot, self, map_parameters);
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

        if best_flow_direction != Direction::None {
            if original_flow_direction == Direction::None {
                original_flow_direction = best_flow_direction;
            }
            self.do_river(
                river_plot_index,
                best_flow_direction,
                original_flow_direction,
                river_id,
                ruleset,
                map_parameters,
            )
        }
    }

    pub fn add_lakes(&mut self, ruleset: &Res<Ruleset>, map_parameters: &MapParameters) {
        let large_lake_num = map_parameters.large_lake_num;

        let mut num_lakes_added = 0;
        let mut num_large_lakes_added = 0;
        let lake_plot_rand = 25;
        let direction_array = self.tile_edge_direction(map_parameters);

        for tile_index in 0..self.tile_list.len() {
            let tile = &self.tile_list[tile_index];
            if !tile.is_water()
                && !tile.is_coastal_land(self, map_parameters)
                && !direction_array
                    .iter()
                    .any(|&direction| tile.has_river(direction, self, map_parameters))
                && !tile
                    .tile_neighbors(self, map_parameters)
                    .iter()
                    .any(|neighbor_tile| neighbor_tile.is_natural_wonder())
                && self.random_number_generator.gen_range(0..lake_plot_rand) == 0
            {
                num_lakes_added += 1;
                if num_large_lakes_added < large_lake_num {
                    let add_more_lakes = self.add_more_lake(tile_index, map_parameters);
                    if add_more_lakes {
                        num_large_lakes_added += 1;
                    }
                }
                let tile = &mut self.tile_list[tile_index];
                tile.terrain_type = TerrainType::Water;
                tile.base_terrain = BaseTerrain::Lake;
                tile.feature = None;
                tile.natural_wonder = None;
            }
        }
        if num_lakes_added > 0 {
            self.recalculate_areas(ruleset, map_parameters)
        }
    }

    fn add_more_lake(&mut self, tile_index: usize, map_parameters: &MapParameters) -> bool {
        let mut large_lake = 0;
        let mut lake_plots = Vec::new();
        let tile = &self.tile_list[tile_index];
        for &direction in self.tile_edge_direction(map_parameters).iter() {
            let neighbor_tile = tile.tile_neighbor(self, direction, map_parameters);
            if let Some(neighbor_tile) = neighbor_tile {
                if !neighbor_tile.is_water()
                    && !neighbor_tile.is_coastal_land(self, map_parameters)
                    && !self
                        .tile_edge_direction(map_parameters)
                        .iter()
                        .any(|&direction| neighbor_tile.has_river(direction, self, map_parameters))
                    && !neighbor_tile
                        .tile_neighbors(self, map_parameters)
                        .iter()
                        .any(|neighbor_tile| neighbor_tile.is_natural_wonder())
                {
                    let tile_index = neighbor_tile.index(map_parameters);
                    if self.random_number_generator.gen_range(0..(large_lake + 4)) < 3 {
                        lake_plots.push(tile_index);
                        large_lake += 1;
                    }
                }
            }
        }

        for &lake_plot in lake_plots.iter() {
            let tile = &mut self.tile_list[lake_plot];
            tile.terrain_type = TerrainType::Water;
            tile.base_terrain = BaseTerrain::Lake;
            tile.feature = None;
            tile.natural_wonder = None;
        }

        large_lake > 2
    }

    pub fn add_features(&mut self, ruleset: &Res<Ruleset>, map_parameters: &MapParameters) {
        let rainfall = match map_parameters.rainfall {
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

        for tile_index in 0..self.tile_list.len() {
            let tile = &self.tile_list[tile_index];

            /* **********start to add ice********** */
            if tile.is_impassable(ruleset) {
                continue;
            } else if tile.is_water() {
                if !self
                    .tile_edge_direction(map_parameters)
                    .iter()
                    .any(|&direction| tile.has_river(direction, self, map_parameters))
                    && ruleset.features["Ice"]
                        .occurs_on_base
                        .contains(&tile.base_terrain)
                {
                    let latitude = Self::tile_latitude(map_parameters.map_size, tile_index);

                    if latitude > 0.78 {
                        let mut score = self.random_number_generator.gen_range(0..100) as f64;
                        score += latitude * 100.;
                        let tile_neighbors = tile.tile_neighbors(self, map_parameters);
                        if tile_neighbors.iter().any(|tile| !tile.is_water()) {
                            score /= 2.0;
                        }
                        let a = tile_neighbors
                            .iter()
                            .filter(|tile| tile.feature == Some(Feature::Ice))
                            .count();
                        score += 10. * a as f64;
                        if score > 130. {
                            let tile = &mut self.tile_list[tile_index];
                            tile.feature = Some(Feature::Ice);
                        }
                    }
                }
            }
            /* **********the end of add ice********** */
            else {
                /* **********start to add Floodplain********** */
                num_land_plots += 1;
                if self
                    .tile_edge_direction(map_parameters)
                    .iter()
                    .any(|&direction| tile.has_river(direction, self, map_parameters))
                    && ruleset.features["Floodplain"]
                        .occurs_on_base
                        .contains(&tile.base_terrain)
                {
                    let tile = &mut self.tile_list[tile_index];
                    tile.feature = Some(Feature::Floodplain);
                    continue;
                }
                /* **********the end of add Floodplain********** */
                /* **********start to add oasis********** */
                else if ruleset.features["Oasis"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                    && (oasis_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= oasis_max_percent
                    && self.random_number_generator.gen_range(0..4) == 1
                {
                    let tile = &mut self.tile_list[tile_index];
                    tile.feature = Some(Feature::Oasis);
                    oasis_count += 1;
                    continue;
                }
                /* **********the end of add oasis********** */
                /* **********start to add march********** */
                if ruleset.features["Marsh"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                    && (marsh_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= marsh_max_percent
                {
                    let mut score = 300;

                    let tile_neighbors = tile.tile_neighbors(self, map_parameters);

                    let a = tile_neighbors
                        .iter()
                        .filter(|tile| tile.feature == Some(Feature::Marsh))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        let tile = &mut self.tile_list[tile_index];
                        tile.feature = Some(Feature::Marsh);
                        marsh_count += 1;
                        continue;
                    }
                };
                /* **********the end of add march********** */
                /* **********start to add jungle********** */
                let latitude = Self::tile_latitude(map_parameters.map_size, tile_index);

                if ruleset.features["Jungle"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                    && (jungle_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= jungle_max_percent
                    && (latitude >= jungle_bottom as f64 / 100.
                        && latitude <= jungle_top as f64 / 100.)
                {
                    let mut score = 300;

                    let tile_neighbors = tile.tile_neighbors(self, map_parameters);

                    let a = tile_neighbors
                        .iter()
                        .filter(|tile| tile.feature == Some(Feature::Jungle))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        let tile = &mut self.tile_list[tile_index];
                        tile.feature = Some(Feature::Jungle);

                        if tile.terrain_type == TerrainType::Hill
                            && (tile.base_terrain == BaseTerrain::Grassland
                                || tile.base_terrain == BaseTerrain::Plain)
                        {
                            tile.base_terrain = BaseTerrain::Plain;
                        } else {
                            tile.terrain_type = TerrainType::Flatland;
                            tile.base_terrain = BaseTerrain::Plain;
                        }

                        jungle_count += 1;
                        continue;
                    }
                }
                /* **********the end of add jungle********** */
                /* **********start to add forest********** */
                if ruleset.features["Forest"]
                    .occurs_on_base
                    .contains(&tile.base_terrain)
                    && (forest_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= forest_max_percent
                {
                    let mut score = 300;

                    let tile_neighbors = tile.tile_neighbors(self, map_parameters);

                    let a = tile_neighbors
                        .iter()
                        .filter(|tile| tile.feature == Some(Feature::Forest))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        let tile = &mut self.tile_list[tile_index];
                        tile.feature = Some(Feature::Forest);
                        forest_count += 1;
                        continue;
                    }
                }
                /* **********the end of add forest********** */
            }
        }
    }

    /// This function is used to generate natural wonders.
    pub fn natural_wonder_generator(
        &mut self,
        ruleset: &Res<Ruleset>,
        map_parameters: &MapParameters,
    ) {
        let natural_wonder_list: Vec<_> = ruleset.natural_wonders.keys().collect();

        let mut natural_wonder_and_tile_index_and_score = HashMap::new();

        let mut land_id_and_area_size: Vec<_> = self
            .tile_list
            .iter()
            .filter(|tile| tile.is_hill() || tile.is_flatland())
            .fold(HashMap::new(), |mut map, tile| {
                *map.entry(tile.area_id).or_insert(0) += 1;
                map
            })
            .into_iter()
            .collect();

        land_id_and_area_size.sort_by_key(|&(_, v)| std::cmp::Reverse(v));

        fn matches_wonder_filter(tile: &Tile, filter: &str) -> bool {
            match filter {
                "Elevated" => tile.is_mountain() || tile.is_hill(),
                _ => {
                    tile.terrain_type.name() == filter
                        || tile.base_terrain.name() == filter
                        || tile.feature.map_or(false, |f| f.name() == filter)
                }
            }
        }

        for (index, tile) in self.tile_list.iter().enumerate() {
            for &natural_wonder_name in &natural_wonder_list {
                let possible_natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                match natural_wonder_name.as_str() {
                    "Great Barrier Reef" => {
                        if let Some(adj_tile) = tile.tile_neighbor(
                            self,
                            self.tile_edge_direction(map_parameters)[1],
                            map_parameters,
                        ) {
                            let mut all_neigbor_indices = HashSet::new();

                            all_neigbor_indices.extend(
                                tile.tile_neighbors(self, map_parameters)
                                    .iter()
                                    .map(|tile| tile.index(map_parameters)),
                            );
                            all_neigbor_indices.extend(
                                adj_tile
                                    .tile_neighbors(self, map_parameters)
                                    .iter()
                                    .map(|tile| tile.index(map_parameters)),
                            );

                            all_neigbor_indices.remove(&tile.index(map_parameters));
                            all_neigbor_indices.remove(&adj_tile.index(map_parameters));

                            if all_neigbor_indices.len() == 8
                                && all_neigbor_indices.iter().all(|&index| {
                                    let tile = &self.tile_list[index];
                                    tile.terrain_type == TerrainType::Water
                                        && tile.base_terrain != BaseTerrain::Lake
                                        && tile.feature != Some(Feature::Ice)
                                })
                                && all_neigbor_indices
                                    .iter()
                                    .filter(|&index| {
                                        let tile = &self.tile_list[*index];
                                        tile.base_terrain == BaseTerrain::Coast
                                    })
                                    .count()
                                    >= 4
                            {
                                natural_wonder_and_tile_index_and_score
                                    .entry(natural_wonder_name)
                                    .or_insert_with(Vec::new)
                                    .push((index, 1));
                            }
                        }
                    }
                    "Rock of Gibraltar" => {
                        if ((tile.terrain_type == TerrainType::Water
                            && tile.base_terrain != BaseTerrain::Lake)
                            || (tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .any(|tile| {
                                    tile.terrain_type == TerrainType::Water
                                        && tile.base_terrain != BaseTerrain::Lake
                                })))
                            && tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .filter(|tile| tile.terrain_type != TerrainType::Water)
                                .count()
                                == 1
                            && tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .filter(|tile| tile.base_terrain == BaseTerrain::Coast)
                                .count()
                                >= 3
                        {
                            natural_wonder_and_tile_index_and_score
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push((index, 1));
                        }
                    }
                    _ => {
                        if tile.is_freshwater(self, map_parameters)
                            != possible_natural_wonder.is_fresh_water
                        {
                            continue;
                        };

                        if !possible_natural_wonder
                            .occurs_on_type
                            .contains(&tile.terrain_type)
                            || !possible_natural_wonder
                                .occurs_on_base
                                .contains(&tile.base_terrain)
                        {
                            continue;
                        }

                        let check_unique_conditions =
                            possible_natural_wonder.uniques.iter().all(|unique| {
                                let unique = Unique::new(unique);
                                match unique.placeholder_text.as_str() {
                                    "Must be adjacent to [] [] tiles" => {
                                        let count = tile
                                            .tile_neighbors(self, map_parameters)
                                            .iter()
                                            .filter(|x| {
                                                matches_wonder_filter(x, unique.params[1].as_str())
                                            })
                                            .count();
                                        count == unique.params[0].parse::<usize>().unwrap()
                                    }
                                    "Must be adjacent to [] to [] [] tiles" => {
                                        let count = tile
                                            .tile_neighbors(self, map_parameters)
                                            .iter()
                                            .filter(|x| {
                                                matches_wonder_filter(x, unique.params[2].as_str())
                                            })
                                            .count();
                                        count >= unique.params[0].parse::<usize>().unwrap()
                                            && count <= unique.params[1].parse::<usize>().unwrap()
                                    }
                                    "Must not be on [] largest landmasses" => {
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        !land_id_and_area_size
                                            .iter()
                                            .take(index)
                                            .any(|(id, _)| tile.area_id == *id)
                                    }
                                    "Must be on [] largest landmasses" => {
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        land_id_and_area_size
                                            .iter()
                                            .take(index)
                                            .any(|(id, _)| tile.area_id == *id)
                                    }
                                    _ => true,
                                }
                            });
                        // end check unique conditions

                        if check_unique_conditions {
                            natural_wonder_and_tile_index_and_score
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push((index, 1));
                        }
                    }
                }
            }
        }

        // Get the natural wonders that can be placed
        let mut selected_natural_wonder_list: Vec<_> = natural_wonder_and_tile_index_and_score
            .keys()
            .cloned()
            .collect();
        /* The order of selected_natural_wonder_list is random, so we should arrange this list in order
        to ensure that the obtained Vec is the same every time. */
        selected_natural_wonder_list.sort_unstable();
        // Shuffle the list that we can choose natural wonder randomly
        selected_natural_wonder_list.shuffle(&mut self.random_number_generator);

        // Store current how many natural wonders have been placed
        let mut j = 0;
        // Store the index of the tile where the natural wonder has been placed
        let mut placed_natural_wonder_tile_index = Vec::new();

        // start to place wonder
        for &natural_wonder_name in &selected_natural_wonder_list {
            if j <= map_parameters.natural_wonder_num {
                // For every natural wonder, give a score to the position where the natural wonder can place.
                // The score is related to the min value of the distance from the position to all the placed natural wonders
                // If no natural wonder has placed, we choose the random place where the current natural wonder can place for the current natural wonder

                // the score method start
                let tile_index_and_score = natural_wonder_and_tile_index_and_score
                    .get_mut(natural_wonder_name)
                    .unwrap();
                for (position_x_index, score) in tile_index_and_score.iter_mut() {
                    let closest_natural_wonder_dist = placed_natural_wonder_tile_index
                        .iter()
                        .map(|position_y_index: &usize| {
                            let position_x_hex = self.tile_list[*position_x_index].hex_position;
                            let position_y_hex = self.tile_list[*position_y_index].hex_position;
                            Hex::hex_distance(Hex::from(position_x_hex), Hex::from(position_y_hex))
                        })
                        .min()
                        .unwrap_or(1000000);
                    *score = if closest_natural_wonder_dist <= 10 {
                        100 * closest_natural_wonder_dist
                    } else {
                        1000 + (closest_natural_wonder_dist - 10)
                    } + self.random_number_generator.gen_range(0..100);
                }
                // the score method end

                // choose the max score position as the candidate position for the current natural wonder
                let max_score_position_index = tile_index_and_score
                    .iter()
                    .max_by_key(|&(_, score)| score)
                    .map(|&(index, _)| index)
                    .unwrap();

                if !placed_natural_wonder_tile_index.contains(&max_score_position_index) {
                    let natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                    let tile = &mut self.tile_list[max_score_position_index];
                    // At first, we should remove feature from the tile
                    tile.feature = None;

                    match natural_wonder_name.as_str() {
                        "Great Barrier Reef" => {
                            let tile = &self.tile_list[max_score_position_index];
                            let adj_tile = tile
                                .tile_neighbor(
                                    self,
                                    self.tile_edge_direction(map_parameters)[1],
                                    map_parameters,
                                )
                                .unwrap();

                            let tile_neigbors: Vec<usize> = tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .map(|tile| tile.index(map_parameters))
                                .collect();
                            let adj_tile_neigbors: Vec<usize> = adj_tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .map(|tile| tile.index(map_parameters))
                                .collect();
                            let adj_tile_index = adj_tile.index(map_parameters);

                            tile_neigbors.into_iter().for_each(|index| {
                                let tile = &mut self.tile_list[index];
                                tile.terrain_type = TerrainType::Water;
                                tile.base_terrain = BaseTerrain::Coast;
                            });
                            adj_tile_neigbors.into_iter().for_each(|index| {
                                let tile = &mut self.tile_list[index];
                                tile.terrain_type = TerrainType::Water;
                                tile.base_terrain = BaseTerrain::Coast;
                            });
                            // place the natural wonder on the candidate position and its adjacent tile
                            let tile = &mut self.tile_list[max_score_position_index];
                            tile.natural_wonder =
                                Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                            let adj_tile = &mut self.tile_list[adj_tile_index];
                            adj_tile.natural_wonder =
                                Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                            // add the position of the placed natural wonder to the list of placed natural wonder positions
                            placed_natural_wonder_tile_index.push(max_score_position_index);
                            placed_natural_wonder_tile_index.push(adj_tile_index);
                        }
                        "Rock of Gibraltar" => {
                            let tile = &self.tile_list[max_score_position_index];
                            let tile_neigbors_indices: Vec<_> = tile
                                .tile_neighbors(self, map_parameters)
                                .iter()
                                .map(|tile| tile.index(map_parameters))
                                .collect();

                            tile_neigbors_indices.into_iter().for_each(|index| {
                                let tile = &mut self.tile_list[index];
                                if tile.terrain_type == TerrainType::Water {
                                    tile.base_terrain = BaseTerrain::Coast;
                                } else {
                                    tile.terrain_type = TerrainType::Mountain;
                                }
                            });

                            let tile = &mut self.tile_list[max_score_position_index];
                            // Edit the choice tile's terrain_type to match the natural wonder
                            tile.terrain_type = TerrainType::Flatland;
                            // Edit the choice tile's base_terrain to match the natural wonder
                            tile.base_terrain = BaseTerrain::Grassland;
                            // place the natural wonder on the candidate position
                            tile.natural_wonder =
                                Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                            // add the position of the placed natural wonder to the list of placed natural wonder positions
                            placed_natural_wonder_tile_index.push(max_score_position_index);
                        }
                        _ => {
                            // Edit the choice tile's terrain_type to match the natural wonder
                            if let Some(turn_into_terrain_type) = natural_wonder.turns_into_type {
                                tile.terrain_type = turn_into_terrain_type;
                            };
                            // Edit the choice tile's base_terrain to match the natural wonder
                            if let Some(turn_into_base_terrain) = natural_wonder.turns_into_base {
                                tile.base_terrain = turn_into_base_terrain;
                            }
                            // place the natural wonder on the candidate position
                            tile.natural_wonder =
                                Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                            // add the position of the placed natural wonder to the list of placed natural wonder positions
                            placed_natural_wonder_tile_index.push(max_score_position_index);
                        }
                    }
                    j += 1;
                }
            }
        }

        // If the natural wonder is not water, and its neighbors have water tile, then change the neighbor tiles to lake or coast
        placed_natural_wonder_tile_index.iter().for_each(|&index| {
            let tile = &self.tile_list[index];
            if tile.terrain_type != TerrainType::Water {
                let tile_neighbors_index: Vec<_> = tile
                    .tile_neighbors(self, map_parameters)
                    .iter()
                    .map(|tile| tile.index(map_parameters))
                    .collect();

                tile_neighbors_index.iter().for_each(|&index| {
                    let tile = &self.tile_list[index];
                    if tile.terrain_type == TerrainType::Water {
                        if tile.tile_neighbors(self, map_parameters).iter().any(
                            |tile_neighbor_neighbor| {
                                tile_neighbor_neighbor.base_terrain == BaseTerrain::Lake
                            },
                        ) {
                            self.tile_list[index].base_terrain = BaseTerrain::Lake;
                        } else {
                            self.tile_list[index].base_terrain = BaseTerrain::Coast;
                        };
                    };
                });
            }
        });
    }
}
