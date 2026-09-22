# Library API sketch

The CLI is only a thin adapter. Embedders should call the Rust crates directly.

```rust
use gacha_core::{ScopeContext, SplitMix64, StateStore, sample_transition};
use gacha_pack::{compile_pack, load_pack};

let raw = load_pack("packs/endfield/chartered-proportional.json")?;
let game = compile_pack(&raw)?;

let mut state = StateStore::default();
let mut rng = SplitMix64::new(42);
let context = ScopeContext::default();

let branch = sample_transition(
    &game,
    "endfield.banner.reference",
    "single_pull",
    &mut state,
    &context,
    &mut rng,
)?;

println!("item={}", game.item(branch.outcome.item).id);
```

For deterministic analysis, call:

```rust
let branches = gacha_core::enumerate_transitions(
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

`StateStore` is serializable JSON and uses scope-qualified stable state keys. This is the current persistence/data interchange mechanism. Higher-level bindings should preserve it rather than exposing compiled numeric IDs.
