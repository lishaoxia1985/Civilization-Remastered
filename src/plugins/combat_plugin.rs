//! 战斗系统插件
//!
//! 管理战斗解析逻辑，包括：
//! - 攻击请求处理
//! - 伤害计算
//! - 战斗结果（摧毁、伤害）
//! - 战斗修正（地形、生命值等）

use bevy::prelude::*;

use crate::{
    AttackRequestMessage, TurnManager,
    components::{Health, Movement, Owner, SelectedUnit, Strength},
};

/// 战斗插件
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, advance_turn_system)
            .add_observer(resolve_combat);
    }
}

// ============ 战斗解析系统 ============

/// 观察者模式 - 响应攻击请求
fn resolve_combat(
    event: On<AttackRequestMessage>,
    mut commands: Commands,
    attacker_query: Query<(&Owner, &Health, &Strength)>,
    defender_query: Query<(&Owner, &Health, &Strength)>,
    turn_manager: Res<TurnManager>,
) {
    let attack = event.event();
    resolve_attack(
        &mut commands,
        attack.attacker,
        attack.target,
        &attacker_query,
        &defender_query,
        turn_manager.turn_number,
    );
}

/// 核心战斗解析逻辑
fn resolve_attack(
    commands: &mut Commands,
    attacker: Entity,
    target: Entity,
    attacker_query: &Query<(&Owner, &Health, &Strength)>,
    defender_query: &Query<(&Owner, &Health, &Strength)>,
    turn_number: u32,
) {
    let Ok((_attacker_owner, attacker_health, attacker_strength)) = attacker_query.get(attacker)
    else {
        warn!("Attacker entity missing combat components");
        return;
    };

    let Ok((_defender_owner, defender_health, defender_strength)) = defender_query.get(target)
    else {
        warn!("Defender entity missing combat components");
        return;
    };

    // 计算基础战斗力
    let attack_power = attacker_strength.0 as f32;
    let defense_power = defender_strength.0 as f32;

    // 计算战斗修正
    // 1. 生命值修正：受伤单位战斗力下降
    let attacker_health_ratio = attacker_health.current as f32 / attacker_health.max as f32;
    let defender_health_ratio = defender_health.current as f32 / defender_health.max as f32;

    let modified_attack = attack_power * (0.5 + 0.5 * attacker_health_ratio);
    let modified_defense = defense_power * (0.5 + 0.5 * defender_health_ratio);

    // 计算获胜概率
    let total_strength = modified_attack + modified_defense;
    let attacker_win_chance = if total_strength > 0.0 {
        modified_attack / total_strength
    } else {
        0.5
    };

    // 基于回合数的随机数种子
    let seed = turn_number as u64;
    let mut rng = SimpleRng::new(seed);
    let roll = rng.f32();

    if roll < attacker_win_chance {
        // 攻击者获胜 - 摧毁目标
        info!(
            "Combat: Attacker ({:.0} power) defeated Defender ({:.0} power)!",
            modified_attack, modified_defense
        );
        commands.entity(target).despawn();

        // 攻击者受到反伤
        let counter_damage = (defense_power * 0.15).max(1.0) as u32;
        let new_health = attacker_health.current.saturating_sub(counter_damage);
        commands.entity(attacker).insert(Health {
            current: new_health,
            max: attacker_health.max,
        });
        info!("Attacker took {} counter damage", counter_damage);
    } else {
        // 防御者获胜 - 攻击者受到伤害
        let damage = (modified_defense * 0.3).max(1.0) as u32;
        let new_health = attacker_health.current.saturating_sub(damage);
        commands.entity(attacker).insert(Health {
            current: new_health,
            max: attacker_health.max,
        });
        info!("Defender repelled attack! Attacker took {} damage", damage);

        // 防御者受到少量伤害
        let defender_damage = (modified_attack * 0.1).max(1.0) as u32;
        let new_defender_health = defender_health.current.saturating_sub(defender_damage);
        commands.entity(target).insert(Health {
            current: new_defender_health,
            max: defender_health.max,
        });
    }

    // 移除攻击者的选中状态
    commands.entity(attacker).remove::<SelectedUnit>();
}

// ============ 回合推进系统 ============

/// 回合推进系统 - 恢复单位移动力
fn advance_turn_system(mut unit_query: Query<&mut Movement>) {
    for mut movement in unit_query.iter_mut() {
        movement.current = movement.max;
    }
}

// ============ 工具类 ============

/// 简单随机数生成器
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as f32 / (1u64 << 31) as f32
    }
}
