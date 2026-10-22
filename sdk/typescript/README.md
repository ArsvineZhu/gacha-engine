# Node 绑定

包名 `gacha-engine`，私有包。函数与 Rust 库是同一组操作。对象字段是驼峰形，函数名保持蛇形。详见 [SDK](../../docs/sdk.md)。

```sh
npm run build
npm test
```

`build` 执行 `napi build --platform --release`。`test` 执行 `node --test test/contract.cjs`，不负责重新构建。测试文件用 `require("..")` 加载本目录。
