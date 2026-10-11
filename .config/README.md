# 开发工具配置

工具配置集中在此目录，脚本实现位于 `tools/`。此目录管理开发工具策略，不包含业务运行参数。

| 配置 | 用途 |
| --- | --- |
| `rustfmt.toml` | Edition 2024、100 列、Unix 换行与导入排序 |
| `deny.toml` | 安全公告、撤回版本、许可证、重复版本和依赖来源 |
| `duplication.json` | Rust/Python 业务代码重复率；最少 8 行/70 个词元，5% 上限 |
| `tools.toml` | 独立工具的固定版本，安装器与 CI 共用 |
| `scripts/code-length.toml` | 维护文本最多 600 行，测试不能进入业务源文件 |
| `scripts/doc-navigation.toml` | 文档本地文件目标与标题锚点检查 |

独立工具包括 [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) 0.20.2、[cargo-machete](https://github.com/bnjbvr/cargo-machete) 0.9.2 和 [jscpd](https://github.com/kucherenko/jscpd) 5.4.1。fmt 和 Clippy 随固定 Rust 工具链安装。jscpd 扫描业务代码和工具脚本；测试中的预期值、真实内容记录和规则声明不进入重复代码统计。

```sh
python tools/install_tools.py
python tools/check.py
cargo fmt-project
cargo fmt-check
cargo deny-project
cargo machete
jscpd --config .config/duplication.json crates tools
```

[.cargo/config.toml](../.cargo/config.toml) 中的别名显式指定集中配置路径。直接执行 `cargo fmt` 不会读取这个目录，因此项目格式化使用 `cargo fmt-project`；编辑器可将 `.config/rustfmt.toml` 作为 rustfmt 的 `--config-path` 参数。

`cargo-deny` 的重复依赖版本检查与 jscpd 的重复代码检查各有独立职责。未使用依赖由 machete 检查。当前依赖图禁止重复版本，许可证允许 MIT、Apache-2.0 和 Unicode-3.0。

文档门禁检查仓库内目标和标题。外部游戏页面的事实与可访问性在资料更新时核对，避免网络波动使每次代码检查失败。完整目录见[架构](../docs/ARCHITECTURE.md)，数据来源见[内容说明](../docs/CONTENT.md)。
