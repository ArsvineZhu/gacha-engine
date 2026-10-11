# SDK 接入

`gacha-sdk` 是第一方接入入口，统一加载、规则准备、会话、分析、模拟和 trace。接入方无需调用编译器、操作数字索引或实现概率计算。版本为 `0.1.0`，当前使用本地路径依赖。

```toml
[dependencies]
gacha-sdk = { path = "../gacha-engine/crates/gacha-sdk", version = "=0.1.0" }
serde_json = "1.0"
```

## 加载与发现

```rust
use gacha_sdk::{Engine, AnalysisRequest, TargetSpec};

let engine = Engine::load("banner.json")?;
let actions = engine.actions();
let action = &actions[0];
let item = engine.item("endfield.weapon.玉中身")?;
```

`load` 接受卡池清单或独立数据包。清单按所在目录解析规则与内容，独立包包含已组装数据，不依赖原目录。`load_with_rules` 替换规则配置，并重新执行原清单的目标与作用域绑定。`from_pack` 接受自定义聚合数据，仍进行语义校验。

使用官方游戏资料时，接入应用另行依赖 `gacha-games`，调用 `gacha_games::load("endfield/banner.preview-si.json")` 即可取得相同的 SDK 引擎。官方库负责内嵌规则、内容、卡池名单与来源，SDK 保持游戏无关。完整接入见[官方示例](../examples/sdk/endfield-danqingdu/README.md)。

`actions` 返回卡池、动作、单次动作抽数、保底组与进度组；`item` 返回稳定标识、名称、稀有度、装备概念、本地化名称和上游标识。`sources` 与 `assumptions` 返回资料依据和模型假设。

## 分析与模拟

```rust
let request = AnalysisRequest::first_hit(
    &action.banner, &action.action,
    TargetSpec::Item { id: item.id.clone() }, 8,
);
let report = engine.study(&request, 10_000, 20261011)?;
report.write_json("report.json")?;
report.write_csv("distribution.csv")?;
```

`actions` 的单位是完整动作。武库申领输入 8 表示八次申领，报告同时给出 80 件的逐抽分布与八次的逐动作分布。首次命中位置和实际执行的完整动作期望分别计算。

请求支持目标条目、条目集合、最低稀有度和标签。`copies` 表示此后第 N 次目标命中，不包含已有库存。初始 `state` 与 `context` 可指定，模拟的每个独立账户均从相同初始条件开始。

| 接口 | 返回内容 |
| --- | --- |
| `analyze` | 逐抽及逐动作 PMF/CDF、未命中质量、分位数、条件与截断期望、状态数、规则指纹、来源和假设 |
| `simulate` | 随机模拟频数、累计概率、未命中账户数、完整动作平均投入、缓存统计 |
| `study` | 分析、模拟及逐点统计对照，支持第一方 JSON/CSV 导出 |
| `transitions` | 单步精确分支、结果、保底、事件和下一状态 |

单步概率使用 `u128` 有理数，多步分析质量以 `f64` 汇总。模拟对照的六倍标准差检查属于统计检查，精确分支与解释器差分另行验证。免费动作与阶段奖励是否执行由接入策略决定，奖励事件不计入抽取目标命中。

## 会话与 trace

```rust
let mut session = engine.session(42);
let records = session.pull_trace(&action.banner, &action.action)?;
let saved = serde_json::to_string(&session.snapshot())?;
let mut restored = engine.restore_session(serde_json::from_str(&saved)?)?;
let next = restored.pull(&action.banner, &action.action)?;
```

`pull` 原子执行完整动作，失败时保持状态和随机数进度。`pull_trace` 增加候选精确概率、全部保底检查、优先级与效果、生效保底、抽后规则检查、状态变化和事件。

快照包含格式版本、引擎版本、SHA-256 规则指纹、状态、上下文和随机数状态。恢复会话时校验版本与指纹，防止将旧状态用于不同规则。`set_state` 校验状态类型与范围；`set_context` 支持应用自行划分会话作用域。库存、票券与支付授权由应用维护。

## CLI

```sh
gacha bundle banner.json standalone.json
gacha study standalone.json request.json --trials 10000 --seed 20261011 --csv distribution.csv --output report.json
gacha trace standalone.json BANNER ACTION --seed 42 --save-session session.json
gacha trace standalone.json BANNER ACTION --session session.json
```

`request.json` 使用 `AnalysisRequest` 的序列化格式，最少包含 `banner`、`action`、`target`、`actions`。`copies` 默认 1，分位数默认 50%/90%/95%/99%/100%。`study`、`trace`、`bundle` 支持 `--rules`。

`pull`、`transitions`、`analyze` 和 `probability` 同样使用 SDK 的准备执行器。旧 `analyze` 查询的 `draws` 保留底层抽数单位，输出追加完整动作统计。

独立数据包是已组装的数据分发格式，加载时仍重新校验并建立内存计划；它不是未经校验即可执行的二进制编译产物。

## 独立接入验证

[丹青渡接入项目](../examples/sdk/endfield-danqingdu/README.md)具有独立工作区、锁文件和输入包，业务代码只依赖 SDK。`tools/check_examples.py` 将其复制到仓库外的临时目录运行，随后以公开 CLI 核对同一输入、trace 和会话续跑，并检查运行期间引擎源文件未变化。

相关结果见[丹青渡报告](../examples/sdk/endfield-danqingdu/REPORT.md)，执行优化见[算法与性能](PERFORMANCE.md)。
