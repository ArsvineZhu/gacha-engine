# 官方示例

| 目录 | 功能 | 结果位置 |
| --- | --- | --- |
| [sdk/endfield-danqingdu](sdk/endfield-danqingdu/README.md) | 外部应用通过公开 SDK 与官方游戏库进行分析、模拟、追踪和会话续跑 | 示例内 `reports/` |
| [performance](performance/README.md) | 解释器与准备执行器的性能及结果一致性对照 | 示例内 `reports/` |
| [cli](cli/README.md) | 命令行查询和状态输入 | 运行时标准输出 |

Rust 示例使用独立 Cargo 工作区和锁文件。`tools/check_examples.py` 将 SDK 示例复制到仓库外，通过公开依赖和 CLI 验证接入流程。
