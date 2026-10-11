# 官方游戏资料库

`gacha-games` 提供明日方舟：终末地、明日方舟、原神、崩坏：星穹铁道、绝区零、鸣潮和异环的规则与内容。资料随库内嵌分发，运行时无需联网或读取仓库文件。

通用引擎与 SDK 不依赖本库。接入应用可以使用本库，也可以向 SDK 提供自己的卡池文档。

## 目录与所有权

所有卡池定义统一维护于 `pools/<游戏>/`，通过文件类型区分职责：

| 文件 | 职责 |
| --- | --- |
| `rule.*.json` | 基础概率、保底、状态作用域、动作及更新机制 |
| `catalog.*.json` | 完整角色、装备与资源身份目录 |
| `banner.*.json` | 实际可抽成员、当期目标、规则及目录引用 |
| `sources.json` | 该游戏来源元数据的唯一维护位置 |

规则与目录通过 `source_documents` 引用来源文件；条目的 `source_ids` 保留具体事实依据。完整目录不会自动决定卡池成员，管理员形态等不可抽条目也可存在于目录中。

`tools/games/` 是各游戏内容导入适配器，`tools/sources.json` 只配置导入任务及来源标识。终末地采用多方人工核对，直接维护规范目录与卡池清单；核对摘要保存在 `tools/reviews/`。

## Rust 接入

```rust
let engine = gacha_games::load("endfield/banner.preview-si.json")?;
let actions = engine.actions();
```

`banners()` 枚举资料标识，`document()` 查询原始文档，`pack()` 返回通用聚合规则，`load_with_profile()` 可切换机制配置。抽取、分析、trace、会话和导出由返回的 SDK 引擎提供。

参考池与版本预览的假设保留在规则及描述中，资料标识不表示当前正在开放。完整例子见[丹青渡接入示例](../../examples/sdk/endfield-danqingdu/README.md)，核对范围见[内容数据](../../docs/CONTENT.md)。
