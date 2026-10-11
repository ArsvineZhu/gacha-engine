# 丹青渡 SDK 接入示例

此示例具有独立 Cargo 工作区和锁文件，依赖公开的 `gacha-sdk`、官方资料库 `gacha-games` 与 JSON 序列化库。`cases.json` 通过卡池及机制标识引用官方库，卡池资料不在示例中重复维护。

从本目录执行：

```sh
cargo run --release --locked -- 10000
```

程序通过 SDK 发现动作、完成分析与模拟、导出 JSON/CSV、获取 trace，并验证会话保存后的续跑。完整动作的期望、分位数及模拟对照由 SDK 提供，示例不实现概率算法。

临时输出位于 `output/`，其中聚合数据包仅用于 CLI 接入验证。经过门禁验证的报告、CSV 和 trace 保存在本示例的 `reports/`，结果见[测试报告](REPORT.md)。`baseline.json` 只保存三种模型的理论 CDF 和六个场景的种子频数哈希，避免复制整份历史报告。

从仓库根目录运行 `python tools/check_examples.py`，可将此项目复制到系统临时目录中构建，再在同一独立目录调用公开 CLI 核对六个场景。项目尚未发布，路径依赖可在发布后替换为对应版本的注册表依赖。
