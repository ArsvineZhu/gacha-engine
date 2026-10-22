# Python binding

Expose the same operations as the Rust library. Count and probability values are decimal strings. `gacha_session_sample` takes a `u64` seed.

The contract test is `python -m unittest tests/test_contract.py` after `maturin develop` in this directory. Cargo tests are disabled.

When a public function changes, update `src/lib.rs`, `tests/test_contract.py` when the contract is affected, and `docs/sdk.md`.
