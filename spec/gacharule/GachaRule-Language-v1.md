# GachaRule 1.0：通用抽卡规则源语言规范

**状态：规范性版本 1.0，供 Rust 编译器与统一执行器直接实现。**

本文中的 **必须 / 禁止 / 错误** 均为规范要求；代码块是源语言，不是 JSON，也不是宿主语言。语言名 `GachaRule` 仅供标识。本文的概率参数样例属于可编译的「游戏风格模型」，不宣称覆盖对应游戏的全部现行、未公开或动态调整的机制。

## 0. 固定决策

| 设计点 | 唯一决定 | 理由 |
|---|---|---|
| 源文件 | `.gmr` 机制、`.gml` 名册、`.gmb` 卡池绑定；扩展名决定顶层文法 | 职责隔离；不再增加文件头关键字 |
| 分块 | 行结束 + 严格四空格缩进 + 冒号引块；无花括号备选 | 保持 Python 式阅读；词法一次生成 `INDENT/DEDENT` |
| 数值 | 带 `p/w/c/t/r` 后缀的**规范化**有理数或整数；不支持百分号与小数 | 字面量即类型；无任何浮点换算与同义数值写法 |
| 保底冲突 | 保底在源码中**从上往下优先级递减**；只应用第一条；不暴露其余命中者 | 不需要显式整数优先级，不存在同优先级冲突 |
| 计数越界 | 有上界的计数器按包含上界饱和；无上界采用任意精度非负整数；明显恒饱和的增量编译拒绝 | 动作不会因正常长期累计意外溢出；仍阻止无意义的常量更新 |
| 遮蔽 | 所有可见的词法作用域均禁止遮蔽；同作用域重复声明报错 | 所有引用静态且唯一 |
| 复用 | 只通过命名 `Selector`、`Distribution` 和多个 `Action` 直接引用；保底与更新是规则级共用定义 | 没有函数、模板、宏或隐式复制 |
| 软保底 | 唯一内建 `linear(counter, start, base, step, cap)`；起点已经加一步 | 一种写法承载唯一精确曲线 |
| 静态类型 | 没有隐式类型转换、隐式声明、运算符重载或上下文推断字面量 | Rust 编译器能在解析后直接检查 |
| 非法条件重抽质量 | 编译时拒绝必然为零者；若某个实际抽前状态使质量为零，则本次动作原子失败 | 表达动态条件时保留精确语义，而非悄悄替换策略 |

### 0.1 文件与编译单元

一次编译接收**显式给定、按顺序排列**的源文件列表。每个 `.gmr` 文件恰好一个 `rule`，每个 `.gml` 文件恰好一个 `roster`，一个 `.gmb` 文件有一个或多个 `banner`。`banner` 所引用的 `rule` 与 `roster` 必须出现在本次编译输入中**更早的文件**。没有 `import`、搜索路径或隐式扫描。一个编译单元可有多个规则、名册与卡池；其顶层名称在编译单元内全局唯一。

`rule` 只声明机制、符号池与符号条目槽；`roster` 只声明真实条目；`banner` 只指定引用、状态组键与成员绑定。三类文件的顶层语法互不兼容。

### 0.2 最小执行接口

编译结果将命名的 `Action` 暴露为 `(banner_id, action_id, N)`。其中 `N` 必须处于该 `Action` 声明的闭区间内，并且全局必须处于 `[1,1000]`。一个 `Action` 不自带可运行脚本：它引用一个稀有度分布，并共享所在规则全部保底与更新。动作开始清空 `@action` 状态，连续执行 `N` 抽；动作结束丢弃 `@action` 状态。外部可以在不改变源码的情况下以不同 `N` 调用宽区间动作。

## 1. 词法（穷尽定义）

### 1.1 编码、行与缩进

- 源文件是严格 UTF-8，允许文件最前面一个 UTF-8 BOM（忽略），禁止文件中其他 BOM。字符串、注释允许 Unicode；标识符仅 ASCII 小写。稳定的运行时条目 ID 定义为 `(roster_id, entry_id)`，与文件路径及显示名无关。
- `LF` 和 `CRLF` 均视为同一个物理换行；孤立 `CR` 非法。输出逻辑 `NEWLINE`。非空末行无换行时，词法器补一个 `NEWLINE`。
- 一条有效逻辑行必须在**同一物理行**内完结；不支持反斜杠续行、括号内隐式续行、多行字符串。所有列表和函数形内建构造均须在同一行完成，唯有冒号引入的缩进块可跨行。
- 行首缩进仅允许 ASCII 空格 `U+0020`；制表符 `U+0009` 在任何字符串外均非法。有效行的缩进必须是 `4 × 深度` 个空格。相邻有效行的深度只能增加一级，且**只能在前一条有效行的最后一个非注释 token 为冒号时增加**。深度下降可一次下降多级，但必须回到先前出现的深度。若前行需要块而下一有效行没有更深缩进，则报 `E_EMPTY_BLOCK`。
- 空行与只有空格和/或 `#` 注释的行对缩进栈完全无影响，不产生 `NEWLINE/INDENT/DEDENT`。普通行内允许词间一个或多个空格；行尾允许空格和注释。
- `#` 起直到物理换行前是 `COMMENT`，词法阶段丢弃。`#` 在字符串内部是字符，不是注释。
- `EOF` 前产生必要的 `DEDENT`，栈最后必须回到零。缩进符号的位置取触发缩进变化的有效行的首个非空白字符。

### 1.2 词法 token 一览

| 类别 | token | 源形式 / 规则 |
|---|---|---|
| 布局 | `NEWLINE`, `INDENT`, `DEDENT`, `EOF` | 由词法布局器产生 |
| 已识别但丢弃 | `SPACE`, `COMMENT`, `BLANK_LINE` | 上述约定 |
| 名称 | `IDENT` | `[a-z][a-z0-9_]*`，且不是保留字 |
| 文本 | `STRING` | 双引号，支持 `\\`、`\"`、`\n`、`\t`、`\u{HEX}`；不支持原始换行及其他转义；Unicode 码点必须有效，`HEX` 为 1—6 位十六进制数字且不得编码代理项/超过 `U+10FFFF` |
| 概率 | `P_LIT` | `N p` 或 `N/D p`（实际无空格：`1p`, `1/125p`） |
| 正权重 | `W_LIT` | `Nw`, `N/Dw` |
| 非负计数 | `C_LIT` | `Nc`，**不可为分数** |
| 动作次数 | `T_LIT` | `Nt`，**不可为分数** |
| 稀有度 | `R_LIT` | `Nr`，**不可为分数** |
| 布尔 | `BOOL_LIT` | `true`, `false` |
| 状态域 | `SCOPE_LIT` | `@pity`, `@progress`, `@banner`, `@action`，各为完整独立 token |
| 类型名 | `COUNTER_T`, `BOOL_T`, `CHANCE_T`, `POOL_T`, `ITEM_T`, `SELECTOR_T`, `DISTRIBUTION_T`, `ACTION_T`, `EVENT_T`, `COUNT_T`, `RARITY_T` | 分别是 `Counter`, `Bool`, `Chance`, `Pool`, `Item`, `Selector`, `Distribution`, `Action`, `Event`, `Count`, `Rarity` |
| 标点 | `: , [ ] ( ) = += -> == >= % .. .` | 每种都是独立 token；没有单独 `/`, `+`, `-`, `@`, `<`, `>` token |
| 关键字 | 见第 3 节 | 关键字均为完整单词精确匹配 |

`N` 和 `D` 均为十进制非负整数字符序列，**只能**为 `0` 或不以零开头的正整数。`N/D` 必须满足 `D>=2`、`N>0`、`gcd(N,D)=1`；整数不得写作 `N/1`。因此 `1/125p` 有唯一规范写法，`8/1000p`、`2/4w`、`01c`、`0/5p`、`1/1p` 均是 `E_NONCANONICAL_NUMBER`。源码不允许 `0.8%`、裸 `8/1000`、`0.008`；前者表示的精确概率在本语言中唯一写为 `1/125p`。不得先按 `f32`/`f64` 读取，再转换为有理数。

`P_LIT` 的数学值为 `N` 或 `N/D`，须满足 `0<=p<=1`；`W_LIT` 必须严格大于零；`C_LIT` 须非负；`T_LIT` 的有效范围由使用位置检查；`R_LIT` 必须为严格正整数，数字没有枚举上限。`0w`、`2p` 与 `0r` 均分别触发字面量范围错误。所有整数任意精度；不存在机器整数溢出后回绕。

**有序词法选择（仅此三处）：**

