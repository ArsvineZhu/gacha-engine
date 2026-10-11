use crate::engine::{enumerate_transitions, sample_transition, state_slot_key};
use crate::{
    Branch, CompiledGame, EngineError, Result, ScopeContext, SplitMix64, StateScope, StateStore,
};

/// 按当前上下文清理过期的会话、批次和单抽状态。
///
/// 账户、游戏、保底组、进度组和卡池状态持久保存，
/// 再次回到同一卡池或组时恢复原进度。
pub fn prune_ephemeral_state(
    game: &CompiledGame,
    banner_id: &str,
    state: &mut StateStore,
    context: &ScopeContext,
) -> Result<()> {
    let (_, banner) = game
        .banner(banner_id)
        .ok_or_else(|| EngineError::new(format!("unknown banner: {banner_id}")))?;

    for def in &game.states {
        if !matches!(
            def.scope,
            StateScope::Session | StateScope::Batch | StateScope::Pull
        ) {
            continue;
        }

        let active_key = state_slot_key(game, banner, def, context);
        let prefix = format!("{}::", def.id);
        state
            .slots
            .retain(|key, _| !key.starts_with(&prefix) || key == &active_key);
    }

    Ok(())
}

/// 结合状态作用域的生命周期枚举转移。
///
/// 顺序执行的高层接口，
/// 先处理临时状态生命周期，再调用精确转移引擎。
pub fn enumerate_step(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &StateStore,
    context: &ScopeContext,
) -> Result<Vec<Branch>> {
    let mut normalized = state.clone();
    prune_ephemeral_state(game, banner_id, &mut normalized, context)?;
    enumerate_transitions(game, banner_id, action_id, &normalized, context)
}

/// 结合状态作用域的生命周期抽样转移。
///
/// 选中分支就地更新 `state`，
/// 普通顺序模拟使用此接口。
pub fn sample_step(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &mut StateStore,
    context: &ScopeContext,
    rng: &mut SplitMix64,
) -> Result<Branch> {
    let mut normalized = state.clone();
    prune_ephemeral_state(game, banner_id, &mut normalized, context)?;
    let branch = sample_transition(game, banner_id, action_id, &mut normalized, context, rng)?;
    *state = normalized;
    Ok(branch)
}

/// 原子执行完整动作，每次启动新的批次与单抽状态，
/// 即使上下文标识相同；失败时保持状态和随机数进度。
pub fn sample_action(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &mut StateStore,
    context: &ScopeContext,
    rng: &mut SplitMix64,
) -> Result<Vec<Branch>> {
    let (_, banner) = game
        .banner(banner_id)
        .ok_or_else(|| EngineError::new(format!("unknown banner: {banner_id}")))?;
    let action = banner
        .action(action_id)
        .ok_or_else(|| EngineError::new(format!("unknown action: {action_id}")))?;
    let mut next_state = state.clone();
    let mut next_rng = rng.clone();
    reset_action_state(game, &mut next_state);
    let mut results = Vec::with_capacity(action.draws as usize);
    for draw in 0..action.draws {
        let mut draw_context = context.clone();
        draw_context.pull = format!("{}:{draw}", context.pull);
        results.push(sample_step(
            game,
            banner_id,
            action_id,
            &mut next_state,
            &draw_context,
            &mut next_rng,
        )?);
    }
    *state = next_state;
    *rng = next_rng;
    Ok(results)
}

/// 完整动作开始前清空批次与单抽状态，保留持久进度。
pub fn reset_action_state(game: &CompiledGame, state: &mut StateStore) {
    for def in &game.states {
        if matches!(def.scope, StateScope::Batch | StateScope::Pull) {
            let prefix = format!("{}::", def.id);
            state.slots.retain(|key, _| !key.starts_with(&prefix));
        }
    }
}
