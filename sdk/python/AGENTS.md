# Python binding

The public surface is `Engine` and `Session` methods, plus `Item`, `AnyOf`, `AtLeastRarity`, and `Tag`. Count and probability values are decimal strings. `Session.sample` takes a `u64` seed. Keep docstrings on every public class and method; the editor stub `gacha_engine.pyi` must carry the same text.

The contract test is `python -m unittest tests/test_contract.py` after `maturin develop` in this directory. Cargo tests are disabled.

When a public function changes, update `src/lib.rs`, `tests/test_contract.py` when the contract is affected, and `docs/sdk.md`.
