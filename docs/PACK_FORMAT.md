# 规则、内容与卡池格式

机器契约以 `gacha-schema` 导出的类型为准。维护格式分为三个文档；`GamePack` 是组装后的内存聚合和低层测试格式。所有项目内部 `version` 统一为 `0.1.0`，`schema_version` 当前为 1。

## 卡池清单

卡池清单从清单所在目录解析规则和内容的相对路径，显式列出具体池的可抽成员：

```json
{
  "schema_version": 1,
  "id": "example.banner",
  "version": "0.1.0",
  "game": "example",
  "rules": "rule.character.json",
  "catalogs": ["catalog.character-01.json"],
  "pools": [{"id": "featured", "items": ["example.character.1001"]}],
  "item_bindings": {"featured": "example.character.1001"},
  "banner_bindings": {
    "example.template": {
      "id": "example.event.1",
      "pity_group": "example.family",
      "progress_group": "example.event.1"
    }
  }
}
```

`item_bindings` 绑定规则中的符号槽位：`force_item` 和相同槽位的奖励事件键会解析为真实条目标识，同时仅在组装结果中追加槽位标签。`banner_bindings` 将规则模板的卡池标识、保底组和进度组绑定为具体清单需要的值；未提供时沿用模板。

## 规则包与内容目录

`RulePack` 包含来源、假设、状态、概率分布、选择器和卡池动作定义，拒绝嵌入 `items` / `pools`。选择器引用符号池，清单负责实际成员。

`ContentCatalog` 包含游戏、内部版本、来源和 `ItemDef`，拒绝机制字段。条目保存 `id`、`kind`、`display_name`、本地化 `names`、规范 `rarity`、`native_rarity`、`upstream_id`、`source_ids`、标签和可选 `gacha_eligible`。

规则包与内容目录可用 `"source_documents": ["sources.json"]` 引用共享的来源定义数组。路径相对于引用文档；`sources` 仍可提供内联定义，同一标识的内容冲突会报错。官方库在每个游戏目录唯一维护来源文件，条目的 `source_ids` 指向其中具体来源。`load_pack`、`load_catalog`、`load_pack_with_rules` 和内嵌资源解析器均解析这些引用；纯组装函数 `compose_pack` 接收已经解析的来源定义。

`gacha_eligible: false` 表示明确不能通过抽取获得，仍保留在完整目录。缺省表示目录层未指定限制；它不构成入池授权。编译器拒绝不可抽条目进入池或成为强制抽取结果。来源包含 URL、核对时间、提交号及适用的 SHA-256，详见[内容数据](CONTENT.md)。

组装拒绝跨游戏目录、重复身份、缺少绑定及冲突来源；编译继续检查引用、池成员、稀有度与完整规则图。目录条目可以不属于任何池。

低层聚合形状：

```json
{
  "schema_version": 1,
  "id": "example.game",
  "version": "0.1.0",
  "sources": [], "assumptions": [], "state": [], "items": [],
  "pools": [], "distributions": [], "selectors": [], "banners": []
}
```

## 状态

状态类型支持 `counter` 与 `boolean`，作用域为 `account`、`game`、`pity_group`、`progress_group`、`banner`、`session`、`batch`、`pull`。

```json
{"id": "six_star_pity", "kind": "counter", "scope": "pity_group", "initial": 0, "max": 79}
```

## 概率表达式

常量和线性增长示例：

```json
{"rarity": 5, "type": "constant", "value": "0.08"}
```

```json
{
  "rarity": 6, "type": "linear_after", "state": "six_star_pity",
  "base": "0.008", "after": 65, "increment": "0.05", "cap": "1"
}
```

`after: 65` 表示状态 0..64 使用基础概率，65 次未命中后的下一抽首次增加概率。

```json
{"rarity": 6, "type": "table", "state": "pity", "values": {"70": "0.2", "71": "0.25"}, "default": "0.01"}
```

`remainder` 取得 `1 - 已指定概率之和`。多个 `share_of_remainder` 按整数权重分配剩余质量：

```json
{"rarity": 5, "type": "share_of_remainder", "weight": 80}
```

## 选择器

```json
{"id": "selector.five", "type": "pool", "pool": "pool.five"}
```

加权分支的权重是相对值，不要求总和为 1：

```json
{
  "id": "selector.six", "type": "weighted",
  "branches": [
    {"weight": "1", "selector": "selector.current"},
    {"weight": "1", "selector": "selector.off_banner"}
  ]
}
```

条件分支只能读取抽前状态，不能使用结果条件。编译器同时检查两条路径的引用、循环、深度和输出稀有度：

```json
{
  "id": "selector.top", "type": "conditional",
  "when": {"op": "state_bool_eq", "state": "featured_due", "value": true},
  "then_selector": "selector.featured", "else_selector": "selector.coin_flip"
}
```

## 动作、批次与条件

动作 `draws` 缺省为 1，范围为 1..=1000。十抽设为 10，在 `batch` 作用域维护抽数和已出现稀有度；最后一抽的保底可判断抽前计数为 9。

`sample_action` 每次清空批次/单抽状态，持久状态继承，完整动作原子提交。`sample_step` / `enumerate_step` 仅操作一次底层抽取，由调用方管理批次边界。分析区间按底层抽数给出，必须包含完整动作；在动作边界重置批次状态，并保留逐抽到达分布。

条件 `op` 支持：`always`、`all`、`any`、`not`、`state_counter_gte`、`state_counter_eq`、`state_bool_eq`、`state_modulo_eq`、`outcome_rarity_eq`、`outcome_rarity_gte`、`outcome_item_in_pool`。结果条件只能用于抽后转移。

## 保底与更新

```json
{
  "id": "featured_120", "priority": 300,
  "when": {"op": "state_counter_gte", "state": "banner_pulls", "value": 119},
  "effect": {"type": "force_item", "item": "featured"}
}
```

`force_rarity` 强制稀有度；`min_rarity` 提供两种策略：`conditional_current` 按满足最低稀有度的当前质量重新归一化，`preserve_higher_fill_floor` 保留更高稀有度质量并把低稀有度移到最低等级。

状态更新支持 `increment`、`reset`、`set_counter` 和 `set_bool` 状态更新。抽后条件基于相同抽前状态及最终结果，匹配更新按声明顺序执行。

## 事件

```json
{"kind": "grant_currency", "key": "arsenal_quota", "amount": 2000}
```

事件是接入层使用的输出；核心不解释资源含义，也不直接改变库存。库调用说明见[接口](API.md)。
