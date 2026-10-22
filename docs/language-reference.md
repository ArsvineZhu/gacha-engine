# 语言参考

本文按查阅顺序整理 GachaRule 1.0。语义以 [规范原文](../spec/gacharule/GachaRule-Language-v1.md) 为准。完整 EBNF 在规范第 2 节。58 个拒绝程序的错误码和 `文件:行:列` 在 [拒绝测试清单](../spec/gacharule/rejection-tests/README.md)。当前编译器的 `rejection_corpus_matches_codes_and_positions` 按该清单逐条对照。

写法示例见 [语言教程](language-tutorial.md)。

## 三文件职责

| 扩展名 | 顶层 | 只负责 |
|---|---|---|
| `.gmr` | 恰好一个 `rule` | 状态、符号池、条目槽、事件、概率、选择器、分布、动作、保底、更新 |
| `.gml` | 恰好一个 `roster` | 真实条目：稀有度、显示名、标签、是否可抽取 |
| `.gmb` | 一个或多个 `banner` | `source_rule`、`source_roster`、组键、符号绑定 |

三类文件的顶层语法互不兼容。一个编译单元可以包含多个规则、名册和卡池，顶层名字在单元内唯一。

编译输入是调用方给出的有序文件列表。`banner` 引用的规则和名册必须在更早的文件中已经声明。没有搜索路径。扩展名选择文法；其他扩展名报 `E_PARSE`。

稳定的运行时条目身份是 `(roster 名, entry 名)`，与路径和显示名无关。

## 词法

源文件是 UTF-8。文件开头允许一个 BOM，文件其余位置禁止 BOM。`LF` 与 `CRLF` 都是换行；孤立的 `CR` 非法。非空末行缺少换行时，词法器补一个换行。一条逻辑行必须在同一物理行内结束。

行首缩进只允许空格 `U+0020`，并且必须是 `4 × 深度`。深度只能在前一条有效行以冒号结尾时增加一级。下降可以一次多级，但必须回到先前出现过的深度。空行和只有空格或 `#` 注释的行不参与缩进。`#` 到行尾是注释；字符串内部的 `#` 是字符。制表符在字符串外报 `E_INDENT`。冒号要求块而下一有效行没有更深缩进时，报 `E_EMPTY_BLOCK`。

标识符是 `[a-z][a-z0-9_]*`，并且不是保留字。大小写敏感。字符串使用双引号，转义只有 `\\`、`\"`、`\n`、`\t`、`\u{HEX}`。`HEX` 是 1 到 6 位十六进制，码点必须有效，且不得是代理项。

数值字面量把单位写在数字后面，中间没有空格：

| 后缀 | 类型 | 形式 |
|---|---|---|
| `p` | 概率，闭区间 [0, 1] | `1p`、`1/125p` |
| `w` | 严格正权重 | `1w`、`1/2w` |
| `c` | 非负计数，必须是整数 | `0c`、`79c` |
| `t` | 动作次数，必须是整数 | `1t`、`10t` |
| `r` | 严格正稀有度，必须是整数 | `5r` |

`N` 和 `D` 只能是 `0` 或不以 0 开头的十进制整数。分数必须 `D ≥ 2`、`N > 0` 且既约。整数不得写成 `N/1`。因此 `8/1000p`、`01c`、`0/5p`、`1/1p` 都是 `E_NONCANONICAL_NUMBER`。`2p`、`0w`、`0r` 是 `E_LITERAL_RANGE`。源文本不接受百分号、小数和裸数字；`%` 只用于计数器的模条件。

布尔字面量是 `true`、`false`。作用域字面量是完整 token：`@pity`、`@progress`、`@banner`、`@action`。

类型名共 11 个，也是保留形式：`Counter`、`Bool`、`Chance`、`Pool`、`Item`、`Selector`、`Distribution`、`Action`、`Event`、`Count`、`Rarity`。

