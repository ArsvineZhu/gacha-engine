# Node binding

Expose `Engine` and `Session` as classes. Method names are camelCase. `session.sample` takes the seed as a decimal string that must fit in `u64`. Targets are `Item`, `AnyOf`, `AtLeastRarity`, and `Tag`.

Hit-report state counts are `u32` in this binding and saturate at `u32::MAX`. Rust, C, and Python keep `u64`.

```sh
npm run build
npm test
```

Cargo tests are disabled. When a public function changes, update `src/lib.rs`, `test/contract.cjs` when the contract is affected, and `docs/sdk.md`.
