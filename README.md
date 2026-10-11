# gacha-engine

gacha-engine 是基于 Rust 的通用抽卡引擎，游戏机制、完整内容目录和具体卡池分别维护。引擎通过同一条状态转移路径执行概率枚举和随机抽样，游戏差异由 JSON 规则表达。

项目尚未发布，工作区各 crate、规则包、内容目录和卡池清单的内部版本统一为 `0.1.0`。游戏版本、来源提交号、快照日期和 `schema_version` 分别记录各自含义。

## 当前能力

当前支持计数器与布尔状态、八种生命周期作用域、常量/表格/线性概率曲线、加权与条件选择器、强制角色/稀有度/最低稀有度保底，以及声明式状态更新和奖励事件。单步概率使用约分后的 `u128` 有理数；抽样使用整数拒绝采样和可复现的 SplitMix64。

`sample_action` 原子执行完整多抽动作，失败时保持原状态和随机数进度。分析接口计算首个或第 N 个目标结果的到达分布、累计概率、分位数及有限区间期望；多步概率质量使用 `f64`，分析按完整动作区间计算，支持单抽与多抽批次。

项目提供七款游戏的 19 份参考、日期与版本预览卡池清单和 1,597 条内容记录：明日方舟：终末地、明日方舟、原神、崩坏：星穹铁道、绝区零、鸣潮、异环。参考模型的未公开概率曲线和简化机制写在各规则包的 `assumptions` 中。

## 开发环境与检查

工具链固定为 Rust 1.99.0，采用 Edition 2024 和 Cargo resolver 3；依赖版本由 `Cargo.lock` 锁定。Python 3.11 或更新版本负责数据导入和仓库门禁。

```sh
python tools/install_tools.py
python tools/check.py
```

工具配置集中在 [.config](.config/README.md)，本地与 Windows/Linux/macOS CI 使用同一个检查入口。它检查 600 行上限、文档本地链接、fmt、Clippy、deny、未使用依赖、重复代码、测试、发布模式构建和 CLI 流程。

单独格式化或审计：

```sh
cargo fmt-project
cargo fmt-check
cargo deny-project
cargo machete
jscpd --config .config/duplication.json crates tools
```

## 命令行使用

以下命令均从仓库根目录执行，每条命令写为一行，可直接用于 PowerShell。

```sh
cargo run -p gacha-cli -- validate official/gacha-games/pools/endfield/banner.winter-hunt-2026-09.json
cargo run -p gacha-cli -- inspect official/gacha-games/pools/endfield/banner.winter-hunt-2026-09.json
cargo run -p gacha-cli -- catalog official/gacha-games/pools/endfield/catalog.character-01.json
cargo run -p gacha-cli -- pull official/gacha-games/pools/endfield/banner.chartered-proportional.json endfield.banner.reference single_pull --count 10 --seed 42
cargo run -p gacha-cli -- transitions official/gacha-games/pools/endfield/banner.chartered-proportional.json endfield.banner.reference single_pull --state examples/cli/state-soft-pity.json
cargo run -p gacha-cli -- analyze official/gacha-games/pools/endfield/banner.chartered-proportional.json examples/cli/endfield-featured-120.query.json --copies 2
cargo run -p gacha-cli -- pull official/gacha-games/pools/endfield/banner.chartered-proportional.json endfield.banner.reference free_ten_pull --seed 42
cargo run -p gacha-cli -- pull official/gacha-games/pools/endfield/banner.weapon-reference.json endfield.weapon requisition --count 8 --seed 42
```

`--count` 表示完整动作的次数；`transitions` 枚举其中一次底层抽取。`--copies 2` 分析第二次命中目标的抽数，奖励事件不计入命中。票券授权、消耗、过期和库存由接入应用维护。

## 规则与内容

| 目录 | 内容 |
| --- | --- |
| `official/gacha-games/pools/<游戏>/rule.*.json` | 概率、曲线、保底、状态及动作 |
| `official/gacha-games/pools/<游戏>/catalog.*.json` | 完整角色、装备身份与来源标识 |
| `official/gacha-games/pools/<游戏>/banner.*.json` | 实际可抽名单、当期目标与引用 |
| `official/gacha-games/pools/<游戏>/sources.json` | 唯一维护的来源元数据 |

具体游戏资料与导入适配由独立[官方库](official/gacha-games/README.md)提供。通用引擎不依赖官方库；规则、目录与卡池清单在同一个游戏目录内按文件职责区分。官方案例统一位于 [examples](examples/README.md)，结果保存在各示例内。

完整目录中的条目不会自动进入任何卡池。男女管理员保留在终末地角色目录，并标为 `gacha_eligible: false`；编译器拒绝将它们用于可抽池或强制抽取保底。具体清单仍需显式列出可抽条目。

| 游戏 / 卡池家族 | 主要模型 |
| --- | --- |
| 终末地特许寻访 | 80/120 保底、软保底、阶段奖励、独立免费十连；两种低稀有度分配模型 |
| 终末地重构寻访 | 家族共享保底、同名复刻进度、30/60/90 奖励 |
| 终末地武器 | 十件动作、第 4/8 次申领保底、阶段奖励；重构武器单独定义进度归属 |
| 明日方舟中坚寻访 | 2% 基础六星、50 次未出后的增长、初次十抽保底 |
| 原神 / 星穹铁道 / 绝区零 | 角色保底与歪后必得；示例软保底曲线；原神包含实验性捕获明光状态模型 |
| 鸣潮 | 80 保底、角色 50/50、武器直接定向；基础模型未加入未公开软保底 |
| 异环 | 限定角色及同名外观进度、弧盘十抽研究模型；棋盘位置和格子效果尚未建模 |

终末地日期清单涵盖 2026 年 9 月「冬猎」「寒夜幽影」和「艺术暴君」重构武器池，记录对应公告与客户端名单。开放时间以公告为准。

## 数据更新与后续工作

```sh
python official/gacha-games/tools/sync.py
```

导入器按固定提交和哈希更新内容目录；终末地规范目录直接维护多方核对后的事实。数据差异与卡池成员需分别审查，避免新增角色进入历史卡池。

后续开发范围包括完整棋盘机制、四星定向保底、大状态空间优化，以及 WASM/Python/HTTP 接口和网页展示。当前仓库提供公开 Rust SDK 与 CLI。

接入入口为 `gacha-sdk`：加载卡池后即可调用 `study` 完成分析、模拟与对照，调用 `session` 执行抽取、trace 与续跑。使用方式见 [SDK 接入](docs/SDK.md)，优化依据及实测见[算法与性能](docs/PERFORMANCE.md)。

进一步说明：[架构](docs/ARCHITECTURE.md)、[设计](docs/DESIGN.md)、[接口](docs/API.md)、[文件格式](docs/PACK_FORMAT.md)、[规则依据](docs/RULE_SOURCES.md)、[内容数据](docs/CONTENT.md)。

丹青渡前瞻场景的枚举、随机模拟和概率图见[卡池测试报告](examples/sdk/endfield-danqingdu/REPORT.md)。