1. 读到 `"` 时，优先完整扫描 `STRING`（内部的 `#` 没有注释意义）；其他位置的 `#` 开始 `COMMENT`。
2. 运算符先识别 `+=`、`->`、`==`、`>=`、`..`，再识别单字符标点；不允许将这些拆开后另作解析。
3. 以数字开头时优先扫描**最长**的完整「整数或有理数 + 单位后缀」；不能退化为裸数字、除法及独立字母。以字母开头先扫描完整单词，再依次按 `BOOL_LIT`、类型名、关键字、`IDENT` 的互斥词表分类。保留字后接 `_x` 的整体是 `IDENT`。

语法层不依赖有序选择；上述优先级是词法唯一需要的优先级。词法报错指向不合法 token 的第一个字符。

## 2. 完整文法（EBNF，显式 token 层）

记号：`{X}` 表示零个或多个，`[X]` 表示至多一个，`X | Y` 是 FIRST 集互不相交的选择；`NL` 代表 `NEWLINE`。`IDENT`、数值、缩进都是第 1 节已生成的独立 token。文件扩展名先选唯一入口，因此顶层也没有有序选择。

```ebnf
MechanismFile  = "rule", IDENT, ":", NL, INDENT, Decl, {Decl}, {Guarantee}, {Update}, DEDENT, EOF ;
RosterFile     = "roster", IDENT, ":", NL, INDENT, Entry, {Entry}, DEDENT, EOF ;
BannerFile     = Banner, {Banner}, EOF ;

Banner         = "banner", IDENT, ":", NL, INDENT,
                 "source_rule", IDENT, NL,
                 "source_roster", IDENT, NL,
                 ["pity_key", STRING, NL],
                 ["progress_key", STRING, NL],
                 {Binding}, DEDENT ;
Binding        = "bind", IDENT, ":", (PoolType | ItemType), "=", (PoolMembers | IDENT), NL ;
PoolMembers    = "[", [IDENT, {",", IDENT}], "]" ;
Entry          = "entry", IDENT, ":", R_LIT, ",", STRING, ",", TagList, ",", BOOL_LIT, NL ;
TagList        = "[", [STRING, {",", STRING}], "]" ;

Decl           = IDENT, ":", DeclTail ;
DeclTail       = CounterType, SCOPE_LIT, "=", C_LIT, NL
               | "Bool", SCOPE_LIT, "=", BOOL_LIT, NL
               | PoolType, NL
               | ItemType, NL
               | "Event", "[", [ParamType, {",", ParamType}], "]", NL
               | "Chance", "=", ChanceExpr, NL
               | "Selector", "[", R_LIT, "]", "=", SelectorRhs
               | "Distribution", ":", NL, INDENT, DistributionRow, {DistributionRow}, DEDENT
               | "Action", "[", T_LIT, "..", T_LIT, "]", "=", IDENT, NL ;
CounterType    = "Counter", "[", [C_LIT], "]" ;
PoolType       = "Pool", "[", R_LIT, "]" ;
ItemType       = "Item", "[", R_LIT, "]" ;
ParamType      = "Count" | "Bool" | "Rarity" | "Item" ;
ChanceExpr     = P_LIT | IDENT | Linear ;
Linear         = "linear", "(", IDENT, ",", C_LIT, ",", P_LIT, ",", P_LIT, ",", P_LIT, ")" ;
SelectorRhs    = SelectorInline, NL | "weighted", ":", NL, INDENT, WeightRow, {WeightRow}, DEDENT ;
SelectorInline = "from", IDENT | "choose", "(", Condition, ",", SelectorInline,
                 ",", SelectorInline, ")" | IDENT ;
WeightRow      = W_LIT, ":", SelectorInline, NL ;
DistributionRow  = R_LIT, ":", DistributionAmount, "->", IDENT, NL ;
DistributionAmount = ChanceExpr | W_LIT | "rest" ;

Guarantee      = "guarantee", IDENT, ":", Condition, "->", Effect, NL ;
Effect         = "item", IDENT | "rarity", R_LIT
               | "floor", R_LIT, ("rescale" | "promote") ;
Update         = "update", Condition, ":", NL, INDENT, Mutation, {Mutation}, DEDENT ;
Mutation       = IDENT, ("=", BOOL_LIT | "+=", C_LIT), NL
               | "reset", IDENT, NL
               | "emit", IDENT, "(", [Arg, {",", Arg}], ")", NL ;
Arg            = C_LIT | R_LIT | BOOL_LIT | IDENT | "result", [".", "grade"] ;

Condition      = OrExpr ;
OrExpr         = AndExpr, {"or", AndExpr} ;
AndExpr        = NotExpr, {"and", NotExpr} ;
NotExpr        = "not", NotExpr | "(", Condition, ")" | Predicate ;
Predicate      = BOOL_LIT
               | IDENT, IdentifierTest
               | "result", ResultTest ;
IdentifierTest = "==", (C_LIT | BOOL_LIT)
               | ">=", C_LIT
               | "%", C_LIT, "==", C_LIT ;
ResultTest     = ".", "grade", ("==" | ">="), R_LIT
               | "in", IDENT ;
```

**关于文法中的** `|`**：** `Decl` 已因子分解为 `IDENT : DeclTail`，紧接着按**唯一类型名**决定分支；`SelectorRhs` 的 `weighted` 和内联选择器的 FIRST 集分别为 `WEIGHTED` 与 `{FROM, CHOOSE, IDENT}`；`ChanceExpr` 的 FIRST 集为 `{P_LIT, IDENT, LINEAR}`；`Condition` 在 `IDENT` 后先吃名字再根据 `== / >= / %` 的单个 token 判别；`ResultTest` 的 FIRST 集 `{., IN}` 互斥；`Binding` 的类型与右值的首 token 各自独立决定分支，再由静态类型检查匹配。`ParamType` 全为独立保留类型名。上述写法仅为 EBNF 便利可用公共前缀因子分解实现为 LL(1) 递归下降。PEG 实现按文本次序书写各互斥分支也得到相同语法树；**没有任何语法有序选择（ordered choice）或依赖回溯的消歧**。

静态语义附加条件：`Binding` 中 `PoolType` 必须配 `PoolMembers`，`ItemType` 必须配单 `IDENT`；错误的组合为 `E_BIND_TYPE`。`SelectorRhs` 的 `weighted:` 后需要至少一个 `WeightRow`；空块在布局或语法阶段直接报 `E_WEIGHT_EMPTY`，无需为非法空块单独添加一种合法语法。`Event[]` 可零参数；池成员列表 `[]` 可解析但必报 `E_SELECTOR_EMPTY`，不会产生有效绑定。

### 2.1 一个标记代表一个固定的语法角色

`result` 是只读内建当前结果（抽前条件禁止读取），`result.grade` 是唯一稀有度访问形式；`from` 只构造池选择器；`weighted` 只构造权重选择器；`choose` 只构造条件选择器；`linear` 只构造内建概率曲线。语法中所有圆括号形式仅属于 `choose`、`linear` 和 `emit`，不构成一般函数调用。

## 3. 完整关键字与保留字表

**词法关键字共 29 个，逐字穷尽：**

| 唯一角色 | 关键字 |
|---|---|
| 文件根声明 | `rule`, `roster`, `banner` |
| 名册条目声明 | `entry` |
| 卡池内来源与组键指令 | `source_rule`, `source_roster`, `pity_key`, `progress_key` |
| 卡池符号绑定 | `bind` |
| 保底定义 | `guarantee` |
| 保底效果判别 | `item`, `rarity`, `floor`, `rescale`, `promote` |
| 更新条件块 | `update` |
| 更新原语 | `reset`, `emit` |
| 选择器构造 | `from`, `weighted`, `choose` |
| 概率构造、剩余档 | `linear`, `rest` |
| 条件运算 | `and`, `or`, `not`, `in` |
| 结果只读主语与字段名 | `result`, `grade` |

此外有 11 个专门的**类型名称**（第 1 节已列出）、2 个**布尔字面量** `true/false` 和 4 个完整**作用域字面量** `@pity/@progress/@banner/@action`；这些也是保留形式，禁止作 `IDENT`。全文不存在其他隐含关键字。`pool`、`when`、`state`、`else`、`then`、`return`、`pass`、`if` **不是**关键字，不允许以这些词引出特殊语法；其中未占用的字符串仍可作普通 `IDENT`，但完全不会触发特殊解析。

## 4. 类型系统与判断规则

### 4.1 类型与判定记号

`Γ` 为本规则当前已声明符号表，`B` 为完成绑定后的卡池环境，`P∈{pre,update,event}` 是表达式所处阶段，`τ` 为**值类型**。

