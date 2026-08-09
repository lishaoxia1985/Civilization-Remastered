//! 战斗系统插件
//!
//! 管理战斗解析逻辑，包括：
//! - 攻击请求处理
//! - 伤害计算
//! - 战斗结果（摧毁、伤害）
//! - 战斗修正（地形、生命值等）

use bevy::prelude::*;

use crate::{
    AttackRequestMessage, TurnManager, TurnState,
    components::{Health, Movement, Owner, SelectedUnit, Strength},
};

/// 战斗插件
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        // 在每回合开始时恢复所有单位的移动力（而不是每帧）
        app.add_systems(OnEnter(TurnState::Start), advance_turn_system)
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
    // ---- 1. 获取组件 ----
    let Ok((_attacker_owner, attacker_health, attacker_strength)) = attacker_query.get(attacker) else {
        warn!("Attacker missing combat components");
        return;
    };
    let Ok((_defender_owner, defender_health, defender_strength)) = defender_query.get(target) else {
        warn!("Defender missing combat components");
        return;
    };

    // ---- 2. 计算最终战斗力（含生命值修正） ----
    let attack_power = attacker_strength.0 as f32;
    let defense_power = defender_strength.0 as f32;

    let attacker_ratio = attacker_health.current as f32 / attacker_health.max as f32;
    let defender_ratio = defender_health.current as f32 / defender_health.max as f32;

    // 文明5中受伤单位的战斗力线性下降（BNW后惩罚略轻，这里简化为线性）
    let modified_attack = attack_power * (0.5 + 0.5 * attacker_ratio);
    let modified_defense = defense_power * (0.5 + 0.5 * defender_ratio);

    // ---- 3. 生成两个独立的随机数（0~1） ----
    // 使用 turn_number 作为种子，以确保可重现
    let mut rng1 = SimpleRng::new(turn_number as u64);
    let p1 = rng1.f32();                     // 攻击伤害随机因子

    let mut rng2 = SimpleRng::new((turn_number + 1) as u64); // 不同种子
    let p2 = rng2.f32();                     // 反击伤害随机因子

    // ---- 4. 计算伤害（文明5公式） ----
    // 攻击方对防守方造成的伤害
    let damage_to_defender = if modified_attack > 0.0 && modified_defense > 0.0 {
        let ratio = modified_attack / modified_defense;
        let base = (24.0 + 12.0 * p1) * ((ratio + 3.0).powi(4) / 512.0 + 0.5);
        base.max(1.0) as u32   // 至少造成1点伤害
    } else {
        0
    };

    // 防守方对攻击方的反击伤害（交换攻守位置）
    let damage_to_attacker = if modified_defense > 0.0 && modified_attack > 0.0 {
        let ratio = modified_defense / modified_attack;
        let base = (24.0 + 12.0 * p2) * ((ratio + 3.0).powi(4) / 512.0 + 0.5);
        base.max(1.0) as u32
    } else {
        0
    };

    // ---- 5. 应用伤害 ----
    // 攻击者扣血
    let new_attacker_hp = attacker_health.current.saturating_sub(damage_to_attacker);
    commands.entity(attacker).insert(Health {
        current: new_attacker_hp,
        max: attacker_health.max,
    });

    // 防守者扣血
    let new_defender_hp = defender_health.current.saturating_sub(damage_to_defender);
    commands.entity(target).insert(Health {
        current: new_defender_hp,
        max: defender_health.max,
    });

    // ---- 6. 移除死亡单位 ----
    if new_attacker_hp == 0 {
        info!("Attacker {} died from counter damage", attacker);
        commands.entity(attacker).despawn();
    }
    if new_defender_hp == 0 {
        info!("Defender {} died from attack damage", target);
        commands.entity(target).despawn();
    }

    // ---- 7. 日志与清理 ----
    info!(
        "Combat: Attacker (ATK={:.1}) dealt {} dmg to Defender (DEF={:.1}), took {} dmg in return.",
        modified_attack, damage_to_defender, modified_defense, damage_to_attacker
    );

    // 移除选中状态
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