关键字共 29 个：

| 角色 | 字 |
|---|---|
| 文件根 | `rule`、`roster`、`banner` |
| 条目 | `entry` |
| 卡池指令 | `source_rule`、`source_roster`、`pity_key`、`progress_key`、`bind` |
| 保底 | `guarantee`、`item`、`rarity`、`floor`、`rescale`、`promote` |
| 更新 | `update`、`reset`、`emit` |
| 选择器与分布 | `from`、`weighted`、`choose`、`linear`、`rest` |
| 条件 | `and`、`or`、`not`、`in` |
| 结果 | `result`、`grade` |

`pool`、`when`、`state`、`else`、`then`、`return`、`pass`、`if` 不是关键字。未占用的这些拼写可以做普通标识符，也不会引出特殊语法。

标点是独立 token：`:`、`,`、`[`、`]`、`(`、`)`、`=`、`+=`、`->`、`==`、`>=`、`%`、`..`、`.`。没有单独的 `/`、`+`、`-`、`@`、`<`、`>` token。

词法错误指向非法 token 的第一个字符。列号是 1 基的 Unicode 标量值序号。

## 类型

每个声明写出一个声明类型。除了把 `Item[r]` 当作事件参数 `Item` 使用，没有隐式转换。

| 写法 | 含义 |
|---|---|
| `Counter[上界] @域 = 初值` 或 `Counter[] @域 = 初值` | 状态槽。读取时的值类型是 `Count` |
| `Bool @域 = true/false` | 布尔状态槽 |
| `Chance = 概率字面量、已声明 Chance 或 linear` | 抽前概率 |
| `Pool[r]`、`Item[r]` | 等待卡池绑定的符号。池在绑定后是非空、不重复、稀有度均为 `r` 的条目序列 |
| `Selector[r]` | 产出稀有度 `r` 的一项 |
| `Distribution` | 稀有度行，每行路由到一个同稀有度选择器 |
| `Action[下界..上界]` | 动作入口，引用一份分布 |
| `Event[参数类型, ...]` | 事件签名。参数类型只能是 `Count`、`Bool`、`Rarity`、`Item`，可以零个参数 |
| `Weight`、`Times`、`Rest` | 没有对应的可变声明。`Rest` 只能出现在分布行 |

`String` 和标签列表只用于名册显示名、标签和组键，不进入抽取。

条件的优先级是 `not` 高于 `and`，`and` 高于 `or`。可用的谓词：

| 谓词 | 位置 |
|---|---|
| `计数器 == Nc`、`计数器 >= Nc` | 抽前 |
| `计数器 % Mc == Rc`，要求 `M > 0` 且 `0 ≤ R < M` | 抽前 |
| `布尔 == true/false` | 抽前 |
| `result.grade == Nr`、`result.grade >= Nr` | 仅更新条件 |
| `result in 池名` | 仅更新条件 |
| `not`、`and`、`or`、括号、`true`、`false` | 与子表达式相同 |

保底条件和 `choose` 的条件禁止读取 `result`，否则 `E_RESULT_PHASE`。条件不提供 `<`、裸布尔名、加减乘除、用户定义运算符，或条目相等比较。

名字从上到下进入规则符号表：右侧检查完成之后，新名字才可见。使用尚未声明的名字是 `E_NAME_UNKNOWN`；同规则里这个名字出现在更下面的声明，则是 `E_NAME_BEFORE_DECL`。选择器直接引用自己是 `E_SELECTOR_CYCLE`。同一作用域禁止重复声明，报 `E_DUP_NAME`。缩进块不建立新的变量作用域。

## 声明

规则内部的声明都是 `名字: 类型…`。保底和更新写在声明之后。