- `String` 与 `TagList`：仅供编译期名册显示名、标签和组键使用的文本与文本列表；其字面量为 `STRING` / `TagList`，不提供同名源类型声明，也不会进入抽取 IR。
- `Count`：无符号任意精度整数。`Counter[U] @scope = i`（或 `Counter[]`）的**声明类型**是有状态 `Counter(scope,i,U)`，读取该名字的值类型是 `Count`。
- `Bool`：二值类型。`Bool @scope = false/true` 为状态声明，读取值类型 `Bool`。
- `Chance`：区间 `[0,1]` 内的精确有理数；表达式既可以是常量，也可以是仅依赖抽前计数器的 `linear`。
- `Weight`：严格正有理数，仅作加权分支/剩余分配的权重，没有 `Weight` 可变声明。
- `Times`：非负整数且附动作次数单位；只用于 `Action` 的上下界。
- `Rarity`：严格正的整数且附稀有度单位，排序按整数大小。
- `Item`：真实条目引用；`Item[r]` 为带固定稀有度的符号槽，绑定后变成一个 `Item`。
- `Pool[r]`：绑定后为非空、互不重复且稀有度皆为 `r` 的条目序列。
- `Selector[r]`：返回一项稀有度为 `r` 的概率分布；绑定前为符号有向图。
- `Distribution`：有稀有度行与选择器路由的分布声明。
- `Action[a..b]`：动作入口，引用一份 `Distribution`。
- `Event[τ1,...,τn]`：已声明事件类型，其实参只允许 `Count / Bool / Rarity / Item`。
- 语句/保底类型 `Unit` / `Guarantee`，文件根分别为 `Rule`、`Roster`、`Banner`，名册行 `Entry`、绑定行为 `Binding`。`rest` 只有内部类型 `Rest`，且**只能**用在分布行，不允许变量声明。

每个声明都显式写出**一个**声明类型，类型构造器为固定语法，不是泛型。`Item[r]` 在选择器等位置可以隐式忘掉稀有度精化成为内建 `Item` 的事件参数；这是同一个条目引用的静态精化消除，**不是**整数/概率之类的类型转换。没有任何其他隐式转换。

### 4.2 表达式判断（规范性）

```text
Γ ⊢ "text" : String      Γ ⊢ [String,...] : TagList
Γ ⊢ Nc : Count          Γ ⊢ Nt : Times       Γ ⊢ Nr : Rarity
Γ ⊢ Qp : Chance         Γ ⊢ Qw : Weight      Γ ⊢ true/false : Bool
Γ(x)=Counter(...)      ───────────────────  Γ ⊢ x : Count
Γ(x)=Bool@scope        ───────────────────  Γ ⊢ x : Bool
Γ(x)=Chance            ───────────────────  Γ ⊢ x : Chance
Γ(x)=Item[r]           ───────────────────  Γ ⊢ x : Item[r]
Γ(x)=Selector[r]       ───────────────────  Γ ⊢ x : Selector[r]
Γ(x)=Pool[r]           ───────────────────  Γ ⊢ x : Pool[r]
Γ(x)=Distribution      ───────────────────  Γ ⊢ x : Distribution

Γ(x)=Counter(...), a:Count, b/s/m:Chance_literals
0<=b<m<=1, 0<s<=1
──────────────────────────────────────────── Γ ⊢ linear(x,a,b,s,m) : Chance

Γ ⊢ p : Pool[r]        ──────────────────── Γ ⊢ from p : Selector[r]
Γ ⊢ u,v : Selector[r], Γ ⊢ C : Bool@pre
──────────────────────────────────────────── Γ ⊢ choose(C,u,v) : Selector[r]
每个 weighted 分支 wi:Weight, si:Selector[r], 分支数>=1
──────────────────────────────────────────── Γ ⊢ weighted-block : Selector[r]
```

`Bool@pre` 表示条件只用抽前状态。`Bool@update` 另可读取当前结果；`Bool@event` 不构成独立条件语言。具体谓词规则：

| 构造 | 要求 | 结果类型 |
|---|---|---|
| `counter == Nc`, `counter >= Nc` | 左值必须为 Counter 状态 | `Bool` |
| `counter % Mc == Rc` | 左值 Counter；`M>0` 且 `0<=R<M` | `Bool` |
| `flag == true/false` | 左值必须为 Bool 状态 | `Bool` |
| `result.grade == Nr`, `result.grade >= Nr` | 仅 `P=update` | `Bool` |
| `result in poolname` | 右操作数为 `Pool[r]`；仅 `P=update` | `Bool` |
| `not C`, `C and D`, `C or D` | 子项均 `Bool`，`not` > `and` > `or` | `Bool` |
| `(C)`、`true`、`false` | 布尔 | `Bool` |
| `result` / `result.grade`（事件实参） | 仅更新体内 `emit` | `Item` / `Rarity` |

条件**不**支持 `counter < k`、布尔裸名 `flag`、任意加减乘除、稀有度小于比较或结果条目相等比较；`>=` 和 `==` 是内建、静态封闭的判定规则，不允许用户重载。

### 4.3 声明与语句判断

- `p: Pool[r]` 类型 `Pool[r]`；`x: Item[r]` 类型 `Item[r]`；均是**必须在卡池中恰好绑定一次**的符号，不是创建内容。
- `s: Selector[r] = E` 须有 `Γ ⊢ E: Selector[r]`，包括加权块；权重按精确分数归一化。
- `c: Chance = E` 须有 `Γ ⊢ E: Chance`，引用仅限先前 Chance 声明。
- `d: Distribution:` 的每行 `r: amount -> s` 要求 `r:Rarity`、`s:Selector[r]`，`amount` 为 `Chance / Weight / Rest`；一个稀有度只出现一次。分布范围检查见 §6。
- `a: Action[lo..hi] = d` 要求 `1<=lo<=hi<=1000`，`d:Distribution`。
- `e: Event[τ...]` 仅声明事件签名；`emit e(args)` 要求实参个数相同、逐个严格匹配 `τ`；该语句类型 `Unit`。
- `guarantee name: C -> item slot`：`C:Bool@pre`、`slot:Item[r]`；`-> rarity r`：有分布对应选择器；`-> floor r rescale/promote`：`r:Rarity`，`promote` 要求存在门槛档。声明类型 `Guarantee`。
- `update C:` 的 `C:Bool@update`，块内的每条 Mutation 类型 `Unit`，整个块类型 `Unit`，没有块内独立作用域或变量声明。
- `flag = bool_literal`：目标**必须**为 Bool 状态；`counter += positive_count_literal`：目标**必须**为 Counter 状态；`reset counter`：目标**必须**为 Counter 状态。没有 `counter = 0c`、`flag += 1c`、`reset flag`。三种状态操作由目标类型互斥区分，恢复初值只写 `reset`。
- `entry`：得到 `Entry`，四元字段强制分别是 `Rarity, String, TagList, Bool`。`bind`：得到 `Binding`，标注的 `Pool[r]` / `Item[r]` 必须与被绑定符号**声明类型相同**；所有条目真实存在、稀有度匹配且 `drawable=true`。

**类型错误示例（错误在引号内的 token）：** `b: Bool @action = false` 后在条件写 `b >= 1c`：期望 Counter，实际 Bool；`c: Counter[9c] ...` 后 `c = false`：期望 Bool 状态，实际 Counter；`Pool[5r]` 用于 `Selector[6r]` 的 `from`：期望 Pool[6r]，实际 Pool[5r]；分布写 `6r: spotlight -> s`：期望 Chance/Weight/Rest，实际 Item[6r]。

## 5. 名称与生命周期

### 5.1 词法作用域

- 编译单元有一个全局顶层符号表，收录已完成的 `rule/roster/banner`；跨文件引用只能引用此前已声明且类型匹配的根。
- `rule` 的所有 `Decl` 名称和 `guarantee` 名称共享唯一的规则内部符号表。源码从上到下注册；声明**初始化式/右侧表达式完成类型检查后**才将新名字加入可见表。唯一例外：直接在自己的 Selector 初始化式引用自身，诊断为 `E_SELECTOR_CYCLE`，而不是普通未声明；它仍然非法。
- `roster` 的 `entry` ID 在所属 roster 内唯一；不向 rule 作用域泄露。`banner` 的 `bind` 左边**引用**规则已声明的符号，不创建新符号；右侧条目 ID 在 `source_roster` 内解析。
- 任何作用域内禁止重复声明；子层（若将来有子层）也禁止遮蔽其可见父名。本版的 Selector/Distribution/Update 缩进块不创建新变量作用域。
- 标识符与所有保留字、类型名称、布尔字面量不可同名；大小写敏感。未声明名统一 `E_NAME_UNKNOWN`；在同一规则中该名字出现在更晚的声明行时报告 `E_NAME_BEFORE_DECL`。自引用 Selector 使用上述专门错误。
- 数据 ID 的含义仅由声明与绑定决定；无字符串到 ID、整数到 Rarity 等隐式转换。

