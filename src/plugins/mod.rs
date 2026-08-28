//! 游戏插件模块
//!
//! 所有游戏功能被组织为 Bevy Plugin，按功能领域划分。

mod ai;
mod asset_plugin;
mod camera_plugin;
mod citizen_assign_screen_plugin;
mod city_construction_plugin;
mod city_interaction_plugin;
mod city_management_plugin;
mod combat_plugin;
mod construction_plugin;
mod era_plugin;
mod interaction;
mod map_plugin;
mod minimap_plugin;
mod movement_plugin;
pub mod tech;
mod turn_plugin;
mod unit_manager_plugin;

pub use ai::*;
pub use asset_plugin::AssetLoadingPlugin;
pub use camera_plugin::CameraPlugin;
pub use citizen_assign_screen_plugin::CitizenAssignScreenPlugin;
pub use city_construction_plugin::CityConstructionPlugin;
pub use city_interaction_plugin::CityInteractionPlugin;
pub use city_management_plugin::CityManagementPlugin;
pub use combat_plugin::CombatPlugin;
pub use construction_plugin::ConstructionPlugin;
pub use era_plugin::EraPlugin;
pub use interaction::*;
pub use map_plugin::MapPlugin;
pub use minimap_plugin::MinimapPlugin;
pub use movement_plugin::MovementPlugin;
pub use turn_plugin::TurnPlugin;
pub use unit_manager_plugin::UnitManagerPlugin;
