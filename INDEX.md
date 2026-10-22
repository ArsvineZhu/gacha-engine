# 仓库地图

人读的说明从 [README.md](README.md) 进入，文档目录在 [docs/INDEX.md](docs/INDEX.md)。

| 路径 | 归属 |
|---|---|
| `src/` | 编译器、检查器、执行器、格式化、命中分析和 `gacha` 命令行 |
| `editors/vscode/` | VS Code 插件。打开三种源文件即高亮；诊断、格式化分别调用 `gacha check --format=json` 和 `gacha fmt`。运行方式在 [开发](docs/development.md) |
| `spec/gacharule/` | 语言规范、正例和 58 个拒绝测试。语义以这里的规范原文为准 |
| `docs/` | 教程、查阅表、开发说明、SDK 和官方数据说明 |
| `docs/examples/` | 教程里的短程序，供 `gacha check` 编译 |
| `official/source/` | 迁移用的目录 JSON 和来源注记。引擎不读取 |
| `official/games/` | 已生成的官方游戏 GachaRule 源。测试编译这些文件 |
| `tools/migrate_official.py` | 从 `official/source/` 重写 `official/games/` |
| `crates/gacha-ffi/` | C ABI。头文件由 cbindgen 生成在 `include/gacha_engine.h` |
| `sdk/python/` | Python 扩展，模块名 `gacha_engine` |
| `sdk/typescript/` | Node 绑定，包名 `gacha-engine` |
| `tests/` | 正例概率、拒绝坐标、命中分析、格式化与 JSON 诊断、官方游戏编译 |

工作区成员是仓库根、`crates/gacha-ffi`、`sdk/python`、`sdk/typescript`。