### 5.2 状态槽和四个存储域

| 源作用域 | 存储身份 | 生命周期 |
|---|---|---|
| `@pity` | `(pity_key, state_identifier)` | 多个使用相同 `pity_key` 的卡池共享；动作之间保留 |
| `@progress` | `(progress_key, state_identifier)` | 相同进度周期共享；动作之间保留 |
| `@banner` | `(banner_id, state_identifier)` | 当前卡池实例内跨动作保留 |
| `@action` | `(action_invocation, state_identifier)` | 每次动作开始新建、动作结束销毁；同动作各抽共享 |

同一状态身份被多个机制引用时，声明的 `Counter` 上界/初值或 `Bool` 初值必须完全一致，否则报 `E_SHARED_SLOT_SCHEMA`。缺失的上界与有上界不是同一种模式。新身份第一次使用时创建初值；已有身份不能因切换卡池、调用另一 Action 而重新初始化。更换 `progress_key` 是开启新进度周期；更换 `banner_id` 是切换独立卡池状态。无 `@pity` 状态的规则禁止 `pity_key`，有则强制提供；`progress_key` 同理。这样不会创建未使用的组配置。

## 6. 编译期语义检查（规范性）

下面的顺序同时定义**多个错误并存时优先诊断哪一类**：先按编译输入顺序扫描各文件，先运行 `LEX/PARSE`，随后按 `NAME`、`TYPE`、`LOCAL`、`BIND`、`GLOBAL` 阶段；每阶段报告 `(文件序号,行,列)` 最小的错误。`E_`* 代码是固定可测试类别；在 `NAME/TYPE` 阶段即使 Rule 尚未声明 Action，也先报已经发现的名称/类型错误。

### 6.1 本地静态检查

1. `E_LEX`, `E_INDENT`, `E_PARSE`, `E_EMPTY_BLOCK`, `E_WEIGHT_EMPTY`, `E_NONCANONICAL_NUMBER`, `E_LITERAL_RANGE`：布局、保留字、不完整字面量、非法单位、空权重块、非法字面量。
2. `E_NAME_UNKNOWN`, `E_NAME_BEFORE_DECL`, `E_DUP_NAME`, `E_SELECTOR_CYCLE`：名称在使用前存在、无重复、无自引用；其他潜在选择器成环必须在构图阶段再次 DFS 检查为 `E_SELECTOR_CYCLE`。
3. `E_TYPE`, `E_RESULT_PHASE`, `E_EVENT_SIGNATURE`, `E_MODULUS`：包括保底及选择器里不得读结果，模数严格正且余数合法。
4. `E_COUNTER_INIT`：有界计数器初值不得超过上界；`+= 0c` 错误。若**无论输入状态如何都必然饱和**，即 `delta > upper`，编译时拒绝 `E_ALWAYS_SATURATED`；这一检查不依赖条件路径：只要 `delta>upper` 就必然在所有合法当前新状态上饱和。合法的其他增加执行时饱和到上界。
5. `E_LINEAR_ARGS`：曲线的参数精确满足 `0<=base<cap<=1`、`0<step<=1`、`start>=0`；首参必须计数器。`linear` 的后四参只能直接写带单位的字面量，不接任意表达式。
6. `E_DISTRIBUTION`：每档稀有度不重复；固定项是 `Chance`（可由已声明 `Chance` 引用或曲线给出）。固定和须在所有**声明域内的抽前状态**上 `<=1`。`Chance`/曲线非负且单调非降，故总和最大值通过所有计数器取上界（无界取曲线封顶值（本版规定步长严格正））精确决定。无剩余档时总和在所有状态必须恰为 1；检查其最小值与最大值同时等于 1。
7. `E_REST_MIX`, `E_REST_FORM`：剩余档模式只能是**恰好一个** `rest`，或者**至少两个** `Weight` 行；两模式互斥，带权模式不允许只有一档。没有剩余档时不得有 `rest/Weight`。有剩余档时若固定和为 1，剩余质量 0 是允许的，该档最终为零概率，但其绑定的池依然必须非空。
8. `E_ACTION_RANGE`, `E_RULE_SHAPE`：动作次数及引用类型合法，规则至少有一个分布、一个 Action，所有 Action 对应的 Distribution 非空，至少含一档。
9. `E_GUARANTEE_EFFECT`：强制稀有度必须在该规则**每个 Action** 引用的 Distribution 中存在对应选择器；`floor ... promote` 的门槛稀有度也必须在每个分布中；`floor ... rescale` 至少应当有一档大于等于门槛。若可用的上方概率质量在所有抽前状态恒为零，静态报 `E_FLOOR_ZERO`。
10. `E_EVENT_SIGNATURE`：已声明事件的参数数量/类型错误；未声明的事件按名称阶段报 `E_NAME_UNKNOWN`。事件不得在更新体外触发。所有 `emit` 保持顺序，无去重。

### 6.2 跨文件链接检查

1. `E_BIND_REF`：`source_rule`、`source_roster` 必须是先前的有效根，且类属正确；引用未知符号池或条目槽、绑定遗漏、重复绑定均非法。
2. `E_BIND_TYPE`：`bind` 左侧显式声明类型必须与规则符号类型逐字等价；池成员列表必须非空、成员唯一；槽必须绑定唯一条目。
3. `E_ENTRY_UNKNOWN`, `E_ENTRY_DISABLED`, `E_RARITY_MISMATCH`：每个被引用条目必须存在，`drawable=true`，稀有度等于符号类型。不可抽取条目可以留在名册，但不得进入任何池、绑定到任何 slot，也不得被保证路径强制产出。
4. `E_SELECTOR_EMPTY`, `E_SELECTOR_CYCLE`：完成绑定后 DFS 确保所有选择器的叶子池非空且引用图无环；由于禁止前向引用，本版合法语法本身即保证不会出现多节点环，该检查仍是构图后必须执行的 IR 不变量。
5. `E_DISTRIBUTION_ROUTE`：每个有概率的稀有度档都明确指定类型正确的选择器；不能依赖运行时猜稀有度；命中的强制稀有度同理。无概率的档依然必须有选择器与非空池，使同一个语法节点语义固定。
6. `E_SHARED_SLOT_SCHEMA`：共享同名、同组的状态槽定义必须完全兼容。

### 6.3 报错协议

每条错误包含固定 `code`、路径、**1 基行号和 Unicode 标量值计数的 1 基列号**、token 半开位置跨度、相关声明的第二位置（若有）、`expected`、`actual`。如果类别与类型无关，`expected/actual` 填入期望声明类别与实际声明类别，如 `expected=Pool[6r], actual=Pool[5r]`、`expected=先前已声明 Selector, actual=稍后才声明的 Selector`。`E_EMPTY_BLOCK` 指向冒号；重复声明指向第二个名称；非法字面量指向整个字面量；引用错误指向引用 ID；调用阶段禁止 `result` 时指向 `result` token；缺少绑定指向 `banner` 头部，附缺失名称。

## 7. 精确概率与到执行表示的降低

### 7.1 目标 IR（非源语法、非 JSON 格式）

编译后执行器只需以下**静态数据结构的语义**（布局由 Rust 实现者决定）：

- `StateSlot(scope, group_or_banner_key, name, kind, initial, upper?)`，kind 是 `Count/Bool`；编译绑定阶段给出稳定整数槽位 ID。
- `Item(id, rarity, drawable=true, tags/display仅在编译器或外部目录保留)`；运行时用条目 ID 与稀有度，显示文字、标签可被剥离。
- `PoolNode(rarity, item_ids[])`；`SelectorNode = Uniform(pool) | Weighted[(positive_rational, child)] | Conditional(pre_predicate, yes, no)`。拓扑有向无环，共享命名节点只产生引用，不展开成宏。
- `ChanceNode = Rational | Linear(slot,start,base,step,cap)`，常量 `Chance` 引用在编译时内联为对应节点 ID（可做公共子表达式复用）。
- `DistributionRow(rarity, amount_kind{fixed/rest/weight}, selector_id)`；带权余数权重与选择器权重分别有独立归一化层。
- `Guarantee(pre_predicate, ForcedItem(item_id) | ForcedRarity(r) | Floor(r,Rescale/Promote))`；**数组顺序即优先级**，不可附带重叠命中者的可观察列表。
- `Update(predicate(pre_state,result), ordered_operations[])`，操作为 `SetBool(slot,v) | SaturatingIncrement(slot,delta) | ResetCount(slot,initial) | Emit(event_id,typed_args)`。
- `Action(id,min_times,max_times,distribution_id)`；`Rule` 级保证与更新数组由所有 Action 共享。`Banner` 将各槽位、池节点、item 绑定成可执行实例。

