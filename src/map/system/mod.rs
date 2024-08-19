mod add_feature;
mod add_river;
mod generate_area_id;
mod generate_coast_and_ocean;
mod generate_empty_map;
mod generate_lake;
mod generate_natural_wonder;
mod generate_terrain;
mod generate_terrain_type;

pub use add_feature::add_features;
pub use add_river::add_rivers;
pub use generate_area_id::{reassign_area_id, recalculate_areas};
pub use generate_coast_and_ocean::{expand_coast, generate_coast_and_ocean};
pub use generate_empty_map::generate_empty_map;
pub use generate_lake::{add_lakes, generate_lake};
pub use generate_natural_wonder::{generate_natural_wonder, regenerate_coast};
pub use generate_terrain::generate_terrain;
pub use generate_terrain_type::{
    generate_terrain_type_for_fractal, generate_terrain_type_for_pangaea,
};
