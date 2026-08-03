//! 交互模块
//!
//! 管理玩家与游戏世界的交互操作，包括：
//! - 单位选择、移动、操作菜单
//! - 游戏状态 UI
//! - 科技树屏幕

mod tech_tree_screen_plugin;
mod ui_plugin;
mod unit_interaction_plugin;

pub use tech_tree_screen_plugin::TechTreeScreenPlugin;
pub use ui_plugin::UiPlugin;
pub use unit_interaction_plugin::UnitInteractionPlugin;