`roster` 的 display/tags、规则中注释与引用来源都不参与概率计算。只有 `drawable=true` 的项可能进入可执行节点；布尔 `drawable` 在绑定后已通过检查，不必每抽重复分支。权重和概率以数学整数对（约分有理数）运算，概率乘法、加法、归一化与比较全程精确；采样从精确离散分布中无偏选择，不采用浮点累积阈值。

### 7.2 单次动作、小抽的严格步骤

1. 调用 `(banner,action,N)`，确认 N 合法，为本动作建立 `@action` 状态；其他状态按组/卡池已存值读取。整个动作按顺序处理每一抽，若运行时语义错误则该动作**整体不提交任何新状态或事件**。
2. 第 i 抽冻结全部抽前状态 `S_i`；从上往下计算各 `Guarantee` 的 pre 条件，遇到第一真值立即选中，后面的保底**不评估、不记录、不暴露**。
3. 对被选中的 `ForcedItem`，直接返回绑定的真实条目，概率质量 1，完全跳过 Distribution/Selector；对 `ForcedRarity(r)`，将稀有度概率改为仅 r 为 1，然后执行 r 档 Selector；对 Floor 执行第 7.3 节；没有保底则采用原始分布。
4. 对每个正概率稀有度，运行其 `Selector`：`Uniform` 每个池成员 `1/size`；`Weighted` 按各分支权重 / 总权重 混合；`Conditional` 仅看 `S_i` 在两子选择器中选一。若一个 item 被不同路径选中，条件概率相加。最终条目概率是 `P(rarity) × P(item | rarity)`，全体条目精确求和为 1。
5. 按源码顺序先计算**全部 Update 条件**，每个条件只可读取 `S_i` 和本次 `result`。创建待写新状态 `S'_i := S_i`，依次执行条件为真的 Update 块及其操作：`SetBool` 直接覆盖；`SaturatingIncrement` 读取**当前** `S'_i`，有界取 `min(U, S'_i[c]+delta)`、无界精确相加；`ResetCount` 写回声明初值；`Emit` 在操作所在位置追加事件。若同一抽多次更新同一槽，后写依次覆盖前写，**无并行合并**。
6. `emit` 的事件实参在**执行该 Emit 的时刻**求值：引用状态时读取当前 `S'_i`；`result` 读取本次结果；常量原样使用。事件按出现顺序进入该动作的事件列表，不管理货币、库存或任何资产。
7. 提交 `S_{i+1}:=S'_i` 并继续下一抽；动作结束提交跨动作的组/卡池状态和事件结果，销毁 `@action` 状态。分析器对每个可能分支执行同样的确定性状态转移，合并等价状态时相加精确概率。

### 7.3 三个保底效果与两种最低稀有度策略

设原始稀有度分布为 `P(r)`，指定门槛为 `k`。

- `item slot`：绑定 `Item[r]` 后直接产出该条目，稀有度读取条目本身。原始稀有度概率、Selectors 完全跳过。
- `rarity r`：`P'(r)=1`，其余为 0，执行 `r` 的选择器。
- `floor k rescale`（**条件重抽**）：`Z = Σ_{r>=k} P(r)`；对 `r>=k` 有 `P'(r)=P(r)/Z`；对 `r<k` 为 0。`Z=0` 是运行时 `E_DRAW_EMPTY_MASS`，动作不提交。
- `floor k promote`（**保留更高并填满门槛**）：对 `r>k` 有 `P'(r)=P(r)`；`P'(k)=1-Σ_{r>k}P(r)`；对 `r<k` 为 0。要求 k 在分布中有一行；其原概率被完全替换。由于原始分布总和为 1，所有结果非负。

两个 Floor 策略只选其中明确写出的一个；没有默认策略。

### 7.4 概率曲线与分布

`linear(c,a,b,s,m)` 按**抽前计数**定义：

```text
steps(c,a) = 0                  if c < a
           = c - a + 1          if c >= a
linear     = min(m, b + s*steps)
```

因此 `c == a` **已经加第一步**。`c` 从 `Counter` 的状态槽读取；原值精确，不能从本次已有更新获取。任何 `Chance` 引用同样是一段纯抽前计算，不能读取 `result`。

设固定档集合 F，`Q = Σ_{r∈F} fixed(r)`，编译期保证任何合法声明域状态下 `0<=Q<=1`，`R=1-Q`：

- 没有剩余档：必须 `Q ≡ 1`；
- 一档 `rest`：该档获得全部 R；
- 至少两档权重 `w_r>0`：每档得到 `R*w_r/Σw`；
- 禁止一个 `rest` 与权重混用，禁止单独一个权重档。

`Q=1` 时有剩余档产生零概率，此时 selector 仍须类型正确、池非空。概率值没有舍入公差、epsilon、浮点修正或“最后一档补误差”。

### 7.5 声明到 IR 的逐项对照

| 源构造 | 唯一降低结果 |
|---|---|
| `Counter[...] @scope = init` | 一个具声明初值/上界的计数槽 |
| `Bool @scope = value` | 一个布尔状态槽 |
| `Pool[r]` / `Item[r]` | 待卡池绑定的稀有度精化符号；绑定后取确定 PoolNode / Item ID |
| `Chance = literal/linear/ref` | Rational / Linear / 指向先前 ChanceNode 的引用 |
| `Selector[r] = from pool` | `Uniform(pool)` |
| `Selector[r] = weighted:` | `Weighted`，按正有理数权重归一；同条目概率相加 |
| `Selector[r] = choose(C,a,b)` | `Conditional(C@pre,a,b)` |
| `Selector[r] = older` | 直接引用同一 SelectorNode |
| `Distribution` 行 | 固定、单剩余或加权剩余的带 Selector 路由表 |
| `Action[lo..hi] = distribution` | Action 入口；所有规则级 Guarantee/Update 对它生效 |
| `guarantee` | 追加到规则有序 Guarantee 数组；名字仅供编译诊断 |
| `update C:` | 追加一个有序 Update；C 先于更新执行统一求值 |
| `flag = literal` | `SetBool` |
| `counter += delta` | `SaturatingIncrement` 或无界精确增加 |
| `reset counter` | `ResetCount(initial)` |
| `emit e(args)` | `Emit`，事件签名已经静态检查 |
| `entry` | 编译期目录 Item 记录，display/tags 可剥离 |
| `source_*`, `*_key`, `bind` | 建立 Banner 绑定、共享槽身份、池的成员序列与 singleton slot，不生成脚本 |

## 8. 三套正例与完整绑定

**编译输入次序：** `endfield.gmr`、`genshin.gmr`、`weapon_ten.gmr`、`sample.gml`、`banners.gmb`。所有代码块均是对应文件的**全部内容**，可一次送入编译器生成三个卡池实例，三者共用**同一套语法、IR 与执行器**。名册中的条目仅作可观察测试数据；真实项目可由 Python 生成 `.gml`，而不能从机制文件取得角色名单。

### 8.1 `endfield.gmr`：终末地式软保底、80 次六星与 120 次限定

```text
rule endfield:
    pity6: Counter[79c] @pity = 0c
    pulls_this_banner: Counter[119c] @banner = 0c
    obtained_featured: Bool @banner = false
    progress_pulls: Counter[] @progress = 0c
    featured6: Pool[6r]
    standard6: Pool[6r]
    featured_target: Item[6r]
    five_items: Pool[5r]
    four_items: Pool[4r]
    reward_current: Event[Count]
    reward_next: Event[Count]
    reward_token: Event[Count]
    six_pick: Selector[6r] = weighted:
        1w: from featured6
        1w: from standard6
    five_pick: Selector[5r] = from five_items
    four_pick: Selector[4r] = from four_items
    rates: Distribution:
        6r: linear(pity6, 65c, 1/125p, 1/20p, 1p) -> six_pick
        5r: 2/25p -> five_pick
        4r: rest -> four_pick
    draw: Action[1t..1000t] = rates
    guarantee limited_120: pulls_this_banner >= 119c and obtained_featured == false -> item featured_target
    guarantee six_80: pity6 >= 79c -> rarity 6r
    update true:
        pity6 += 1c
        pulls_this_banner += 1c
        progress_pulls += 1c
    update result.grade >= 6r:
        reset pity6
    update result in featured6:
        obtained_featured = true
    update progress_pulls == 29c:
        emit reward_current(10c)
    update progress_pulls == 59c:
        emit reward_next(10c)
    update progress_pulls % 240c == 239c:
        emit reward_token(1c)
```

