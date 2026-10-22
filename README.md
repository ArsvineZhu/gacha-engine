# gacha-engine

GachaRule 1.0 的编译器和精确执行器。机制、名册和卡池绑定分别写在 `.gmr`、`.gml`、`.gmb` 里。单步抽取的概率是有理数，用分子和分母表示。

版本 0.1.0，许可证见 [LICENSE](LICENSE)（MIT）。

## 当前能力

- 按显式文件列表编译一个单元，并检查词法、类型、分布和绑定。
- 命令行只有 `gacha check`，用来确认这组源文件能够编译。
- Rust 库提供同一组操作：编译、列出卡池与动作、读写计数和布尔状态、查看单步精确分支、按条目提交、按种子抽样、做首次或第 N 次命中分析。C、Python 和 Node 绑定调用这同一组操作。
- 单步分支保持有理数。命中分析把多步质量汇总成浮点数，并且不改写当前会话。
- 规范正例在 `spec/gacharule/positive/`。七款官方游戏的 GachaRule 源在 `official/games/`，由 `tools/migrate_official.py` 从 `official/source/` 的 JSON 生成。运行时只编译 `.gmr`、`.gml`、`.gmb`。

语言语义以 [GachaRule 1.0 规范](spec/gacharule/GachaRule-Language-v1.md) 为准。

## 最快上手

工具链见 `rust-toolchain.toml`：stable，并包含 clippy 与 rustfmt。`Cargo.toml` 要求 Rust 1.85。

```sh
cargo test
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/genshin.gmr spec/gacharule/positive/weapon_ten.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`check` 成功时没有输出，退出码为 0。文件按依赖顺序给出：被引用的规则和名册在卡池绑定之前。

写第一条规则见 [语言教程](docs/language-tutorial.md)。构建、测试和目录归属见 [开发](docs/development.md)。

## 文档地图

| 目的 | 文档 |
|---|---|
| 仓库有哪些区域 | [INDEX.md](INDEX.md) |
| 文档怎么读 | [docs/README.md](docs/README.md) |
| 写规则 | [语言教程](docs/language-tutorial.md) |
| 查词法、类型、保底和错误类别 | [语言参考](docs/language-reference.md) |
| 调用 Rust、C、Python、Node | [SDK](docs/sdk.md) |
| 官方游戏数据和名册生成 | [官方游戏数据](docs/official-games.md) |
