//! 游戏插件模块
//!
//! 所有游戏功能被组织为 Bevy Plugin，按功能领域划分。

mod asset_plugin;
mod camera_plugin;
mod combat_plugin;
mod map_plugin;
mod minimap_plugin;
mod tech_tree_screen_plugin;
mod turn_plugin;
mod ui_plugin;

pub use asset_plugin::AssetLoadingPlugin;
pub use camera_plugin::CameraPlugin;
pub use combat_plugin::CombatPlugin;
pub use map_plugin::MapPlugin;
pub use minimap_plugin::MinimapPlugin;
pub use tech_tree_screen_plugin::TechTreeScreenPlugin;
pub use turn_plugin::TurnPlugin;
pub use ui_plugin::UiPlugin;