源中没有任何具体条目名字。`1/125p=0.8%`，`2/25p=8%`，`1/20p=5%`。当 `pity6 == 65c` 时，六星概率恰为 `1/125 + 1/20 = 29/500`（5.8%）；更早为 0.8%。当 `pity6==79c`，六星硬保底直接选中，曲线不再参与该抽分布。若第 120 抽且还未获得本期限定，`limited_120` 比六星硬保底优先，强制条目。进度奖励的判定用抽前 `progress_pulls`，第 30、60、240、480……抽分别发出对应事件；事件的货币语义留给外部。

**建模取舍：** 本正例把「120 抽前未出限定，则第 120 抽强制指定」作为演示业务规则。真实游戏若还有更复杂的重置、继承、相遇倍率等，必须调整**源数据/规则**，不能修改执行器的判断顺序。

### 8.2 `genshin.gmr`：原神式角色池、五星 UP 继承与四星保底

```text
rule genshin_character:
    pity5: Counter[89c] @pity = 0c
    pity4: Counter[9c] @pity = 0c
    missed_featured5: Bool @pity = false
    missed_featured4: Bool @pity = false
    featured5: Pool[5r]
    standard5: Pool[5r]
    featured4: Pool[4r]
    standard4: Pool[4r]
    standard3: Pool[3r]
    regular5: Selector[5r] = weighted:
        1w: from featured5
        1w: from standard5
    guaranteed5: Selector[5r] = from featured5
    select5: Selector[5r] = choose(missed_featured5 == true, guaranteed5, regular5)
    regular4: Selector[4r] = weighted:
        1w: from featured4
        1w: from standard4
    guaranteed4: Selector[4r] = from featured4
    select4: Selector[4r] = choose(missed_featured4 == true, guaranteed4, regular4)
    select3: Selector[3r] = from standard3
    rates: Distribution:
        5r: linear(pity5, 73c, 3/500p, 3/50p, 1p) -> select5
        4r: 51w -> select4
        3r: 943w -> select3
    draw: Action[1t..1000t] = rates
    guarantee five_hard: pity5 >= 89c -> rarity 5r
    guarantee four_hard: pity4 >= 9c -> floor 4r promote
    update true:
        pity5 += 1c
        pity4 += 1c
    update result.grade >= 5r:
        reset pity5
    update result.grade >= 4r:
        reset pity4
    update result in standard5:
        missed_featured5 = true
    update result in featured5:
        missed_featured5 = false
    update result in standard4:
        missed_featured4 = true
    update result in featured4:
        missed_featured4 = false
```

这里 `3/500p=0.6%`；软保底从抽前 `pity5==73c` 对应的第 74 抽开始，起点已加 `3/50p=6%`；第 90 抽由 `five_hard` 保证五星。四星保底是「四星或以上」，故使用 `floor 4r promote`，保持 5 星原概率不变并将余量全交给四星。若第十抽同时命中五星保底，则仅五星保底生效。

普通状态下稀有度 5 星占 `3/500`，剩下 `497/500` 按 `51:943` 分到 4 星和 3 星；由此 4 星在基础阶段恰好是 `51/1000=5.1%`。软保底阶段，四星和三星按比例共享减小后的剩余量。五星 / 四星精选机制分别由**持久 Bool 状态 + 条件选择器**实现，卡池换期后同 `pity_key` 即共享计数与是否下次 UP。此例**有意简化**当前游戏的其他子机制（如「捕获明光」、部分实际公示概率的细分），用来验证类型、四星保底、选择器与状态组合的表达能力。

### 8.3 `weapon_ten.gmr`：武器十连、按动作内次数保底、条件重抽

```text
rule weapon_ten:
    misses_in_action: Counter[9c] @action = 0c
    six_weapons: Pool[6r]
    five_weapons: Pool[5r]
    four_weapons: Pool[4r]
    rare_drop: Event[Item]
    select6: Selector[6r] = from six_weapons
    select5: Selector[5r] = from five_weapons
    select4: Selector[4r] = from four_weapons
    rates: Distribution:
        6r: 1/100p -> select6
        5r: 9/100p -> select5
        4r: rest -> select4
    ten: Action[10t..10t] = rates
    guarantee at_least_five: misses_in_action >= 9c -> floor 5r rescale
    update true:
        misses_in_action += 1c
    update result.grade >= 5r:
        reset misses_in_action
        emit rare_drop(result)
```

动作 10 抽从 `misses_in_action=0` 开始，若前九抽均未获得 5 星以上，第十抽被强制达到至少 5 星；若前面已获得 5 星以上，则对应时刻计数重置。此处使用 `rescale`，所以触发时 6 星和 5 星的条件概率严格为 `1/10` 与 `9/10`，这与前例四星保底的 `promote` 有意不同。动作结束该计数销毁；下一次十连重新计数。本规则不管理武器类别，只使用名册中的普通标签。

### 8.4 `sample.gml`：一份独立、可复用的名册

```text
roster samples:
    entry ef_featured: 6r, "本期限定干员", ["character", "limited"], true
    entry ef_standard: 6r, "常驻六星干员", ["character"], true
    entry ef_five: 5r, "五星干员", ["character"], true
    entry ef_four: 4r, "四星干员", ["character"], true
    entry gi_featured5: 5r, "本期五星角色", ["character", "limited"], true
    entry gi_standard5: 5r, "常驻五星角色", ["character"], true
    entry gi_featured4a: 4r, "本期四星角色甲", ["character", "limited"], true
    entry gi_featured4b: 4r, "本期四星角色乙", ["character", "limited"], true
    entry gi_standard4: 4r, "常驻四星条目", ["character"], true
    entry gi_standard3: 3r, "普通三星条目", ["weapon"], true
    entry wp_six: 6r, "六星武器", ["weapon"], true
    entry wp_five: 5r, "五星武器", ["weapon"], true
    entry wp_four: 4r, "四星武器", ["weapon"], true
    entry ef_archived: 6r, "已归档不可抽取", ["archive"], false
```

`ef_archived` 合法存在于名册且不会进入任何实例；条目「种类」只是一组文本标签。条目顺序不是抽取概率的权重。

### 8.5 `banners.gmb`：一份纯绑定文件，包含三个实例

```text
banner endfield_event_a:
    source_rule endfield
    source_roster samples
    pity_key "endfield_limited_character"
    progress_key "endfield_phase_one"
    bind featured6: Pool[6r] = [ef_featured]
    bind standard6: Pool[6r] = [ef_standard]
    bind featured_target: Item[6r] = ef_featured
    bind five_items: Pool[5r] = [ef_five]
    bind four_items: Pool[4r] = [ef_four]

banner genshin_event_a:
    source_rule genshin_character
    source_roster samples
    pity_key "genshin_character_event"
    bind featured5: Pool[5r] = [gi_featured5]
    bind standard5: Pool[5r] = [gi_standard5]
    bind featured4: Pool[4r] = [gi_featured4a, gi_featured4b]
    bind standard4: Pool[4r] = [gi_standard4]
    bind standard3: Pool[3r] = [gi_standard3]

banner weapon_sample_a:
    source_rule weapon_ten
    source_roster samples
    bind six_weapons: Pool[6r] = [wp_six]
    bind five_weapons: Pool[5r] = [wp_five]
    bind four_weapons: Pool[4r] = [wp_four]
```

换期操作是生成新 `banner`，指定新条目及正确的组键，不复制或重写抽卡算法。例如第二个使用 `genshin_character` 的新 `banner` 采用**同一个** `pity_key` 即继承五星、四星以及精选保证状态；新卡池 ID 独立维护 `@banner`。Python 可以创建与验证名册源文本，但 Rust 编译器是唯一语言解释者。

### 8.6 正例必须满足的精确性质

