# Repository agent rules

Human documentation is Simplified Chinese. This file and other `AGENTS.md` files stay in English.

## Layout

- `src/` owns the compiler, checker, executor, hit analysis, and the `gacha` binary.
- `spec/gacharule/GachaRule-Language-v1.md` is the normative language. Do not invent syntax or describe a host feature as available when the current functions do not provide it.
- `official/source/` is migration input. The engine compiles only `.gmr`, `.gml`, and `.gmb`.
- `tools/migrate_official.py` deletes `official/games/<game>/*.{gmr,gml,gmb}` and rewrites them. Do not treat hand edits there as durable unless the generator is updated too.
- `crates/gacha-ffi/include/gacha_engine.h` is generated from `crates/gacha-ffi/src/lib.rs` by cbindgen during the crate build.

## Commands

```sh
cargo test
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/genshin.gmr spec/gacharule/positive/weapon_ten.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`gacha check` is the only CLI subcommand. Pass sources in dependency order.

## Host contract

- `draw`, `apply`, and `sample` advance one pull. They do not reset `@action`.
- `draw` does not write the session. `apply` and `sample` commit the chosen branch.
- `Session::analyze` is the call that resets `@action` at each action boundary, on a copy. It does not mutate the session. Multi-step masses are `f64`.
- There is no public function that commits N pulls as one action.
- Single-step probabilities are `u128` rationals. Arithmetic overflow panics; it does not wrap and it is not a `RuntimeError`.

## Documentation sync

- Public Rust, C, Python, or Node functions: update `docs/sdk.md`.
- Language rules: update the spec and, when the lookup would be wrong, `docs/language-reference.md`.
- Tutorial programs: keep `docs/examples/` identical to the fenced programs in `docs/language-tutorial.md`.
- Build, test, or directory ownership: update `docs/development.md`.
- Official migration: update `docs/official-games.md`.
