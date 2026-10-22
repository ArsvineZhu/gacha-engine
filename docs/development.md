# 开发

构建和测试的命令以仓库根的 `Cargo.toml` 与 `rust-toolchain.toml` 为准。

## 环境

`rust-toolchain.toml` 使用 stable，并安装 clippy 与 rustfmt。包要求 `rust-version = "1.85"`，edition 2024。

工作区成员：

| 包 | 路径 | 产物 |
|---|---|---|
| `gacha-engine` | 仓库根 | 库，以及二进制 `gacha`（`src/main.rs`） |
| `gacha-engine-ffi` | `crates/gacha-ffi` | C ABI：rlib、staticlib、cdylib。构建时 cbindgen 写 `crates/gacha-ffi/include/gacha_engine.h` |
| `gacha-engine-py` | `sdk/python` | Python 扩展，模块名 `gacha_engine`。Cargo 测试关闭，合同测试是 unittest |
| `gacha-engine-node` | `sdk/typescript` | Node 扩展。Cargo 测试关闭，合同测试是 `node --test` |

## 构建与测试

在仓库根：

```sh
cargo test
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/genshin.gmr spec/gacharule/positive/weapon_ten.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`cargo test` 覆盖：

| 测试 | 文件 | 它固定的事实 |
|---|---|---|
| `positive_examples_match_exact_probabilities` | `tests/corpus.rs` | 正例按规范第 8.6 节的有理数概率 |
| `rejection_corpus_matches_codes_and_positions` | `tests/corpus.rs` | 58 个拒绝目录的错误码和 `文件:行:列` |
| `official_games_compile`、`official_base_rates_match_the_old_rules` | `tests/official.rs` | 七款官方源能编译，并核对若干基础概率 |
| `positive_hits_match_exact_step_probabilities` 等 | `tests/analysis.rs` | 命中分析、快照恢复，以及分析不改写会话 |
| `positive_format_is_stable_and_compiles` | `tests/fmt.rs` | 正例格式化两次不变，压平后再格式化仍回到原文，并且格式化结果能编译 |
| `check_json_and_fmt_cli` | `tests/fmt.rs` | `gacha check --format=json` 与 `gacha fmt` 的进程输出 |

只跑其中一个文件：

```sh
cargo test --test corpus
cargo test --test official
cargo test --test analysis
cargo test --test fmt
```

教程短程序：

```sh
cargo run -- check docs/examples/mini/mini.gmr docs/examples/mini/mini.gml docs/examples/mini/mini.gmb
cargo run -- check docs/examples/hard/hard.gmr docs/examples/hard/hard.gml docs/examples/hard/hard.gmb
cargo run -- check docs/examples/soft/soft.gmr docs/examples/soft/soft.gml docs/examples/soft/soft.gmb
```

C ABI：

```sh
cargo build -p gacha-engine-ffi
```

Python 绑定由 `sdk/python/pyproject.toml` 声明，构建后端是 maturin，要求 Python 3.9 或更新。在 `sdk/python`：

```sh
maturin develop
python -m unittest tests/test_contract.py
```

Node 绑定在 `sdk/typescript`。`package.json` 的 `build` 是 `napi build --platform --release`，`test` 是 `node --test test/contract.cjs`。先构建再测试：

```sh
npm run build
npm test
```

官方源的重新生成见 [官方游戏数据](official-games.md)。

## 命令行

`gacha` 接受 `check` 和 `fmt`。用法行是：

```text
用法:
  gacha check [--format=text|json] [--stdin 名称] 文件...
  gacha fmt [--stdin 名称] [文件]