| 输入状态 / 操作 | 必须得到的性质 |
|---|---|
| 终末地 `pity6=64` | 六星原始概率 `1/125` |
| 终末地 `pity6=65` | 六星原始概率 `29/500` |
| 终末地 `pity6=79` | 本抽六星概率 `1`；选中 `six_80`，除非先命中更高优先级的 `limited_120` |
| 终末地 `pulls_this_banner=119` 且未出限定 | 指定 `ef_featured` 的概率 `1`，并跳过原始 Selector |
| 终末地 `progress_pulls=239` | 本抽追加一次 `reward_token(1c)` 事件 |
| 原神 `pity5=0` | 5 星 `3/500`，4 星 `51/1000`，3 星 `943/1000` |
| 原神 `pity5=73` 且无优先保底 | 5 星概率 `33/500`（6.6%） |
| 原神 `pity4=9`，未命中更高保底 | 5 星维持原始概率，4 星取得其余全部概率，3 星为零 |
| 原神 `pity5=89`, `pity4=9` | 五星保底独占；四星保底不可观察 |
| 武器十连前 9 抽均为四星 | 第 10 抽在 5+ 内重抽：六星概率 `1/10`、五星 `9/10` |
| 武器十连第 3 抽出五星，第 4—10 抽为四星 | 整个十连已满足至少一次 5+，第 10 抽无保底 |

## 9. 编译必须拒绝的最小程序与错误坐标

随规范交付 `rejection-tests/`：其中**每个编号子目录均含完整、独立的源程序**，后缀恰为本语言的三种源文件后缀。单文件反例只需一个 `.gmr`，跨文件反例包含规则、名册、绑定文件；`57_shared_group_schema` 含两个机制文件。下表精确给出每例的**应拒绝类别**及应指向的 `文件:行:列`（以 offending token 起点计算）。整套语料不包含任何执行器或编译器实现。

| 编号 / 程序目录                           | 必须拒绝为                   | 错误位置         | 最小反例中的关键源码        | 判定原因                      |
| ----------------------------------- | ----------------------- | ------------ | ----------------- | ------------------------- |
| `01_indent_tab/`                    | `E_INDENT`              | `r.gmr:2:1`  | `\t`              | 缩进使用制表符                   |
| `02_indent_width/`                  | `E_INDENT`              | `r.gmr:2:1`  | `p:`              | 每级缩进不是四空格                 |
| `03_percent_literal/`               | `E_LEX`                 | `r.gmr:2:17` | `0.8%`            | 百分号和小数不是源数值形式             |
| `04_noncanonical_rational/`         | `E_NONCANONICAL_NUMBER` | `r.gmr:2:17` | `8/1000p`         | 概率分数未约成既约分数               |
| `05_bad_rational_denominator/`      | `E_NONCANONICAL_NUMBER` | `r.gmr:2:17` | `1/0p`            | 有理数的分母必须大于一               |
| `06_probability_over_one/`          | `E_LITERAL_RANGE`       | `r.gmr:2:17` | `2p`              | Chance 字面量超出一             |
| `07_reserved_name/`                 | `E_PARSE`               | `r.gmr:1:6`  | `rule`            | 关键字不能成为根声明名字              |
| `08_name_unknown/`                  | `E_NAME_UNKNOWN`        | `r.gmr:3:28` | `absent`          | 引用不存在的符号                  |
| `09_name_before_decl/`              | `E_NAME_BEFORE_DECL`    | `r.gmr:2:28` | `p`               | 首次使用早于声明                  |
| `10_name_duplicate/`                | `E_DUP_NAME`            | `r.gmr:3:5`  | `p: Pool[3r]`     | 同一作用域的第二次声明               |
| `11_selector_self_cycle/`           | `E_SELECTOR_CYCLE`      | `r.gmr:2:23` | `s`               | 选择器自指形成一节点环               |
| `12_selector_forward_cycle/`        | `E_NAME_BEFORE_DECL`    | `r.gmr:2:24` | `s2`              | 试图用前向引用形成双节点环             |
| `13_bool_as_counter/`               | `E_TYPE`                | `r.gmr:3:18` | `b >= 1c`         | 期望 Counter 状态，实际 Bool     |
| `14_counter_as_bool_target/`        | `E_TYPE`                | `r.gmr:4:9`  | `c = false`       | 期望 Bool 状态赋值，实际 Counter   |
| `15_item_as_chance/`                | `E_TYPE`                | `r.gmr:6:13` | `i -> s`          | 概率位置意外收到 Item             |
| `16_selector_rarity_type/`          | `E_TYPE`                | `r.gmr:3:28` | `p`               | 期望 Pool[5r] 实际 Pool[4r]   |
| `17_result_in_guarantee/`           | `E_RESULT_PHASE`        | `r.gmr:3:18` | `result`          | 保底条件访问本次结果                |
| `18_result_in_selector/`            | `E_RESULT_PHASE`        | `r.gmr:3:30` | `result`          | 选择器条件访问本次结果               |
| `19_modulo_zero/`                   | `E_MODULUS`             | `r.gmr:3:22` | `0c == 0c`        | 模数为 0                     |
| `20_modulo_out_of_range/`           | `E_MODULUS`             | `r.gmr:3:28` | `3c`              | 余数必须小于模数                  |
| `21_counter_initial_out_of_bounds/` | `E_COUNTER_INIT`        | `r.gmr:2:30` | `3c`              | 初值大于有界计数器上界               |
| `22_always_saturated_add/`          | `E_ALWAYS_SATURATED`    | `r.gmr:4:14` | `3c`              | 增量超过所有可能可增长空间             |
| `23_zero_increment/`                | `E_COUNTER_INIT`        | `r.gmr:4:14` | `0c`              | 零增量不合法                    |
| `24_linear_invalid_cap/`            | `E_LINEAR_ARGS`         | `r.gmr:3:43` | `1/4p`            | 起始概率超过曲线上限                |
| `25_distribution_duplicate_rarity/` | `E_DISTRIBUTION`        | `r.gmr:6:9`  | `3r: 1p`          | 同一稀有度行重复                  |
| `26_distribution_fixed_over_one/`   | `E_DISTRIBUTION`        | `r.gmr:8:13` | `3/4p -> s4`      | 固定概率总和大于一                 |
| `27_distribution_fixed_under_one/`  | `E_DISTRIBUTION`        | `r.gmr:5:13` | `1/2p`            | 无剩余档时固定概率未恰为一             |
| `28_rest_and_weight/`               | `E_REST_MIX`            | `r.gmr:8:13` | `1w`              | 剩余档混用了 rest 和权重           |
| `29_rest_single_weight/`            | `E_REST_FORM`           | `r.gmr:8:13` | `1w`              | 只有一个加权剩余档，应使用 rest        |
| `30_weight_zero/`                   | `E_LITERAL_RANGE`       | `r.gmr:4:9`  | `0w`              | 权重必须严格大于零                 |
| `31_weight_empty/`                  | `E_WEIGHT_EMPTY`        | `r.gmr:3:31` | `:`               | 加权选择器没有任何分支               |
| `32_action_zero/`                   | `E_ACTION_RANGE`        | `r.gmr:6:15` | `0t`              | 一次动作不得含零抽                 |
| `33_action_too_many/`               | `E_ACTION_RANGE`        | `r.gmr:6:19` | `1001t`           | 一次动作不得超过一千抽               |
| `34_action_wrong_target/`           | `E_TYPE`                | `r.gmr:6:25` | `s`               | Action 只允许引用 Distribution |
| `35_rule_no_action/`                | `E_RULE_SHAPE`          | `r.gmr:1:1`  | `rule r`          | 缺少 Action 声明              |
| `36_guarantee_rarity_missing/`      | `E_GUARANTEE_EFFECT`    | `r.gmr:7:33` | `5r`              | 强制稀有度缺少对应分布行              |
| `37_floor_promote_no_threshold/`    | `E_GUARANTEE_EFFECT`    | `r.gmr:7:26` | `floor 4r`        | 保留更高策略缺少门槛行               |
| `38_floor_rescale_zero_mass/`       | `E_FLOOR_ZERO`          | `r.gmr:7:26` | `floor 5r`        | 重抽门槛及以上没有任何档              |
| `39_event_wrong_arguments/`         | `E_EVENT_SIGNATURE`     | `r.gmr:9:21` | `true`            | 声明 Count 事件却发出 Bool 参数    |
| `40_reset_bool/`                    | `E_TYPE`                | `r.gmr:4:15` | `f`               | Bool 状态不能用 Counter 重置原语   |
| `41_missing_block/`                 | `E_EMPTY_BLOCK`         | `r.gmr:1:7`  | `:`               | 冒号后未提供缩进块                 |
| `42_illegal_keyword_alias/`         | `E_PARSE`               | `r.gmr:1:1`  | `pool`            | 不存在 pool 前缀声明语法           |
| `43_bare_numeric_literal/`          | `E_LEX`                 | `r.gmr:5:9`  | `6:`              | 没有裸整数型，必须写 6r/6c 等        |
| `58_empty_weight_branch/`           | `E_PARSE`               | `r.gmr:4:11` | `:`               | 权重分支的冒号后没有子选择器            |
| `44_unknown_rule/`                  | `E_BIND_REF`            | `b.gmb:2:17` | `missing_rule`    | 规则来源根不存在                  |
| `45_unknown_roster/`                | `E_BIND_REF`            | `b.gmb:3:19` | `missing_roster`  | 名册来源根不存在                  |
| `46_unknown_bind_symbol/`           | `E_BIND_REF`            | `b.gmb:4:10` | `lost`            | 绑定的 Pool 符号未声明            |
| `47_missing_bind/`                  | `E_BIND_REF`            | `b.gmb:1:1`  | `banner b`        | 已声明 Pool p 未绑定            |
| `48_empty_pool/`                    | `E_SELECTOR_EMPTY`      | `b.gmb:4:24` | `[]`              | 池不能是空列表                   |
| `49_missing_roster_entry/`          | `E_ENTRY_UNKNOWN`       | `b.gmb:4:25` | `missing`         | 绑定了名册中没有的条目               |
| `50_rarity_mismatch/`               | `E_RARITY_MISMATCH`     | `b.gmb:4:25` | `x`               | 条目稀有度和 Pool[3r] 不同        |
| `51_wrong_bind_annotation/`         | `E_BIND_TYPE`           | `b.gmb:4:13` | `Pool[4r]`        | bind 类型必须精确匹配规则符号         |
| `52_duplicate_bind/`                | `E_BIND_REF`            | `b.gmb:5:10` | `p`               | 同一符号重复绑定                  |
| `53_duplicate_pool_member/`         | `E_BIND_TYPE`           | `b.gmb:4:28` | `x`               | 同一个池重复列出条目                |
| `54_undrawable_forced_item/`        | `E_ENTRY_DISABLED`      | `b.gmb:5:29` | `closed`          | 保证槽不得绑定到不可抽取条目            |
| `55_slot_unbound/`                  | `E_BIND_REF`            | `b.gmb:1:1`  | `banner b`        | 声明了 Item 槽却未绑定            |
| `56_missing_pity_group/`            | `E_BIND_REF`            | `b.gmb:1:1`  | `banner b`        | 有 @pity 状态却缺少 pity_key    |
| `57_shared_group_schema/`           | `E_SHARED_SLOT_SCHEMA`  | `b.gmb:9:5`  | `pity_key "same"` | 同组同名状态类型/上界不一致            |

