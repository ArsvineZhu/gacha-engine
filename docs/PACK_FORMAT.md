# Rule Pack format (schema version 1)

The authoritative machine representation is the Rust structs in `crates/gacha-schema/src/lib.rs`. This document describes the intended semantics.

## Top level

```json
{
  "schema_version": 1,
  "id": "example.game",
  "version": "0.1.0",
  "sources": [],
  "assumptions": [],
  "state": [],
  "items": [],
  "pools": [],
  "distributions": [],
  "selectors": [],
  "banners": []
}
```

## State

Supported kinds:

- `counter`
- `boolean`

Supported scopes:

- `account`
- `game`
- `pity_group`
- `progress_group`
- `banner`
- `session`
- `batch`
- `pull`

Example:

```json
{
  "id": "six_star_pity",
  "kind": "counter",
  "scope": "pity_group",
  "initial": 0,
  "max": 79
}
```

## Probability expressions

### Constant

```json
{"rarity": 5, "type": "constant", "value": "0.08"}
```

### Linear after N failures

`after: 65` means that state values 0..64 use `base`, while state value 65 receives the first increment. Thus the next draw after 65 failures is the first increased-probability draw.

```json
{
  "rarity": 6,
  "type": "linear_after",
  "state": "six_star_pity",
  "base": "0.008",
  "after": 65,
  "increment": "0.05",
  "cap": "1"
}
```

### Table

```json
{
  "rarity": 6,
  "type": "table",
  "state": "pity",
  "values": {"70": "0.2", "71": "0.25"},
  "default": "0.01"
}
```

### Remainder

```json
{"rarity": 4, "type": "remainder"}
```

Receives `1 - sum(fixed probabilities)`.

### Share of remainder

```json
{"rarity": 5, "type": "share_of_remainder", "weight": 80}
{"rarity": 4, "type": "share_of_remainder", "weight": 912}
```

All such entries divide the remaining probability in proportion to their integer weights.

## Selectors

Uniform pool:

```json
{"id": "selector.five", "type": "pool", "pool": "pool.five"}
```

Weighted nested selector:

```json
{
  "id": "selector.six",
  "type": "weighted",
  "branches": [
    {"weight": "1", "selector": "selector.current"},
    {"weight": "1", "selector": "selector.off_banner"}
  ]
}
```

Weights are relative, not required to sum to one.

## Conditions

Supported `op` values:

- `always`
- `all`
- `any`
- `not`
- `state_counter_gte`
- `state_counter_eq`
- `state_bool_eq`
- `state_modulo_eq`
- `outcome_rarity_eq`
- `outcome_rarity_gte`
- `outcome_item_in_pool`

## Guarantees

Force item:

```json
{
  "id": "featured_120",
  "priority": 300,
  "when": {"op": "state_counter_gte", "state": "banner_pulls", "value": 119},
  "effect": {"type": "force_item", "item": "example.item"}
}
```

Force rarity:

```json
{"effect": {"type": "force_rarity", "rarity": 6}}
```

Minimum rarity with conditional renormalization:

```json
{
  "effect": {
    "type": "min_rarity",
    "rarity": 5,
    "strategy": "conditional_current"
  }
}
```

Minimum rarity while preserving higher-rarity mass:

```json
{
  "effect": {
    "type": "min_rarity",
    "rarity": 5,
    "strategy": "preserve_higher_fill_floor"
  }
}
```

## State updates

Supported update types:

- `increment`
- `reset`
- `set_counter`
- `set_bool`

## Events

Events are opaque engine outputs for higher layers:

```json
{"kind": "grant_currency", "key": "arsenal_quota", "amount": 2000}
```

The core does not know what that currency means and does not mutate inventory itself.
