# gacha-engine

A clean-room, rule-driven gacha engine prototype written in Rust.

The engine does **not** contain game-specific branches. Games are described by untrusted JSON Rule Packs that are parsed, semantically validated, compiled into indexed IR, and then executed by the same transition graph for both probability enumeration and random sampling.

## Status

The V0.1 baseline on `main` has been built successfully by the maintainer on a local Rust toolchain. The current V0.2 development work adds the analysis query API and explicit ephemeral-scope lifecycle handling. The ChatGPT build sandbox still has no Rust toolchain, so changes on the V0.2 branch must be verified with `cargo test --workspace` before merge.

Implemented:

- typed state (`counter`, `boolean`)
- state scopes: account, game, pity group, progress group, banner, session, batch, pull
- scope-aware high-level step APIs that prune stale session/batch/pull slots
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
- transition enumeration as the canonical semantic path
- sampling implemented on top of transition enumeration to prevent semantic drift
- query-driven finite-horizon first-hit analysis using the same transition graph
- first-hit PMF/CDF, survival probability, quantiles and finite-horizon expectations
- analysis targets by item, item set, minimum rarity, or item tag
- serializable analysis query/report DTOs suitable for future bindings
- developer CLI
- two Endfield reference packs using different assumptions for unpublished probability details

Not implemented yet:

- optimized direct-sampling hot path
- arbitrary-precision rational multi-draw DP
- state projection / dependency reduction for large exact-analysis state spaces
- copy-count and multi-objective probability queries
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

Run the query-driven first-hit analysis API:

```bash
cargo run -p gacha-cli -- analyze \
  packs/endfield/chartered-proportional.json \
  examples/endfield-featured-120.query.json
```

The legacy convenience command remains available:

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

For normal sequential use, call `sample_step()` / `enumerate_step()`. They apply the lifecycle rules for ephemeral scopes and then delegate to the canonical transition engine. The lower-level `sample_transition()` / `enumerate_transitions()` remain available when a caller intentionally wants to manage scope lifecycle itself.

Sampling still derives from exact transition enumeration. This is intentionally slower than a specialized simulator, but it prevents the simulator and analyzer from implementing different gacha semantics.

See `docs/DESIGN.md`, `docs/API.md`, and `docs/PACK_FORMAT.md`.
