# gacha-engine-ffi

GachaRule 引擎的 C ABI，包名 `gacha-engine-ffi`。函数与 Rust 库是同一组操作，说明见 [SDK](../../docs/sdk.md)。

```sh
cargo build -p gacha-engine-ffi
```

构建时 cbindgen 根据 `src/lib.rs` 和 `cbindgen.toml` 写入 `include/gacha_engine.h`。头文件与源码不一致时，重新构建，而不是只改头文件。

返回值 0 表示成功，1 表示失败。本库分配的指针用 `gacha_free` 归还。
