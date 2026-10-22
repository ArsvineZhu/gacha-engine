# 语言教程

这篇教程给写 GachaRule 的人。语义细节和错误码在 [语言参考](language-reference.md)。规范原文是 [GachaRule 1.0](../spec/gacharule/GachaRule-Language-v1.md)。

一次编译是一组按顺序给出的源文件。扩展名决定顶层文法：`.gmr` 是机制，`.gml` 是名册，`.gmb` 是卡池绑定。没有 `import`。卡池引用的 `rule` 和 `roster` 必须出现在更早的文件里。

命令行只做检查。下面每组三文件都已用当前 `gacha check` 编译通过。仓库里带精确概率测试的完整模型在 `spec/gacharule/positive/`，见最后一节。

缩进是四空格。数值必须带单位：概率 `p`，权重 `w`，计数 `c`，动作次数 `t`，稀有度 `r`。分数要写成既约形式，例如 `1/10p`，不要写成 `2/20p`。

## 最小卡池

机制只声明符号和怎么抽。名册才放具体条目。绑定把符号接到条目上。

`docs/examples/mini/mini.gmr`：

```text
rule mini:
    faces: Pool[1r]
    pick: Selector[1r] = from faces
    rates: Distribution:
        1r: 1p -> pick
    flip: Action[1t..1t] = rates
```

`Pool[1r]` 是稀有度 1 的符号池，这里还没有成员。`from faces` 在池成员上均匀抽取。分布只有一档，概率 `1p`，总和必须是 1。`Action[1t..1t]` 声明这个动作的合法次数是 1。动作次数的全局范围是 1 到 1000。

`docs/examples/mini/mini.gml`：

```text
roster coins:
    entry heads: 1r, "正面", ["coin"], true
    entry tails: 1r, "反面", ["coin"], true
```

`entry` 的四个字段依次是稀有度、显示名、标签列表、是否可抽取。显示名和标签不参与概率。最后的 `true` 表示可以进入池或条目槽。

`docs/examples/mini/mini.gmb`：

```text
banner fair:
    source_rule mini
    source_roster coins
    bind faces: Pool[1r] = [heads, tails]
```

`bind` 左边的类型必须与规则里的声明逐字相同。池成员的稀有度必须是 `1r`，并且可抽取。

```sh
cargo run -- check docs/examples/mini/mini.gmr docs/examples/mini/mini.gml docs/examples/mini/mini.gmb
```

成功时没有输出。用库对 `fair` / `flip` 做一抽时，`heads` 与 `tails` 各为 `1/2`，保底名为空。调用方式见 [SDK](sdk.md)。

规则里每一个 `Pool` 和 `Item` 都要在卡池里绑定一次。这一条规则没有 `@pity` 或 `@progress` 状态，所以绑定里不要写 `pity_key` 或 `progress_key`。

## 硬保底

硬保底是一条 `guarantee`：抽前条件为真时，它替换这一抽的分布。源码从上到下，只采用第一条为真的保底。

`docs/examples/hard/hard.gmr`：

```text
rule hard:
    since: Counter[9c] @pity = 0c
    rare: Pool[5r]
    common: Pool[3r]
    pick5: Selector[5r] = from rare
    pick3: Selector[3r] = from common
    rates: Distribution:
        5r: 1/10p -> pick5
        3r: rest -> pick3
    draw: Action[1t..1t] = rates
    guarantee tenth: since >= 9c -> rarity 5r
    update true:
        since += 1c
    update result.grade >= 5r:
        reset since
```

`Counter[9c] @pity = 0c` 是有上界的计数器，初值 0，上界 9，存在 `@pity` 域。`rest` 拿走固定概率剩下的全部质量，所以三星是 `9/10`。

`guarantee tenth` 在抽前 `since >= 9c` 时把稀有度改成五星概率 1，再走五星选择器。`rarity 5r` 要求分布里有 `5r` 这一档。

更新在这一抽的结果已经确定之后执行。全部 `update` 条件先按抽前状态和本次结果求值，然后按源码顺序执行条件为真的块。`since += 1c` 每次加 1，到达上界后饱和。`reset since` 写回声明初值 `0c`。计数器只允许 `+=` 正整数和 `reset`；布尔状态只允许赋 `true` 或 `false`。

`docs/examples/hard/hard.gml`：

```text
roster roster_hard:
    entry star: 5r, "五星", ["character"], true
    entry stone: 3r, "三星", ["item"], true
```

`docs/examples/hard/hard.gmb`：

```text
banner hard_banner:
    source_rule hard
    source_roster roster_hard
    pity_key "hard_pity"
    bind rare: Pool[5r] = [star]
    bind common: Pool[3r] = [stone]
```

规则含有 `@pity` 状态，绑定必须给出 `pity_key`。相同 `pity_key` 的卡池共享这组计数。

```sh
cargo run -- check docs/examples/hard/hard.gmr docs/examples/hard/hard.gml docs/examples/hard/hard.gmb
```

`since` 为 0 时，`star` 为 `1/10`，`stone` 为 `9/10`。把 `since` 设为 9 再抽，命中的保底名是 `tenth`，`star` 的概率是 1。

## 软保底

软保底是分布上的 `linear`，在硬保底生效之前改变稀有度概率。形式只有这一种：

```text
linear(计数器, 起点, 基础概率, 步长, 上限)
```

抽前计数小于起点时，概率等于基础概率。计数大于或等于起点时，步数是 `计数 - 起点 + 1`，概率是 `min(上限, 基础 + 步长 × 步数)`。起点那一次已经加上第一步。四个数值参数必须直接写字面量，并满足 `0 ≤ 基础 < 上限 ≤ 1`、`0 < 步长 ≤ 1`。