```

`check` 的文件按依赖顺序给出。诊断里的 `file` 就是命令行上的那个路径字符串。`--stdin 名称` 从标准输入读取该名称对应的文本；只写 `--stdin` 时，这个名称就是唯一文件，同时列出多个文件时，名称必须是其中之一。扩展名仍决定文法。

| 情况 | 退出码 |
|---|---|
| `check` 编译成功，默认文本格式 | 0，标准输出为空 |
| `check --format=json` 编译成功 | 0，标准输出 `{"diagnostics":[]}` |
| 读文件失败，或编译被拒绝 | 1 |
| `fmt` 成功 | 0，标准输出是格式化后的全文 |
| `fmt` 遇到词法或语法错误 | 1，标准错误一行 `路径:行:列: 错误码 期望 …，实际 …`，不改写文件 |
| 缺少子命令、子命令未知，或参数不完整 | 2，并在标准错误打印用法 |

文本格式的编译失败写到标准错误：

```text
路径:行:列: 错误码 实际内容
```

`--format=json` 把零条或一条诊断写到标准输出。每条诊断含 `code`、`file`、`line`、`column`、`expected`、`actual`。读文件失败时 `code` 为 `E_IO`。

`fmt` 只重排换行、四空格缩进、冒号引块和词间空格，token 文本保持原样。多个 `banner` 之间空一行。它把结果打到标准输出，不写回原文件。

库类型 `Diagnostic` 同时带有 `expected` 与 `actual`。`format_source` 是同一格式化器的库入口。`load` 把编译失败收成 `RuntimeError`：`code` 仍是编译错误码，`message` 含 `文件:行:列: 期望 …，实际 …`。

## 编辑器

插件在 `editors/vscode`。`.gmr`、`.gml`、`.gmb` 注册为同一种语言 `GachaRule`。语法高亮按规范第 1–3 节的词法；文件以 `rule`、`roster` 或 `banner` 开头时，不属于该文件的顶层关键字标成非法关键字。

| 能力 | 触发 | 调用 |
|---|---|---|
| 高亮 | 打开这三种文件 | TextMate 语法，不调用 `gacha` |
| 诊断 | 打开时、保存时，以及修改后约 200 毫秒 | `gacha check --format=json` |
| 格式化 | 编辑器命令「格式化文档」，默认快捷键 `Shift+Alt+F` | `gacha fmt --stdin`。插件不打开 `editor.formatOnSave` |
| 补全 | 输入，以及触发字符 `@`、`.` | 只看当前文件已写下的名字，再加上关键字、类型名、`true`/`false` 和作用域 `@pity`、`@progress`、`@banner`、`@action` |
| 悬停 | 光标放在词上 | 本文件里的名字显示声明类型。关键字、类型名、作用域和布尔字面量显示对应说明 |

同一目录里还有其他 `.gmr`、`.gml`、`.gmb` 时，插件按扩展名（机制、名册、绑定）再按文件名排序，把整组交给编译器。当前文件用编辑器里的文本，同目录的其他文件用磁盘上的内容。只检查当前文件时，词法和语法错误画在这个文件上。

在仓库根执行 `cargo build` 或 `cargo test` 后，二进制在 `target/debug/gacha`。用 VS Code 打开 `editors/vscode`，按 F5，启动扩展开发主机。插件按这个顺序找可执行文件：

1. 设置项 `gacharule.gachaPath`（默认空）
2. 从当前文件向上，先 `target/debug/gacha`，再 `target/release/gacha`
3. `PATH` 里的 `gacha`

找不到时，诊断是一条警告：先 `cargo build`，或设置 `gacharule.gachaPath`。

插件测试不需要安装 npm 依赖：

```sh
node --test editors/vscode/test/*.js
```

在 `editors/vscode` 里执行 `npm test` 是同一条测试。插件自己的说明在 [editors/vscode/README.md](../editors/vscode/README.md)。

## 目录归属

| 路径 | 谁改它 |
|---|---|
| `src/lex.rs`、`src/parse.rs`、`src/check.rs` | 词法、文法和编译期检查。改语义时先改规范与拒绝测试 |
| `src/fmt.rs` | 格式化。诊断仍由 `check` 产生 |
| `src/exec.rs`、`src/analysis.rs`、`src/rational.rs` | 单步执行、抽样和命中分析 |
| `src/lib.rs`、`src/main.rs` | 公开 Rust API、`gacha check` 与 `gacha fmt` |
| `editors/vscode/` | VS Code 插件。类型诊断调用 `gacha check --format=json` |
| `spec/gacharule/` | 规范、正例、拒绝语料。编译器必须跟上这里的错误码和坐标 |
| `docs/examples/` | 教程短程序。与 [语言教程](language-tutorial.md) 中的源文本保持一致 |
| `official/source/` | 迁移输入。引擎不编译 JSON |
| `official/games/` | `tools/migrate_official.py` 的输出。脚本会先删除该目录下已有的 `.gmr`、`.gml`、`.gmb` |
| `crates/gacha-ffi/src/lib.rs` | C ABI。头文件在构建时重写，手改 `include/gacha_engine.h` 会被下一次构建覆盖 |
| `sdk/python/src/lib.rs`、`sdk/typescript/src/lib.rs` | 绑定。函数集合与 Rust 公开操作对齐，合同测试在各自的 `tests/` 或 `test/` |

调用形状见 [SDK](sdk.md)。语言规则见 [语言参考](language-reference.md)。
