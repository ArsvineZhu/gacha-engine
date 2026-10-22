# C ABI

`src/lib.rs` is the ABI source. `include/gacha_engine.h` is cbindgen output; a crate build rewrites it when the generated text differs.

Keep the exported operations aligned with the Rust functions listed in `docs/sdk.md`. Single-step probabilities stay decimal strings. Do not add an f64 field to a one-pull branch.

After changing exports, run `cargo build -p gacha-engine-ffi` and update `docs/sdk.md` to match the header.
