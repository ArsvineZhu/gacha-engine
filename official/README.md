# 官方游戏数据

`source/` 是迁移输入：每个游戏目录里有 `rule.*.json`、`banner.*.json`、`catalog.*.json`，以及给人看的 `sources.json`。

`games/` 是生成出的 GachaRule 源。引擎和 `tests/official.rs` 只编译这里的 `.gmr`、`.gml`、`.gmb`。

重新生成的命令、文件命名和名册来源见 [官方游戏数据](../docs/official-games.md)。
