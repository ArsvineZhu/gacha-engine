# Design

## 1. Rules are data

`gacha-core` has no Endfield, Arknights, Genshin, Wuthering Waves, character, operator, or weapon logic. It only knows items, pools, selectors, states, probabilities, guarantees, transitions, and events.

A game-specific Rule Pack is parsed by `gacha-schema`, validated/compiled by `gacha-pack`, and then represented with numeric IDs in `gacha-core`.

## 2. State scope is explicit

Every state variable declares a lifecycle scope. A state slot key is resolved from the current banner and execution context.

Examples:

```text
six_star_pity::pity:endfield.chartered
banner_pulls::progress:endfield.banner.reference
```

This deliberately separates "what the state means" from "how long it survives". Cross-banner pity and banner-local reward progress therefore do not need special engine code.

## 3. Non-Turing-complete Rule Packs

The JSON format supports a closed set of conditions, probability expressions, selector nodes, and state updates. It does not execute JavaScript, Python, Rust, shell commands, network calls, filesystem calls, loops, recursion, or arbitrary expressions.

Selector recursion is data recursion only and cycles are rejected by the compiler.

## 4. Probability semantics

One-step probabilities use an exact reduced `u128/u128` rational type. Decimal strings such as `"0.008"` are parsed exactly as rational values.

This is sufficient for rule evaluation and one-step enumeration. It is intentionally **not** claimed to be an arbitrary-precision exact long-horizon solver: denominator growth can exceed `u128` in repeated symbolic arithmetic. `gacha-analysis` therefore accumulates long-horizon state mass in `f64` in V0.1 while obtaining every one-step branch from the exact rule engine.

A later exact analyzer should replace this with an arbitrary-precision rational or a specialized probability representation without changing Rule Pack semantics.

## 5. Guarantee priority

Matching guarantees are ordered by descending `priority`.

The highest-priority matching guarantee is the effective guarantee. Its effect is applied exactly once (`force_item`, `force_rarity`, or `min_rarity`). All matching guarantee IDs are still returned for diagnostics. Packs should avoid equal-priority contradictory guarantees.

## 6. Unknown rules are models, not facts

The two Endfield reference packs intentionally disagree about unpublished probability redistribution details:

- `chartered-proportional.json`: increased 6-star probability takes mass from 5/4 proportionally; the 5+ guarantee conditions the current distribution on rarity >= 5.
- `chartered-fixed-five.json`: 5-star remains 8% while 4-star receives the remainder; the 5+ guarantee preserves 6-star probability and fills 5-star with the excluded mass.

Both packs use the exact same Rust engine.

This is the intended mechanism for empirical revisions: replace model data, not engine code.

## 7. Transition semantics

Conditions in post-draw transition rules are evaluated against the same pre-draw state plus the resolved outcome. Matching rule updates are then applied in declaration order to the next state.

Packs should use mutually exclusive rules where multiple writes to the same state would be ambiguous.

## 8. Security model

Treat Rule Packs as untrusted input.

Current defenses:

- strict typed deserialization;
- explicit ID resolution;
- unknown reference rejection;
- state-kind checks;
- selector cycle detection;
- empty pool rejection;
- duplicate ID rejection;
- probability range checks;
- transition probability sum validation;
- checked integer/rational arithmetic in the engine.

Future remote/community pack support should additionally impose document size, node count, graph depth, state-space, and runtime budgets, plus canonical hashing/signatures where appropriate.
