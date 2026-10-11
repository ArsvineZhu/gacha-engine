# Rust 接口

普通接入使用 [gacha-sdk](SDK.md)。本页描述底层组合接口，供需要自定义执行或验证的场景使用；CLI 使用 SDK 的准备执行器。

## 读取与组装

`load_pack("official/gacha-games/pools/.../banner.*.json")` 从清单目录解析相对规则和内容路径，返回 `GamePack`。`compile_pack` 校验并编译聚合结果。`load_catalog` 校验独立内容目录；`compose_pack(&manifest, &rules, &catalogs)` 提供不读文件的纯组装；`parse_pack` 用于低层聚合文档及合成测试数据。

`CompiledItem` 保留概念、名称、原生稀有度和原始标识。`featured` 仅为清单绑定标签，不修改内容目录。明确不可抽取的条目由编译器排除在池及强制抽取保底之外。

## 顺序抽取

普通单抽使用 `sample_step`，它先清理临时作用域，再进入精确转移引擎：

```rust
use gacha_core::{ScopeContext, SplitMix64, StateStore, sample_step};
use gacha_pack::{compile_pack, load_pack};

let raw = load_pack("official/gacha-games/pools/endfield/banner.chartered-proportional.json")?;
let game = compile_pack(&raw)?;
let mut state = StateStore::default();
let mut rng = SplitMix64::new(42);
let mut context = ScopeContext::default();
for draw in 1..=10 {
    context.pull = draw.to_string();
    let branch = sample_step(
        &game, "endfield.banner.reference", "single_pull",
        &mut state, &context, &mut rng,
    )?;
    println!("{}", game.item(branch.outcome.item).id);
}
```

`enumerate_step` 接收相同游戏、卡池、动作、状态和上下文，枚举所有分支。每条分支包含精确概率、条目/稀有度、匹配保底、事件和完整下一状态。

低层 `enumerate_transitions` / `sample_transition` 由调用方自行维护临时状态生命周期。

## 完整动作

`draws > 1` 的完整动作使用 `sample_action`：

```rust
let results = gacha_core::sample_action(
    &game, "endfield.banner.reference", "free_ten_pull",
    &mut state, &context, &mut rng,
)?;
assert_eq!(results.len(), 10);
```

每次调用启动新批次，即使上下文标识相同；所有抽取成功后提交状态和随机数进度。失败保持调用前状态。事件交给接入应用处理库存和奖励，免费动作自身不校验票券。

## 目标到达分析

`analyze_first_hit` 等同于 `analyze_nth_hit(..., copies = 1)`。`copies` 必须为正，表示此后抽取中匹配目标的次数，不含已有持有量或奖励事件。`draws` 表示底层抽取次数；多抽动作的分析区间必须是动作抽数的整数倍。每次完整动作开始前清空批次与单抽状态，逐抽 PMF/CDF 可在动作边界汇总为批次分布。

```rust
use gacha_analysis::{FirstHitQuery, TargetSpec, analyze_first_hit};
use gacha_core::{ScopeContext, StateStore};

let query = FirstHitQuery {
    banner: "endfield.banner.reference".into(),
    action: "single_pull".into(),
    target: TargetSpec::Item { id: "endfield.character.莱万汀".into() },
    draws: 120,
    quantiles: vec![0.5, 0.9, 0.95, 0.99, 1.0],
};
let report = analyze_first_hit(
    &game, &StateStore::default(), &ScopeContext::default(), &query,
)?;
```

目标还支持显式条目集合、最低稀有度和标签。报告包含每抽到达概率 PMF、累计 CDF、未命中概率、有限区间条件/截断期望、分位数和存活状态数。单步概率保持精确，多步质量用 `f64` 汇总。`probability_of_item_within` 是单目标便捷包装。

## JSON 查询与状态持久化

```sh
cargo run -p gacha-cli -- analyze official/gacha-games/pools/endfield/banner.chartered-proportional.json examples/cli/endfield-featured-120.query.json
```

CLI 接收同一个查询类型并输出 JSON，可供未来其他语言或网页接口使用。

`StateStore` 使用可序列化的稳定字符串键。账户、游戏、保底组、进度组及卡池状态持久保存；会话、批次和单抽槽位由高层接口清理。接入层应保存字符串传输形式，避免持久化编译后的数字索引。