`docs/examples/soft/soft.gmr`：

```text
rule soft:
    pity: Counter[9c] @pity = 0c
    featured: Pool[5r]
    standard: Pool[5r]
    filler: Pool[3r]
    top: Selector[5r] = weighted:
        1w: from featured
        1w: from standard
    low: Selector[3r] = from filler
    rates: Distribution:
        5r: linear(pity, 5c, 1/10p, 1/10p, 1p) -> top
        3r: rest -> low
    draw: Action[1t..1t] = rates
    guarantee hard: pity >= 9c -> rarity 5r
    update true:
        pity += 1c
    update result.grade >= 5r:
        reset pity
```

`weighted` 的权重必须严格为正，并在选择器内部归一。这里两个 `1w` 把五星对半拆开。`choose(条件, 是, 否)` 是另一条只看抽前状态的选择器，正例 `genshin.gmr` 用它写五星和四星的定向。

`docs/examples/soft/soft.gml`：

```text
roster roster_soft:
    entry featured_char: 5r, "限定", ["character", "limited"], true
    entry standard_char: 5r, "常驻", ["character"], true
    entry filler_item: 3r, "填充", ["item"], true
```

`docs/examples/soft/soft.gmb`：

```text
banner soft_banner:
    source_rule soft
    source_roster roster_soft
    pity_key "soft_pity"
    bind featured: Pool[5r] = [featured_char]
    bind standard: Pool[5r] = [standard_char]
    bind filler: Pool[3r] = [filler_item]
```

```sh
cargo run -- check docs/examples/soft/soft.gmr docs/examples/soft/soft.gml docs/examples/soft/soft.gmb
```

对 `soft_banner` / `draw` 实测：

| 抽前 `pity` | 五星总概率 | 条目 |
|---|---|---|
| 0 或 4 | `1/10` | `featured_char` 与 `standard_char` 各 `1/20`，`filler_item` 为 `9/10` |
| 5 | `1/5` | 两个五星条目各 `1/10`，填充为 `4/5` |
| 9 | 1 | 保底名 `hard`，结果只有 `featured_char` 与 `standard_char`，各 `1/2` |

`pity = 5` 时步数已经是 1，所以是 `1/10 + 1/10 = 1/5`。`pity = 9` 时硬保底先于曲线生效，曲线不再参与这一抽。

名册里的标签可以交给命中分析，例如正例用标签 `limited` 作为目标。分析接口见 [SDK](sdk.md)。

## 仓库里的三套正例

`spec/gacharule/positive/` 是规范第 8 节的全部源文件，并且有测试锁住有理数结果。五个文件必须按这个顺序一起编译：

```sh
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/genshin.gmr spec/gacharule/positive/weapon_ten.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`cargo test --test corpus` 的 `positive_examples_match_exact_probabilities` 检查下表。卡池名和动作名来自 `banners.gmb` 与各规则的 `Action`。

| 调用 | 结果 |
|---|---|
| `endfield_event_a` / `draw`，`pity6 = 64` | 六星 `1/125` |
| 同上，`pity6 = 65` | 六星 `29/500` |
| 同上，`pity6 = 79` 且 `pulls_this_banner = 0` | 保底 `six_80`，六星概率 1 |
| 同上，`pulls_this_banner = 119` 且 `obtained_featured = false` | 保底 `limited_120`，条目 `ef_featured` 概率 1 |
| `genshin_event_a` / `draw`，初值 | 五星 `3/500`，四星 `51/1000`，三星 `943/1000` |
| 同上，`pity5 = 73` | 五星 `33/500` |
| 同上，`pity5 = 0` 且 `pity4 = 9` | 保底 `four_hard`；五星仍为 `3/500`，四星 `497/500`，三星 0 |
| 同上，`pity5 = 89` 且 `pity4 = 9` | 保底 `five_hard`，五星概率 1 |
| `weapon_sample_a` / `ten`，先对 `wp_four` 提交 9 次 | 下一抽六星 `1/10`、五星 `9/10` |

终末地规则把软保底、80 抽六星和 120 抽限定写成两条保底。120 抽那条写在前面，所以它优先于六星硬保底。进度奖励用 `emit` 发出带 `Count` 参数的事件；规则本身不管理货币。

原神规则用 `choose(missed_featured5 == true, guaranteed5, regular5)` 在「下次必出限定池」和「限定与常驻对半」之间切换。四星硬保底写的是 `floor 4r promote`：比 4 更高的稀有度保持原概率，门槛档吃掉其余质量，更低的档变成 0。因此五星硬保底和四星硬保底同时满足时，只有写在前面的五星保底会留下名字。

武器十连的动作声明是 `Action[10t..10t]`。保底使用 `floor 5r rescale`：只保留门槛及以上的档，再按它们原来的比例归一。基础分布是六星 `1/100`、五星 `9/100`，所以第十抽在门槛内重抽时六星 `1/10`、五星 `9/10`。计数器 `misses_in_action` 放在 `@action`。单步 `apply` 会把这个计数留在会话里，所以上面的测试用连续 9 次提交走到保底。命中分析在每个动作开始时另有重置规则，见 [语言参考](language-reference.md) 的执行器一节。

`sample.gml` 里的 `ef_archived` 以 `false` 留在名册中。它可以存在，但不能放进任何池或条目槽。

换期是再写一个 `banner`：换条目，并决定是否沿用同一个 `pity_key`。沿用则共享 `@pity` 状态；新的卡池名拥有自己的 `@banner` 状态。
