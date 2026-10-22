# gacha-engine

GachaRule 1.0 的编译器和精确执行器。机制、名册、卡池绑定分别写在 `.gmr`、`.gml`、`.gmb` 里。概率在编译期和单步执行中保持为有理数。

规范与语料在 `spec/gacharule/`。

```sh
cargo test
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`check` 的文件必须按依赖顺序给出：被引用的规则和名册在卡池绑定之前。

跨语言入口是同一组操作：编译、列出卡池与动作、读写计数和布尔状态、查看单步精确分支、按条目提交、按种子抽样、以及首次或第 N 次命中分析。C ABI 在 `crates/gacha-ffi`，头文件由 cbindgen 生成。Python 包在 `sdk/python`，Node 绑定在 `sdk/typescript`。单步概率用分子和分母的十进制字符串传递。

七款官方游戏的 GachaRule 源在 `official/games/`。`tools/migrate_official.py` 从 `official/source/` 的目录 JSON 重新生成它们；运行时只编译 `.gmr`、`.gml`、`.gmb`。
