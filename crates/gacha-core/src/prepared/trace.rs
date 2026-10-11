use super::*;
use crate::engine::evaluate_condition;

#[derive(Debug, Clone, Serialize)]
pub struct RuleCheck {
    pub id: String,
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateChange {
    pub key: String,
    pub before: StateValue,
    pub after: StateValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateProbability {
    pub item: String,
    pub probability: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionTrace {
    pub applied_guarantee: Option<String>,
    pub guarantee_checks: Vec<RuleCheck>,
    pub transition_checks: Vec<RuleCheck>,
    pub candidates: Vec<CandidateProbability>,
    pub state_changes: Vec<StateChange>,
    pub events: Vec<Event>,
}

impl PreparedRuntime {
    pub fn trace_step(
        &self,
        banner: &str,
        action: &str,
        state: &StateStore,
        context: &ScopeContext,
        branch: &Branch,
    ) -> Result<TransitionTrace> {
        let (bi, ai) = self.action(banner, action)?;
        let pre = self.normalize(banner, state, context)?;
        let b = &self.game.banners[bi];
        let a = &b.actions[ai];
        let plan = self.plan(bi, ai, &pre, context)?;
        let check = |id: &str, condition: &Condition, outcome| -> Result<RuleCheck> {
            Ok(RuleCheck {
                id: id.into(),
                matched: evaluate_condition(&self.game, b, &pre, context, outcome, condition)?,
                priority: None,
                effect: None,
            })
        };
        let guarantee_checks = a
            .guarantees
            .iter()
            .map(|g| {
                let mut decision = check(&g.id, &g.when, None)?;
                decision.priority = Some(g.priority);
                decision.effect = Some(match g.effect {
                    GuaranteeEffect::ForceItem(id) => {
                        format!("force_item:{}", self.game.item(id).id)
                    }
                    GuaranteeEffect::ForceRarity(r) => format!("force_rarity:{r}"),
                    GuaranteeEffect::MinRarity { rarity, strategy } => {
                        format!("min_rarity:{rarity}:{strategy:?}")
                    }
                });
                Ok(decision)
            })
            .collect::<Result<_>>()?;
        let transition_checks = a
            .transitions
            .iter()
            .map(|r| check(&r.id, &r.when, Some(branch.outcome)))
            .collect::<Result<_>>()?;
        let candidates = plan
            .choices
            .iter()
            .map(|c| CandidateProbability {
                item: self.game.item(c.outcome.item).id.clone(),
                probability: c.probability,
            })
            .collect();
        let state_changes = pre
            .slots
            .iter()
            .filter_map(|(key, before)| {
                branch
                    .next_state
                    .slots
                    .get(key)
                    .filter(|after| *after != before)
                    .map(|after| StateChange {
                        key: key.clone(),
                        before: before.clone(),
                        after: after.clone(),
                    })
            })
            .collect();
        Ok(TransitionTrace {
            applied_guarantee: plan.guarantees.first().cloned(),
            guarantee_checks,
            transition_checks,
            candidates,
            state_changes,
            events: branch.events.clone(),
        })
    }
}
