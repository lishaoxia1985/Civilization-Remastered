//! 战斗系统插件
//!
//! 管理战斗解析逻辑，包括：
//! - 攻击请求处理
//! - 伤害计算
//! - 战斗结果（摧毁、伤害）
//! - 战斗修正（地形、生命值等）
//! - 远程/近战攻击区分（远程单位攻击不会受到反击）

use bevy::prelude::*;
use civ_map_generator::ruleset::enums::Nation;

use crate::{
    AttackRequestMessage, NationComponent, ResolutionPhase, TurnManager, TurnState,
    components::{Health, Movement, Owner, Range, RangedStrength, SelectedUnit, Strength},
};

/// 战斗插件
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(TurnState::Start),
            restore_movement.in_set(ResolutionPhase::MovementRestore),
        )
        .add_observer(resolve_combat);
    }
}

// ============ 战斗解析系统 ============

/// 观察者模式 - 响应攻击请求
fn resolve_combat(
    event: On<AttackRequestMessage>,
    mut commands: Commands,
    unit_query: Query<(
        &Owner,
        &Health,
        &Strength,
        Option<&RangedStrength>,
        Option<&Range>,
    )>,
    turn_manager: Res<TurnManager>,
) {
    let attack = event.event();
    resolve_attack(
        &mut commands,
        attack.attacker,
        attack.target,
        &unit_query,
        turn_manager.turn_number,
    );
}