### 9.1 完整反例示范：固定概率和小于一

`27_distribution_fixed_under_one/r.gmr` 的完整内容：

```text
rule r:
    p: Pool[3r]
    s: Selector[3r] = from p
    d: Distribution:
        3r: 1/2p -> s
    a: Action[1t..1t] = d
```

必须在 `1/2p` 拒绝为 `E_DISTRIBUTION`，因为没有 `rest` 或加权剩余档，固定总概率为 1/2 而不是 1。

### 9.2 完整反例示范：不可抽取条目被保证抽出

`54_undrawable_forced_item` 的三个文件如下（依次编译）：

`r.gmr`

```text
rule r:
    p: Pool[3r]
    target: Item[3r]
    s: Selector[3r] = from p
    d: Distribution:
        3r: 1p -> s
    a: Action[1t..1t] = d
    guarantee force: true -> item target
```

`t.gml`

```text
roster t:
    entry x: 3r, "x", [], true
    entry closed: 3r, "closed", [], false
```

`b.gmb`

```text
banner b:
    source_rule r
    source_roster t
    bind p: Pool[3r] = [x]
    bind target: Item[3r] = closed
```

此程序必须在 `bind target ... = closed` 的 `closed` 处拒绝为 `E_ENTRY_DISABLED`。`closed` 在名册里合法存在，错误是它被绑定到可产出的 `Item` 槽。

**不可产生“同一语法可解析为两棵树”的合法源程序。** `07_reserved_name`、`42_illegal_keyword_alias`、`43_bare_numeric_literal` 分别覆盖关键字伪装标识符、非规范化旧式宣告及无单位数字；它们都在词法/语法阶段拒绝。这条验收还必须由编译器的 parser property test 检查：任何接受的 token 流恰有一个 AST，且规范中的关键字名单与真实词法词表逐字相等。

## 10. 明确不做的语言能力

| 不做的特性 | 具体原因 / 规范替代 |
|---|---|
| 浮点、小数、百分数、裸有理数及无单位数字 | 源概率只能用规范化 `p` 有理数，消除浮点与换算差异；`%` 唯一用于计数模运算 |
| 八种或更多状态域 | 仅保留 `@pity/@progress/@banner/@action` 四种实际生命周期；其余没有运行时身份 |
| 函数、lambda、用户递归、循环、用户宏、泛型或自定义代数类型 | 不是通用计算语言；用固定原语和静态图描述选择过程与状态转移 |
| 任意整数/有理数算术表达式 | 只允许内建 `linear`、计数模条件、常量增量与精确内建归一化；避免不可验证的概率程序 |
| `if` / `else` 语句、动态脚本或异常捕获 | 条件只出现在 `choose`、`guarantee`、`update` 中；运行行为完全由静态节点决定 |
| 导入、模块、文件内动态引用、文件系统查找或旧规则迁移 | 编译单元输入列表显式给出；跨文件仅引用先前已声明的 rule/roster 和符号绑定 |
| 关键字别名、类型省略、字面量推断、语法糖、运算符重载 | 每个行为一种规范写法；解析与类型判断独立确定 |
| `counter = literal`、`reset bool`、`counter += 0c` | Bool 用赋值，Counter 仅增量与恢复初值；防止多种写法表达同一更新 |
| 用户自定义事件消息字符串、货币/票券/库存管理 | 事件必须有类型签名；外部消费者解释事件，规则不操作资产 |
| 稀有度对应的「角色/武器/音擎/光锥」枚举 | `Rarity` 只负责数值排序；种类放名册标签中，由内容决定 |
| 解释源码的 TypeScript/Python 运行时 | 只有 Rust 编译器解析/检查/降低；其他语言只调用编译结果，Python 可产生名册源文本 |
| 自动处理未公开机制、概率推断、掉率统计校准 | 属于外部内容研究。规则中的概率与曲线都是显式输入，未知事实留在注释/外部资料 |
| 可观测的「其余同时命中保底名单」 | 保底按源码先后唯一生效；其他匹配不继续计算，也不进入事件或执行结果 |
| 运行时猜测池中条目稀有度、空池自动修补、无定义的默认概率 | 编译期严格检查 Pool/Selector/Distribution；不隐藏配置缺陷 |
| 动作外的单抽临时状态、账号/游戏/会话作用域 | 分别由明确的动作生命周期和三种持久组身份承载，避免冗余存储概念 |
| 一个动作同时执行多个不同规则 | 一个 Action 只引用一份 Distribution，并共享一个 rule 的所有保证/更新；跨机制组合由上层显式分次调用 |

## 11. 实现者验收准则

实现者可直接按以下顺序实现，无需再选择语法或求值约定：

1. UTF-8 词法器产生单位明确的字面量和 `NEWLINE/INDENT/DEDENT`；拒绝非规范分数、错误缩进与未定义符号。
2. 使用第 2 节**因子分解后的 EBNF**编写 LL(1) 递归下降，或等价 PEG；形成有精确源码位置的 AST。表达式文法只涉及内建形式，无解释器。
3. 按 §5 建立符号表，按 §4 做严格类型判断，按 §6 做本地和跨文件检查；禁止前向引用、遮蔽及无声隐式转换。
4. 按 §7 构造游戏无关 IR，精确实现四状态生命周期、保底优先级、两种 Floor、`linear` 起点第一步和有序状态更新。
5. 将 §8 的三个规则、一个名册和一个卡池绑定文件**一起编译成功**，验证 §8.6 的精确有理数结果；切换组键时按 §5.2 共享/隔离状态。
6. 将 §9 对应目录的每个最小源程序单独编译，按 `rejection-tests/README.md` 对照固定错误类别与 `文件:行:列`；同一 token 流不能形成两种 AST。
7. 对任意合法 Action，逐抽概率分布以有理数精确归一；精确分析与无偏抽样消费同一分布与同一个状态转移定义。

**语义变更登记：** 相对于题设旧执行模型，本规范的保底交集处理改为**源码第一优先，其余完全不可观察**；计数越界改为**有上界则饱和，必然饱和的明显常量增量编译拒绝**；另将条件重抽的动态零质量确定为**动作原子失败**。其余抽前条件、奖池/条目效果、更新顺序、事件追加顺序与组生命周期按题设逐条保留。
