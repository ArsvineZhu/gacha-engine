# CLI 输入示例

此目录保存分析查询与初始状态输入，不维护游戏卡池定义。卡池资料统一位于 `official/gacha-games/pools/`。

从仓库根目录执行：

```sh
cargo run -p gacha-cli -- analyze official/gacha-games/pools/endfield/banner.chartered-proportional.json examples/cli/endfield-featured-120.query.json
cargo run -p gacha-cli -- transitions official/gacha-games/pools/endfield/banner.chartered-proportional.json endfield.banner.reference single_pull --state examples/cli/state-soft-pity.json
```

完整命令说明见[接口文档](../../docs/API.md)。
