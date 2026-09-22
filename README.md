# gacha-engine

A clean-room, rule-driven gacha engine prototype written in Rust.

The engine does **not** contain game-specific branches. Games are described by untrusted JSON Rule Packs that are parsed, semantically validated, compiled into indexed IR, and then executed by the same transition graph for both probability enumeration and random sampling.

## Status

This is a V0.1 core prototype created without a Rust toolchain in the build sandbox. JSON/TOML/data invariants were checked locally with Python, but the Rust source could **not** be compiled in the sandbox. Run `cargo test --workspace` first on a machine with stable Rust and report any compiler diagnostics before treating the package as production-ready.

Implemented:

- typed state (`counter`, `boolean`)
- state scopes: account, game, pity group, progress group, banner, session, batch, pull
- exact rational one-step probabilities (`u128` numerator/denominator)
- constant, table and linear-after probability curves
- `remainder` and proportional `share_of_remainder`
- selector graph: uniform pools and weighted nested selectors
- guarantee rules: force item, force rarity, minimum rarity
- two minimum-rarity strategies for unknown guarantee behavior
- declarative post-draw state transitions
- generic emitted events
- Rule Pack reference validation and selector-cycle rejection
- reproducible SplitMix64 RNG
- `enumerate_transitions()` as the canonical semantic path
- sampling implemented on top of transition enumeration to prevent semantic drift
- approximate multi-draw target probability DP (f64 mass, exact one-step rules)
- developer CLI
- two Endfield reference packs using different assumptions for unpublished probability details

Not implemented yet:

- optimized direct-sampling hot path
- arbitrary-precision rational multi-draw DP
- batch actions such as an isolated free ten-pull with its own batch guarantee
- dynamic roster queries / automatic previous-banner rotation
- WASM/Python/HTTP bindings
- signatures / remote Rule Pack distribution
- UI

## Requirements

Rust stable with Edition 2024 support. `rust-toolchain.toml` requests the stable channel.

```bash
cargo test --workspace
```

## CLI

Validate a pack:

```bash
cargo run -p gacha-cli -- validate packs/endfield/chartered-proportional.json
```

Inspect it:

```bash
cargo run -p gacha-cli -- inspect packs/endfield/chartered-proportional.json
```

Perform reproducible pulls:

```bash
cargo run -p gacha-cli -- pull \
  packs/endfield/chartered-proportional.json \
  endfield.banner.reference single_pull \
  --count 10 --seed 42
```

Enumerate every branch of one pull:

```bash
cargo run -p gacha-cli -- transitions \
  packs/endfield/chartered-proportional.json \
  endfield.banner.reference single_pull
```

Inspect the first soft-pity step:

```bash
cargo run -p gacha-cli -- transitions \
  packs/endfield/chartered-proportional.json \
  endfield.banner.reference single_pull \
  --state examples/state-soft-pity.json
```

Inspect the 120th counted pull:

```bash
cargo run -p gacha-cli -- transitions \
  packs/endfield/chartered-proportional.json \
  endfield.banner.reference single_pull \
  --state examples/state-120.json
```

Approximate probability of the example featured item within 120 pulls:

```bash
cargo run -p gacha-cli -- probability \
  packs/endfield/chartered-proportional.json \
  endfield.banner.reference single_pull \
  endfield.operator.current_featured 120
```

## Core invariant

The engine follows this conceptual contract:

```text
(State, Action, CompiledRules)
    -> [(Probability, Outcome, NextState, Events)]
```

`sample_transition()` first calls `enumerate_transitions()` and samples from the returned branches. This is intentionally slower than a specialized simulator, but it guarantees that the simulator and analyzer do not implement different gacha semantics.

See `docs/DESIGN.md` and `docs/PACK_FORMAT.md`.
