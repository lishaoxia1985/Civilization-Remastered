//! 科技领域模块
//!
//! 管理科技研究与时代演进相关的一切内容，包括：
//! - [科技组件](components)：科技研究状态、时代等数据组件
//! - [科技逻辑函数](functions)：成本计算、可研究性判断等
//! - [科技消息](messages)：科技研发完成等消息
//! - [科技插件](tech_plugin)：回合科研结算
//! - [AI 科技选择](ai)：AI 文明自动选择科技

mod ai;
mod components;
mod functions;
mod messages;
mod tech_plugin;

pub use ai::AiTechPlugin;
pub use components::*;
pub use functions::*;
pub use messages::*;
pub use tech_plugin::TechPlugin;
