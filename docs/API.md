# Library API

The CLI is only a thin adapter. Embedders should call the Rust crates directly.

## Sequential simulation

Use the scope-aware `sample_step` API for ordinary sequential execution. It applies
session/batch/pull lifecycle cleanup before delegating to the exact transition engine.

```rust
use gacha_core::{ScopeContext, SplitMix64, StateStore, sample_step};
use gacha_pack::{compile_pack, load_pack};

let raw = load_pack("packs/endfield/chartered-proportional.json")?;
let game = compile_pack(&raw)?;

let mut state = StateStore::default();
let mut rng = SplitMix64::new(42);
let mut context = ScopeContext::default();

for draw in 1..=10 {
    context.pull = draw.to_string();
    let branch = sample_step(
        &game,
        "endfield.banner.reference",
        "single_pull",
        &mut state,
        &context,
        &mut rng,
    )?;
    println!("item={}", game.item(branch.outcome.item).id);
}
```

For deterministic one-step enumeration, use `enumerate_step`:

```rust
let branches = gacha_core::enumerate_step(
    &game,
    "endfield.banner.reference",
    "single_pull",
    &state,
    &context,
)?;
```

Every returned branch contains:

- exact one-step rational probability;
- resolved item/rarity outcome;
- matching guarantee IDs;
- generic events;
- complete next `StateStore` snapshot.

`enumerate_transitions` and `sample_transition` remain available as lower-level APIs.
They do not advance or prune ephemeral scope lifecycle on behalf of the caller.

## First-hit analysis

`gacha-analysis` provides a serializable query DTO and report. The query target may be:

- one item;
- any item from an explicit set;
- any outcome at or above a rarity;
- any item carrying a tag.

```rust
use gacha_analysis::{FirstHitQuery, TargetSpec, analyze_first_hit};
use gacha_core::{ScopeContext, StateStore};

let query = FirstHitQuery {
    banner: "endfield.banner.reference".into(),
    action: "single_pull".into(),
    target: TargetSpec::Item {
        id: "endfield.operator.current_featured".into(),
    },
    draws: 120,
    quantiles: vec![0.5, 0.9, 0.95, 0.99, 1.0],
};

let report = analyze_first_hit(
    &game,
    &StateStore::default(),
    &ScopeContext::default(),
    &query,
)?;
```

The report contains:

- first-hit PMF for every draw;
- cumulative first-hit probability (CDF);
- survival / no-hit probability;
- finite-horizon conditional and capped expectations;
- requested probability quantiles;
- final and peak surviving-state counts for diagnostics.

One-step probabilities remain exact rationals. Multi-step probability mass is accumulated
in `f64` to avoid unbounded rational denominator growth during long-horizon state merging.

The legacy convenience function `probability_of_item_within` is implemented on top of
this query API.

## JSON query interface

The CLI exposes the same DTO directly:

```bash
cargo run -p gacha-cli -- analyze \
  packs/endfield/chartered-proportional.json \
  examples/endfield-featured-120.query.json
```

The output is JSON and is intended to be reusable by future WASM, HTTP, Python, or Node
bindings without requiring them to parse human-oriented CLI text.

## State persistence

`StateStore` is serializable JSON and uses scope-qualified stable state keys. Persistent
scopes (account/game/pity-group/progress-group/banner) are retained when switching away
and can be restored when returning. Ephemeral session/batch/pull slots are pruned by the
high-level step APIs when their context changes.

Higher-level bindings should preserve the stable string-keyed transport representation
rather than exposing compiled numeric IDs.
