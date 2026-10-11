use super::*;

pub(crate) fn validate_initial_distributions(game: &CompiledGame) -> Result<()> {
    for banner in &game.banners {
        let mut state = StateStore::default();
        materialize_state(game, banner, &mut state, &ScopeContext::default())?;
        for action in &banner.actions {
            let branches = enumerate_transitions(
                game,
                &banner.id,
                &action.id,
                &state,
                &ScopeContext::default(),
            )?;
            if branches.is_empty() {
                return Err(EngineError::new(format!(
                    "banner {} action {} produces no initial branches",
                    banner.id, action.id
                )));
            }
        }
    }
    Ok(())
}
