# Node binding

Expose the same operations as the Rust library. NAPI object fields are camelCase; function names stay snake_case. `gacha_session_sample` takes the seed as a decimal string that must fit in `u64`.

Hit-report state counts are `u32` in this binding and saturate at `u32::MAX`. Rust, C, and Python keep `u64`.

```sh
npm run build
npm test
```

Cargo tests are disabled. When a public function changes, update `src/lib.rs`, `test/contract.cjs` when the contract is affected, and `docs/sdk.md`.
