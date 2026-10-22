# 文档

这些文档给写规则的人、调用引擎的人，以及改编译器或绑定的人。说明使用简体中文。`AGENTS.md` 使用英文，只写给代理的操作约束。

语言语义的准绳是 [GachaRule 1.0 规范](../spec/gacharule/GachaRule-Language-v1.md)。[语言参考](language-reference.md) 按查阅顺序整理同一套规则，并写明当前执行器的调用边界。完整 EBNF 和 58 个拒绝坐标留在规范与 [拒绝测试清单](../spec/gacharule/rejection-tests/README.md)。

## 阅读顺序

| 读者 | 顺序 |
|---|---|
| 写规则 | [语言教程](language-tutorial.md) → [语言参考](language-reference.md) → 规范原文 |
| 调用引擎 | [SDK](sdk.md) |
| 构建和改代码 | [开发](development.md) |
| 官方游戏数据 | [官方游戏数据](official-games.md) |

目录见 [INDEX.md](INDEX.md)。仓库区域见 [../INDEX.md](../INDEX.md)。

## 事实放在哪里

| 事实 | 所在文档 |
|---|---|
| 构建、测试、退出码、目录归属 | [开发](development.md) |
| VS Code 插件的运行方式 | [editors/vscode/README.md](../editors/vscode/README.md) |
| 怎么写规则，以及教程短程序 | [语言教程](language-tutorial.md) 与 `examples/` |
| 词法、类型、声明、保底、更新、编译错误类别 | [语言参考](language-reference.md)；语义冲突时以规范原文为准 |
| 拒绝测试的文件、行列 | [rejection-tests/README.md](../spec/gacharule/rejection-tests/README.md) |
| Rust、C、Python、Node 的函数和运行时错误 | [SDK](sdk.md) |
| 官方源文件位置和名册生成 | [官方游戏数据](official-games.md) |
