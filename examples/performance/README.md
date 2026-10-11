# 性能对照示例

从仓库根目录执行：

```sh
cargo run --manifest-path examples/performance/Cargo.toml --release --locked
```

官方库提供角色和武器规则，通用解释器与公开 SDK 分别执行同一组分析和固定种子模拟。每项重复三次，报告记录中位数、倍率及一致性校验，保存于本示例的 [reports/performance.json](reports/performance.json)。

本示例为了比较底层算法，直接依赖核心、分析和编译接口。普通应用接入见[SDK 示例](../sdk/endfield-danqingdu/README.md)。算法依据与测量口径见[性能说明](../../docs/PERFORMANCE.md)。