```text
pity6: Counter[79c] @pity = 0c
flag: Bool @banner = false
open_count: Counter[] @progress = 0c
featured: Pool[6r]
target: Item[6r]
seen: Event[Item]
base: Chance = 1/125p
curve: Chance = linear(pity6, 65c, 1/125p, 1/20p, 1p)
pick: Selector[6r] = from featured
mix: Selector[6r] = weighted:
    1w: from featured
    1w: from standard
gated: Selector[6r] = choose(flag == true, pick, mix)
rates: Distribution:
    6r: curve -> mix
    5r: 2/25p -> five_pick
    4r: rest -> four_pick
draw: Action[1t..1000t] = rates
```

`Action` 要求 `1 ≤ 下界 ≤ 上界 ≤ 1000`。规则至少有一个非空分布和一个动作，否则 `E_RULE_SHAPE`。

有界计数器的初值不得超过上界。`+=` 的增量必须是正的计数字面量。增量大于上界时，无论当前值如何都会饱和，编译期报 `E_ALWAYS_SATURATED`。零增量报 `E_COUNTER_INIT`。

分布里每个稀有度至多一行。数量是固定 `Chance`、至少一个 `Weight` 行，或恰好一个 `rest`，三者按下面的规则组合：

| 写法 | 要求 |
|---|---|
| 只有固定概率 | 在计数器声明域内的每一个抽前状态上，固定项之和都恰好为 1 |
| 恰好一个 `rest` | 该档获得 `1 - 固定和`。固定和在所有合法抽前状态上 ≤ 1 |
| 至少两个 `Weight` 行 | 剩余质量按权重归一后分给这些行。单独一个权重行报 `E_REST_FORM` |
| `rest` 与 `Weight` 同时出现 | `E_REST_MIX` |

固定和为 1 时，剩余档的概率可以是 0；它的选择器和池仍然必须有效。

名册行：

```text
entry ef_featured: 6r, "本期限定干员", ["character", "limited"], true
```

卡池：

```text
banner endfield_event_a:
    source_rule endfield
    source_roster samples
    pity_key "endfield_limited_character"
    progress_key "endfield_phase_one"
    bind featured6: Pool[6r] = [ef_featured]
    bind featured_target: Item[6r] = ef_featured
```

`Pool` 绑定的右边是非空、不重复的条目列表。`Item` 绑定的右边是一个条目。类型注记必须与规则符号相同，否则 `E_BIND_TYPE`。条目必须存在、稀有度一致、`drawable` 为 `true`。不可抽取条目可以留在名册里。漏绑定、重复绑定、未知规则或未知名册是 `E_BIND_REF`。空池列表在绑定检查里是 `E_SELECTOR_EMPTY`。

有 `@pity` 状态就必须写 `pity_key`，没有则禁止写。`progress_key` 与 `@progress` 同样配对。

## 状态域

| 源作用域 | 身份 | 生命周期 |
|---|---|---|
| `@pity` | `(pity_key, 状态名)` | 相同 `pity_key` 的卡池共享，动作之间保留 |
| `@progress` | `(progress_key, 状态名)` | 相同进度键共享，动作之间保留 |
| `@banner` | `(卡池名, 状态名)` | 该卡池实例内保留 |
| `@action` | 同一次动作调用中的状态名 | 规范规定动作开始建立、动作结束丢弃 |

同一身份被多处声明时，计数器的上界与初值、或布尔初值必须完全一致，否则 `E_SHARED_SLOT_SCHEMA`。有上界和无上界是两种模式。新身份第一次使用时取声明初值；已有身份不因切换卡池或动作而重新初始化。

快照里的域名字是 `pity`、`progress`、`banner`、`action`。组键对 `@pity` 是 `pity_key`，对 `@progress` 是 `progress_key`，对 `@banner` 和 `@action` 是卡池名。

## 保底、分布与更新

一抽的规范顺序：

