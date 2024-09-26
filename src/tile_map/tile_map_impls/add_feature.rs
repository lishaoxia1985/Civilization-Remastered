use rand::Rng;

use crate::{
    map::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType},
    ruleset::Ruleset,
    tile_map::{MapParameters, Rainfall, TileMap},
};

impl TileMap {
    pub fn add_feature(&mut self, ruleset: &Ruleset, map_parameters: &MapParameters) {
        let rainfall = match map_parameters.rainfall {
            Rainfall::Arid => -4,
            Rainfall::Normal => 0,
            Rainfall::Wet => 4,
            Rainfall::Random => self.random_number_generator.gen_range(0..11) - 5,
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

        for tile_index in self.tile_indices_iter() {
            let latitude = tile_index.latitude(&map_parameters);

            let neighbor_tile_indices = tile_index.neighbor_tile_indices(&map_parameters);

            /* **********start to add ice********** */
            if tile_index.is_impassable(self, &ruleset) {
                continue;
            } else if tile_index.terrain_type(self) == TerrainType::Water {
                if !map_parameters
                    .edge_direction_array()
                    .iter()
                    .any(|&direction| tile_index.has_river(direction, self, &map_parameters))
                    && ruleset.features["Ice"]
                        .occurs_on_type
                        .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Ice"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                {
                    if latitude > 0.78 {
                        let mut score = self.random_number_generator.gen_range(0..100) as f64;
                        score += latitude * 100.;
                        if neighbor_tile_indices
                            .iter()
                            .any(|&tile_index| tile_index.terrain_type(self) != TerrainType::Water)
                        {
                            score /= 2.0;
                        }
                        let a = neighbor_tile_indices
                            .iter()
                            .filter(|tile_index| tile_index.feature(self) == Some(Feature::Ice))
                            .count();
                        score += 10. * a as f64;
                        if score > 130. {
                            self.feature_query[*tile_index] = Some(Feature::Ice);
                        }
                    }
                }
            }
            /* **********the end of add ice********** */
            else {
                /* **********start to add Floodplain********** */
                num_land_plots += 1;
                if map_parameters
                    .edge_direction_array()
                    .iter()
                    .any(|&direction| tile_index.has_river(direction, self, &map_parameters))
                    && ruleset.features["Floodplain"]
                        .occurs_on_type
                        .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Floodplain"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                {
                    self.feature_query[*tile_index] = Some(Feature::Floodplain);
                    continue;
                }
                /* **********the end of add Floodplain********** */
                /* **********start to add oasis********** */
                else if ruleset.features["Oasis"]
                    .occurs_on_type
                    .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Oasis"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                    && (oasis_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= oasis_max_percent
                    && self.random_number_generator.gen_range(0..4) == 1
                {
                    self.feature_query[*tile_index] = Some(Feature::Oasis);
                    oasis_count += 1;
                    continue;
                }
                /* **********the end of add oasis********** */
                /* **********start to add march********** */
                if ruleset.features["Marsh"]
                    .occurs_on_type
                    .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Marsh"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                    && (marsh_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= marsh_max_percent
                {
                    let mut score = 300;

                    let a = neighbor_tile_indices
                        .iter()
                        .filter(|tile_index| tile_index.feature(self) == Some(Feature::Marsh))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        self.feature_query[*tile_index] = Some(Feature::Marsh);
                        marsh_count += 1;
                        continue;
                    }
                };
                /* **********the end of add march********** */
                /* **********start to add jungle********** */
                if ruleset.features["Jungle"]
                    .occurs_on_type
                    .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Jungle"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                    && (jungle_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= jungle_max_percent
                    && (latitude >= jungle_bottom as f64 / 100.
                        && latitude <= jungle_top as f64 / 100.)
                {
                    let mut score = 300;

                    let a = neighbor_tile_indices
                        .iter()
                        .filter(|tile_index| tile_index.feature(self) == Some(Feature::Jungle))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        self.feature_query[*tile_index] = Some(Feature::Jungle);

                        if tile_index.terrain_type(self) == TerrainType::Hill
                            && matches!(
                                tile_index.base_terrain(self),
                                BaseTerrain::Grassland | BaseTerrain::Plain
                            )
                        {
                            self.base_terrain_query[*tile_index] = BaseTerrain::Plain;
                        } else {
                            self.terrain_type_query[*tile_index] = TerrainType::Flatland;
                            self.base_terrain_query[*tile_index] = BaseTerrain::Plain;
                        }

                        jungle_count += 1;
                        continue;
                    }
                }
                /* **********the end of add jungle********** */
                /* **********start to add forest********** */
                if ruleset.features["Forest"]
                    .occurs_on_type
                    .contains(&tile_index.terrain_type(self))
                    && ruleset.features["Forest"]
                        .occurs_on_base
                        .contains(&tile_index.base_terrain(self))
                    && (forest_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= forest_max_percent
                {
                    let mut score = 300;

                    let a = neighbor_tile_indices
                        .iter()
                        .filter(|tile_index| tile_index.feature(self) == Some(Feature::Forest))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        self.feature_query[*tile_index] = Some(Feature::Forest);
                        forest_count += 1;
                        continue;
                    }
                }
                /* **********the end of add forest********** */
            }
        }
    }
}
