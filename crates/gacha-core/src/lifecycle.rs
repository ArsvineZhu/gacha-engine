use crate::engine::{enumerate_transitions, sample_transition, state_slot_key};
use crate::{
    Branch, CompiledGame, EngineError, Result, ScopeContext, SplitMix64, StateScope, StateStore,
};

/// Removes stale session/batch/pull scoped slots for the supplied context.
///
/// Persistent scopes (account/game/pity/progress/banner) are intentionally retained so
/// switching away from and later returning to a banner/group restores their state.
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

/// Scope-aware transition enumeration.
///
/// This is the preferred high-level API for sequential execution. It applies the
/// lifecycle rules for ephemeral state before delegating to the exact transition engine.
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

/// Scope-aware sampled transition.
///
/// The selected branch updates `state` in place. Use this instead of the low-level
/// `sample_transition` for ordinary sequential simulations.
pub fn sample_step(
    game: &CompiledGame,
    banner_id: &str,
    action_id: &str,
    state: &mut StateStore,
    context: &ScopeContext,
    rng: &mut SplitMix64,
) -> Result<Branch> {
    prune_ephemeral_state(game, banner_id, state, context)?;
    sample_transition(game, banner_id, action_id, state, context, rng)
}
