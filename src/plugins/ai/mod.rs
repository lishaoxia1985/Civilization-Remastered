//! AI 模块
//!
//! 管理 AI 文明的行为逻辑，包括：
//! - AI 战斗决策
//!
//! 注：AI 科技选择已迁移至 [`crate::plugins::tech::ai`]。

mod ai_combat;

pub use ai_combat::AiCombatPlugin;