1. 冻结抽前状态。保底从上到下求值，第一条为真的生效，后面的不求值、不记录、不暴露。
2. `item 槽` 直接产出绑定条目，概率 1，跳过分布和选择器。`rarity r` 把该稀有度概率改为 1，再执行该档选择器。没有保底则使用原始分布。
3. `floor k rescale`：只保留 `r ≥ k` 的档，除以它们的质量和。质量和为 0 时，这一抽失败，错误码 `E_DRAW_EMPTY_MASS`，不提交。若所有抽前状态上该质量都是 0，编译期已经是 `E_FLOOR_ZERO`。
4. `floor k promote`：`r > k` 保持原概率，`P(k)` 变成剩余质量，`r < k` 为 0。门槛稀有度必须在每个相关分布中有一行。
5. 选择器：`from` 均匀；`weighted` 按正权重归一，同一条目的概率相加；`choose` 只看抽前条件选一个子选择器。条目概率是稀有度概率乘以选择器条件概率。
6. 全部更新条件先求值，只读抽前状态和本次结果。然后按源码顺序执行为真的块：布尔赋值覆盖，计数 `+=` 对有界槽饱和、对无界槽精确相加，`reset` 写回初值，`emit` 在该位置追加事件。同一抽内后写覆盖前写。
7. `emit` 的状态参数在执行到该条 `emit` 时读取当时的新状态。`result` 是本次条目，`result.grade` 是本次稀有度。事件按出现顺序保留，规则不解释货币或库存。

`linear` 的定义：

```text
步数 = 0                  当计数 < 起点
步数 = 计数 - 起点 + 1    当计数 ≥ 起点
概率 = min(上限, 基础 + 步长 × 步数)
```

正例里 `pity6 == 65`、起点 `65c`、基础 `1/125p`、步长 `1/20p` 时，六星原始概率是 `29/500`。`pity6 == 64` 时仍是 `1/125`。这组等式由 `tests/corpus.rs` 固定。

保底名字会作为这一抽选中的那一条返回。其余同时为真的保底没有可观察名单。

## 编译错误

多个错误并存时，先按文件顺序做词法和语法，再按 `NAME`、`TYPE`、`LOCAL`、`BIND`、`GLOBAL`。每一阶段报告文件序号、行、列最小的那一条。

| 错误码 | 含义 |
|---|---|
| `E_LEX` | 非法 token，包括百分号、小数、裸数字 |
| `E_INDENT` | 制表符或缩进宽度不对 |
| `E_PARSE` | 文法不匹配，或扩展名不对 |
| `E_EMPTY_BLOCK` | 冒号后没有缩进块 |
| `E_WEIGHT_EMPTY` | `weighted` 块没有分支 |
| `E_NONCANONICAL_NUMBER` | 分数未既约、分母不大于 1，或整数写法非法 |
| `E_LITERAL_RANGE` | 概率、权重或稀有度超出字面量范围 |
| `E_NAME_UNKNOWN` | 名字不存在 |
| `E_NAME_BEFORE_DECL` | 使用早于声明 |
| `E_DUP_NAME` | 重复声明 |
| `E_SELECTOR_CYCLE` | 选择器自指 |
| `E_TYPE` | 类型不匹配 |
| `E_RESULT_PHASE` | 在抽前位置读取 `result` |
| `E_EVENT_SIGNATURE` | 事件参数个数或类型不符 |
| `E_MODULUS` | 模数不是正数，或余数不在范围内 |
| `E_COUNTER_INIT` | 初值超过上界，或增量为 0 |
| `E_ALWAYS_SATURATED` | 增量大于上界 |
| `E_LINEAR_ARGS` | `linear` 的参数不满足曲线约束 |
| `E_DISTRIBUTION` | 稀有度重复，或固定概率之和不合法 |
| `E_REST_MIX` | `rest` 与权重混用 |
| `E_REST_FORM` | 剩余档的形式不对，包括只有一个权重行 |
| `E_ACTION_RANGE` | 动作次数不在 1 到 1000，或下界大于上界 |
| `E_RULE_SHAPE` | 规则缺少可用的分布或动作 |
| `E_GUARANTEE_EFFECT` | 强制稀有度或 `promote` 门槛在分布中没有对应行 |
| `E_FLOOR_ZERO` | `rescale` 的门槛及以上在所有抽前状态都没有质量 |
| `E_BIND_REF` | 规则、名册、符号、组键缺失，或绑定重复 |
| `E_BIND_TYPE` | 绑定类型与声明不一致，或池成员重复 |
| `E_ENTRY_UNKNOWN` | 名册中没有该条目 |
| `E_ENTRY_DISABLED` | 条目不可抽取 |
| `E_RARITY_MISMATCH` | 条目稀有度与符号不同 |
| `E_SELECTOR_EMPTY` | 绑定后的池为空 |
| `E_SHARED_SLOT_SCHEMA` | 同一共享槽的声明不一致 |