/// 核心战斗解析逻辑
fn resolve_attack(
    commands: &mut Commands,
    attacker: Entity,
    target: Entity,
    unit_query: &Query<(
        &Owner,
        &Health,
        &Strength,
        Option<&RangedStrength>,
        Option<&Range>,
    )>,
    turn_number: u32,
) {
    // ---- 1. 获取组件 ----
    let Ok((
        attacker_owner,
        attacker_health,
        attacker_strength,
        attacker_ranged_strength,
        attacker_range,
    )) = unit_query.get(attacker)
    else {
        warn!("Attacker missing combat components");
        return;
    };
    let Ok((defender_owner, defender_health, defender_strength, _, _)) = unit_query.get(target)
    else {
        warn!("Defender missing combat components");
        return;
    };

    let attacker_owner = attacker_owner.0;
    let defender_owner = defender_owner.0;

    // 判断攻击者是否为远程单位（同时拥有 RangedStrength 和 Range 组件）
    let is_ranged_attacker = attacker_ranged_strength.is_some() && attacker_range.is_some();

    // ---- 2. 基础战斗力（不受血量影响） ----
    // 远程单位使用远程战斗力，近战单位使用近战战斗力
    // TODO: 当前并未添加地形和UnitPromotion对攻击力的影响
    let attack_power = if is_ranged_attacker {
        attacker_ranged_strength.unwrap().0 as f32
    } else {
        attacker_strength.0 as f32
    };
    let defense_power = defender_strength.0 as f32;

    // ---- 3. 血量惩罚系数（受伤单位造成伤害减少） ----
    // 文明5公式：每损失 3% 生命值，造成伤害 -1%
    // ratio = 1 - (max - current) / (3 * max)
    let health_damage_ratio = |health: &Health, nation: Nation| -> f32 {
        // TODO: 临时处理，日本无血量惩罚
        //       未来应当基于文明的独特特性来计算，因为未来可以自定义mod时此特性就非Japan独有
        if nation == Nation::Japan {
            1.0
        } else {
            let missing = (health.max - health.current) as f32;
            let max = health.max as f32;
            1.0 - missing / (3.0 * max)
        }
    };
    let attacker_health_factor = health_damage_ratio(attacker_health, attacker_owner);
    let defender_health_factor = health_damage_ratio(defender_health, defender_owner);

    // ---- 4. 生成两个独立的随机数（0~1） ----
    let mut rng1 = SimpleRng::new(turn_number as u64);
    let p1 = rng1.f32(); // 攻击伤害随机因子

    let mut rng2 = SimpleRng::new((turn_number + 1) as u64);
    let p2 = rng2.f32(); // 反击伤害随机因子

    // ---- 5. 伤害计算（文明5底层公式） ----
    // 参数说明：
    // - attacker_to_defender_ratio: 攻击方战斗力 / 防守方战斗力
    // - damage_to_attacker: true 表示计算攻击方受到的（反击）伤害
    // - randomness: 0~1 随机数
    // - health_factor: 造成伤害一方的血量惩罚系数
    fn compute_damage(
        attacker_to_defender_ratio: f32,
        damage_to_attacker: bool,
        randomness: f32,
        health_factor: f32,
    ) -> u32 {
        if attacker_to_defender_ratio <= 0.0 || health_factor <= 0.0 {
            return 0;
        }

        // 强/弱比值（始终 >= 1）
        let r = if attacker_to_defender_ratio >= 1.0 {
            attacker_to_defender_ratio
        } else {
            1.0 / attacker_to_defender_ratio
        };

        // 基础修正值
        let modifier = ((r + 3.0) / 4.0).powi(4).mul_add(0.5, 0.5); // ((...)/2 + 0.5) 即 (((...)+1)/2)

        // 弱势方造成伤害时取倒数
        let is_weaker_dealing_damage = if damage_to_attacker {
            attacker_to_defender_ratio > 1.0 // 防守方更弱
        } else {
            attacker_to_defender_ratio < 1.0 // 攻击方更弱
        };
        let effective_modifier = if is_weaker_dealing_damage {
            1.0 / modifier
        } else {
            modifier
        };

        let random_base = 24.0 + 12.0 * randomness;
        let damage = random_base * effective_modifier * health_factor;

        damage.max(1.0).round() as u32
    }

    let ratio = attack_power / defense_power;

    // 攻击方对防守方造成的伤害（攻击方是伤害来源，受攻击方血量惩罚）
    let damage_to_defender = compute_damage(ratio, false, p1, attacker_health_factor);

    // 防守方对攻击方造成的反击伤害（防守方是伤害来源，受防守方血量惩罚）
    // 远程单位攻击时不会受到反击伤害；近战单位攻击时和原算法一致
    let damage_to_attacker = if is_ranged_attacker {
        0
    } else {
        compute_damage(ratio, true, p2, defender_health_factor)
    };

    // ---- 6. 应用伤害 ----
    let new_attacker_hp = attacker_health.current.saturating_sub(damage_to_attacker);
    commands.entity(attacker).insert(Health {
        current: new_attacker_hp,
        max: attacker_health.max,
    });

    let new_defender_hp = defender_health.current.saturating_sub(damage_to_defender);
    commands.entity(target).insert(Health {
        current: new_defender_hp,
        max: defender_health.max,
    });

    // ---- 7. 移除死亡单位 ----
    if new_attacker_hp == 0 {
        info!("Attacker {} died from counter damage", attacker);
        commands.entity(attacker).despawn();
    }
    if new_defender_hp == 0 {
        info!("Defender {} died from attack damage", target);
        commands.entity(target).despawn();
    }

    // ---- 8. 日志与清理 ----
    info!(
        "Combat: Attacker (ATK={:.1}) dealt {} dmg to Defender (DEF={:.1}), took {} dmg in return.",
        attack_power, damage_to_defender, defense_power, damage_to_attacker
    );

    // 移除选中状态
    commands.entity(attacker).remove::<SelectedUnit>();
}

/// 恢复当前 Nation 所属单位的移动力
fn restore_movement(
    mut unit_query: Query<(&Owner, &mut Movement)>,
    turn_manager: Res<TurnManager>,
    nation_query: Query<&NationComponent>,
) {
    // 获取当前回合的 Nation
    let current_nation_entity = turn_manager.current_nation_entity();
    let Ok(nation_component) = nation_query.get(current_nation_entity) else {
        panic!("Current nation entity missing NationComponent");
    };
    let current_nation = nation_component.0;

    // 只恢复当前 Nation 所属单位的移动力
    for (owner, mut movement) in unit_query.iter_mut() {
        if owner.0 == current_nation {
            movement.current = movement.max;
        }
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
