use crate::*;
use gacha_core::{SplitMix64, materialize_state, reset_action_state, state_slot_key};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub format_version: u32,
    pub engine_version: String,
    pub fingerprint: String,
    pub state: StateStore,
    pub context: ScopeContext,
    pub rng: SplitMix64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DrawRecord {
    pub item: ItemInfo,
    pub probability: Rational,
    pub guarantees: Vec<String>,
    pub events: Vec<Event>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<TransitionTrace>,
}

pub struct Session {
    engine: Engine,
    state: StateStore,
    context: ScopeContext,
    rng: SplitMix64,
}

impl Engine {
    pub fn session(&self, seed: u64) -> Session {
        Session {
            engine: self.clone(),
            state: StateStore::default(),
            context: ScopeContext::default(),
            rng: SplitMix64::new(seed),
        }
    }

    pub fn restore_session(&self, snapshot: SessionSnapshot) -> Result<Session> {
        if snapshot.format_version != 1
            || snapshot.engine_version != env!("CARGO_PKG_VERSION")
            || snapshot.fingerprint != self.fingerprint
        {
            return Err(EngineError::new(
                "session snapshot does not match engine and rule fingerprint",
            ));
        }
        Ok(Session {
            engine: self.clone(),
            state: snapshot.state,
            context: snapshot.context,
            rng: snapshot.rng,
        })
    }
}

impl Session {
    pub fn set_context(&mut self, context: ScopeContext) {
        self.context = context;
    }

    pub fn state(&self) -> &StateStore {
        &self.state
    }
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            format_version: 1,
            engine_version: env!("CARGO_PKG_VERSION").into(),
            fingerprint: self.engine.fingerprint.clone(),
            state: self.state.clone(),
            context: self.context.clone(),
            rng: self.rng.clone(),
        }
    }

    pub fn set_state(&mut self, banner: &str, id: &str, value: StateValue) -> Result<()> {
        let game = self.engine.runtime.game();
        let (_, b) = game
            .banner(banner)
            .ok_or_else(|| EngineError::new("unknown banner"))?;
        let index = game
            .state_lookup
            .get(id)
            .ok_or_else(|| EngineError::new("unknown state"))?;
        let mut next = self.state.clone();
        next.slots.insert(
            state_slot_key(game, b, &game.states[index.0], &self.context),
            value,
        );
        materialize_state(game, b, &mut next, &self.context)?;
        self.state = next;
        Ok(())
    }

    pub fn pull(&mut self, banner: &str, action: &str) -> Result<Vec<DrawRecord>> {
        self.execute(banner, action, false)
    }
    pub fn pull_trace(&mut self, banner: &str, action: &str) -> Result<Vec<DrawRecord>> {
        self.execute(banner, action, true)
    }

    fn execute(&mut self, banner: &str, action: &str, traced: bool) -> Result<Vec<DrawRecord>> {
        let mut next = self.state.clone();
        let mut rng = self.rng.clone();
        let runtime = &self.engine.runtime;
        let branches = runtime.sample_action(banner, action, &mut next, &self.context, &mut rng)?;
        let mut pre = self.state.clone();
        reset_action_state(runtime.game(), &mut pre);
        let mut records = Vec::with_capacity(branches.len());
        for (i, branch) in branches.iter().enumerate() {
            let mut ctx = self.context.clone();
            ctx.pull = format!("{}:{i}", ctx.pull);
            let trace = if traced {
                Some(runtime.trace_step(banner, action, &pre, &ctx, branch)?)
            } else {
                None
            };
            let item = runtime.game().item(branch.outcome.item);
            records.push(DrawRecord {
                item: self.engine.item(&item.id)?,
                probability: branch.probability,
                guarantees: branch.guarantees.clone(),
                events: branch.events.clone(),
                trace,
            });
            pre = branch.next_state.clone();
        }
        self.state = next;
        self.rng = rng;
        Ok(records)
    }
}