规范第 6.2 节还列出 `E_DISTRIBUTION_ROUTE`。当前编译器不产生这个错误码；选择器类型错误走 `E_TYPE`，强制稀有度缺失走 `E_GUARANTEE_EFFECT`，空池走 `E_SELECTOR_EMPTY`。

命令行的打印格式和退出码见 [开发](development.md)。

## 当前执行器

规范第 0.2 节和第 7.2 节把一次动作定义为 `(卡池, 动作, N)`：`N` 在该 `Action` 的闭区间内，动作开始建立 `@action`，连续执行 `N` 抽，结束时丢弃 `@action`。运行时失败则整段动作不提交。

当前公开函数里：

- `draw`、`apply`、`sample` 每次推进一抽。它们不重置 `@action`。`draw` 不写会话。`apply` 在所选条目仍有正概率时提交该抽；`sample` 提交抽中的分支。条目不在正概率分支中时返回 `E_ENTRY`，会话保持原样。
- `analyze` 在会话副本上执行动作边界。`times` 必须落在该动作的闭区间内，`actions` 是连续多少个这样的动作。每个动作开始时把 `@action` 恢复成初值，然后推进 `times` 抽。返回后原会话与调用前一致。多步命中质量是 `f64`。
- 没有把 `N` 抽一次提交进会话的函数。武器十连的单步测试因此用连续 `apply` 累加 `@action`；同文件的分析测试则用 `times = 10` 在副本上从初值重算。见 `tests/corpus.rs` 与 `tests/analysis.rs`。

单步概率的分子和分母是 `u128` 有理数，运算不回绕。乘加超出 `u128` 时执行器 panic，不返回 `RuntimeError`。抽样把分支权重通分后的总权重限制在 `u64` 内，超出则 panic。

宿主函数、参数形状和运行时错误码见 [SDK](sdk.md)。

## 语言不包含的能力

规范第 10 节固定排除这些能力，当前编译器也不接受它们：

- 浮点、小数、百分数、裸有理数、无单位数字。
- 四种状态域以外的作用域。
- 函数、lambda、递归、循环、宏、泛型、自定义代数类型。
- 一般的整数或有理数算术表达式。内建计算是 `linear`、模条件、常量增量和精确归一。
- `if` / `else`、动态脚本、异常捕获。条件只出现在 `choose`、`guarantee` 和 `update`。
- `import`、模块、按文件名搜索。
- 关键字别名、省略类型、字面量推断、运算符重载。
- `计数器 = 字面量`、`reset` 布尔、`+= 0c`。
- 自由文本事件，以及在规则里管理货币、库存或票券。
- 把角色、武器等种类做成稀有度枚举。种类是名册标签。
- 用 TypeScript 或 Python 解释源码。这两种语言只调用编译结果。
- 从统计反推未写进源文件的概率。
- 暴露同一抽里其余命中的保底名单。
- 运行时猜测稀有度、自动修补空池，或填上默认概率。
- 一个动作同时执行多个规则。跨规则由调用方分次调用。
