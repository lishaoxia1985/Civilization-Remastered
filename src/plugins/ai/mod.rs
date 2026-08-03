//! AI 模块
//!
//! 管理 AI 文明的行为逻辑，包括：
//! - AI 科技选择
//! - AI 战斗决策

mod ai_combat;
mod ai_tech;

pub use ai_combat::AiCombatPlugin;
pub use ai_tech::AiTechPlugin;
