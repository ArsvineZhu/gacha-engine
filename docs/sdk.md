# SDK

Rust 库、C ABI、Python 扩展和 Node 绑定提供同一组操作。概率、计数和稀有度在单步结果里是十进制字符串（Rust 侧是 `u128` 有理数）。命中分析的多步质量是 `f64`。

动作边界和 `@action` 的当前行为见 [语言参考](language-reference.md) 的「当前执行器」。这里只列已经存在的函数。

## 操作

| 操作 | 作用 |
|---|---|
| 编译 | 按给定顺序编译内存中的源。`name` 的扩展名决定文法 |
| 加载 | 按路径读文件再编译。读失败的错误码是 `E_IO` |
| 会话 | 从编译结果建立会话，状态取声明初值 |
| 列出卡池 | 卡池名，以及每个动作的名字和次数闭区间 |
| 列出状态 | 某个卡池的状态名、域和 `count` 或 `bool` |
| 读写真值 | 会话绑在一个卡池上，按状态名读写计数或布尔。计数超出上界是 `E_COUNTER_RANGE` |
| 快照 / 恢复 | 导出或写回槽。恢复时域、组、名字和值类型必须对得上 |
| 查看一抽 | `draw`：返回当前状态下的正概率分支，不写会话 |
| 提交条目 | `apply`：若该条目在本次正概率分支中，提交这一抽并返回该分支 |
| 抽样 | `sample`：按精确权重无偏抽取一抽并提交。同一个种子得到同一个分支 |
| 命中分析 | `analyze`：在副本上计算首次或第 `copies` 次命中。不写会话 |

`draw` 的 `guarantee` 在没有保底命中时为空。分支字段是条目名、稀有度、分子、分母，以及按顺序发生的事件。

事件参数 `kind`：

| 值 | 常量（C） | 含义 | 读取 |
|---|---|---|---|
| 1 | `GACHA_ARG_COUNT` | 计数 | 十进制文本 |
| 2 | `GACHA_ARG_BOOL` | 布尔 | `bool_value`，1 为真 |
| 3 | `GACHA_ARG_RARITY` | 稀有度 | 十进制文本 |
| 4 | `GACHA_ARG_ITEM` | 条目名 | 文本 |

分析目标：

| Rust | Python | Node | C |
|---|---|---|---|
| `Target::Item(name)` | `Item("名字")` | `new Item("名字")` | `GACHA_TARGET_ITEM`（1）加条目名 |
| `Target::Any(names)` | `AnyOf(["a", "b"])` | `new AnyOf(["a", "b"])` | `GACHA_TARGET_ANY`（2）加条目名列表 |
| `Target::RarityAtLeast(n)` | `AtLeastRarity("6")` | `new AtLeastRarity("6")` | `GACHA_TARGET_RARITY`（3）加十进制文本 |
| `Target::Tag(tag)` | `Tag("limited")` | `new Tag("limited")` | `GACHA_TARGET_TAG`（4）加标签 |

`HitQuery` 的 `times` 是每个动作里的抽数，必须落在该 `Action` 的闭区间内。`actions` 是连续几个动作。`copies` 是第几次命中，从 1 计。三者都必须为正。`quantiles` 的每个数必须在 (0, 1] 内。分位数的下标是累积概率第一次达到该值的抽或动作序号；区间内达不到时，Rust 的 `index` 为 `None`，其他语言用 `has_index` / `hasIndex` 为 0。

报告里的 `probability_within` 是区间内至少命中 `copies` 次的概率，`probability_not_hit` 是仍未达到的质量。`expected_draw_if_hit` 只在有命中时存在。`expected_draws_capped` 把未命中的质量算到区间终点。动作维的字段同理。分布点从 1 编号。

下面的 Rust 片段与 `tests/corpus.rs`、`sdk/python/tests/test_contract.py`、`sdk/typescript/test/contract.cjs` 使用同一组正例。`pity6 = 65` 时六星总概率是 `29/500`。

## Rust

库包名 `gacha-engine`。公开入口在 `src/lib.rs`。

```rust
use gacha_engine::{Engine, Target};

let engine = Engine::load(&[
    "endfield.gmr",
    "sample.gml",
    "banners.gmb",
])?;
let mut session = engine.play("endfield_event_a")?;
session.set_count("pity6", 65)?;
let draw = session.draw("draw")?;
let report = session.analyze("draw", 1, 1, 1, Target::RarityAtLeast(6), vec![])?;
```

`Engine::compile` 接受 `Source` 切片，适合源文本已经在内存里的时候。计数是 `u128`。`draw` 不写会话。`Play` 在创建时记住卡池名。

| 函数 | 说明 |
|---|---|
| `Engine::compile(sources) -> Result<Engine, Diagnostic>` | `Source` 含 `name` 与 `text` |
| `Engine::load(paths) -> Result<Engine, RuntimeError>` | 路径列表 |
| `Engine::play(&self, banner) -> Result<Play, RuntimeError>` | 绑定卡池的会话 |
| `Compiled::banners(&self) -> impl Iterator<Item = &str>` | 卡池名 |
| `Compiled::actions(&self, banner) -> Result<Vec<ActionInfo>, RuntimeError>` | `name`、`min_times`、`max_times`，类型 `u128` |
| `Compiled::states(&self, banner) -> Result<Vec<StateInfo>, RuntimeError>` | `name`、`scope`、`kind` |
| `Play::get_count` / `set_count` | 值类型 `u128`，参数是状态名 |
| `Play::get_bool` / `set_bool` | 值类型 `bool` |
| `Play::snapshot(&self) -> Vec<SlotState>` | `scope`、`group`、`name`、`count: Option<u128>`、`flag: Option<bool>` |
| `Play::restore(&mut self, slots: &[SlotState]) -> Result<(), RuntimeError>` | 按槽写回 |
| `Play::draw(&self, action) -> Result<Draw, RuntimeError>` | 不写会话 |
| `Play::apply(&mut self, action, entry) -> Result<Branch, RuntimeError>` | 提交该条目 |
| `Play::sample(&mut self, action, rng: &mut Rng) -> Result<Branch, RuntimeError>` | 使用调用方的 `Rng` |
| `Play::sample_with_seed(&mut self, action, seed: u64) -> Result<Branch, RuntimeError>` | 用该种子新建 `Rng` |
| `Play::analyze(&self, action, times, actions, copies, target, quantiles)` | 不写会话。卡池名取自 `Play` |
| `Rng::new(seed: u64) -> Rng` | 抽样器 |
| `Rational::new` / `numer` / `denom` / `zero` / `one` / `is_zero` | 有理数。`checked_add`、`checked_sub`、`checked_mul`、`checked_div` 在溢出时返回 `None` |

`Branch` 的概率字段是 `Rational`。`FiredArg` 是 `Count(u128)`、`Bool(bool)`、`Rarity(u128)`、`Item(String)`。

`HitQuery` 字段：`banner`、`action`、`times`、`actions`、`copies`、`target`、`quantiles: Vec<f64>`。`HitReport` 里两个期望值是 `Option<f64>`，状态计数是 `u64`。

`Diagnostic` 字段：`code`、`file`、`line`、`column`、`expected`、`actual`。

## C

声明在 `crates/gacha-ffi/include/gacha_engine.h`，由构建 `gacha-engine-ffi` 时从 `src/lib.rs` 生成。成功返回 0，失败返回 1。失败时 `*error` 指向 `GachaError`（`code` 与 `message`）。本库返回的指针，包括错误对象，用 `gacha_free` 释放；`gacha_free` 可以接受空指针。

引擎和会话是不透明指针。字符串在调用期间由调用方保持有效。计数和稀有度是十进制 C 字符串。`gacha_session_set_bool` 把非 0 当作真，`gacha_session_get_bool` 写出 0 或 1。`gacha_session_sample` 的种子是 `uint64_t`。未命中保底时 `GachaDraw.guarantee` 是空指针。

```c
struct GachaSource sources[5];
struct GachaEngine *engine = NULL;
struct GachaError *error = NULL;
if (gacha_engine_compile(sources, 5, &engine, &error) != 0) {
    /* 读 error->code、error->message，然后 gacha_free(error) */
}
struct GachaSession *session = NULL;
gacha_engine_session(engine, "endfield_event_a", &session, &error);
gacha_session_set_count(session, "pity6", "65", &error);
struct GachaDraw *draw = NULL;
gacha_session_draw(session, "draw", &draw, &error);
gacha_free(draw);
gacha_free(session);
gacha_free(engine);
```

函数：

| 函数 | 结果 |
|---|---|
| `gacha_engine_compile` | `GachaEngine **` |
| `gacha_engine_load` | 路径是 `const char *const *` |
| `gacha_engine_session` | 第二个参数是卡池名，写出 `GachaSession **` |
| `gacha_engine_banners` | `GachaBannerList`，每项带动作数组 |
| `gacha_engine_states` | `GachaStateList` |
| `gacha_session_get_count` | 写出 `char *`，十进制 |
| `gacha_session_set_count` | 参数是状态名和十进制字符串 |
| `gacha_session_get_bool` / `gacha_session_set_bool` | `int32_t` |
| `gacha_session_snapshot` | `GachaSlotList` |
| `gacha_session_restore` | `const GachaSlot *` 与个数 |
| `gacha_session_draw` | `GachaDraw **` |
| `gacha_session_apply` | `GachaBranch **` |
| `gacha_session_sample` | 种子 `uint64_t`，结果 `GachaBranch **` |
| `gacha_session_analyze` | `const GachaHitQuery *`，结果 `GachaHitReport **` |
| `gacha_free` | 归还上述指针 |

`GachaHitReport` 的存活状态数是 `uint64_t`。`has_expected_draw` 与 `has_expected_action` 为 0 时，对应的期望字段无意义。

Rust 的 `banners` 只给卡池名，`actions(banner)` 再给动作。C 的 `gacha_engine_banners`、Python 的 `Engine.banners()` 和 Node 的 `engine.banners()` 在每个卡池下面带上动作列表。分析请求里的卡池就是打开会话时传入的那个。

## Python

模块名 `gacha_engine`，包名也是 `gacha-engine`。源码在 `sdk/python/src/lib.rs`，构建方式见 [开发](development.md)。编辑器通过 `sdk/python/gacha_engine.pyi` 显示下面这些说明；这个存根不包含实现，运行前仍要在 `sdk/python` 执行 `maturin develop`。失败抛出 `RuntimeError`，文本是 `错误码: 消息`。

先得到一份 `Engine`，再在一个卡池上打开 `Session`。会话记住卡池名，后面的读写和抽取不用重复它。

```python
from gacha_engine import AtLeastRarity, Engine

engine = Engine(["endfield.gmr", "sample.gml", "banners.gmb"])
session = engine.session("endfield_event_a")
session.set_count("pity6", "65")
draw = session.draw("draw")
report = session.analyze(
    "draw",
    times=1,
    actions=1,
    copies=1,
    target=AtLeastRarity("6"),
    quantiles=[29 / 500],
)
```

`Engine(paths)` 按给定顺序读文件。`Engine.compile(sources)` 接受内存中的源，每一项是含 `name` 和 `text` 的字典，扩展名仍然决定文法。`engine.banners()` 列出卡池和动作，`engine.states(banner)` 列出状态槽。

`session.draw(action)` 返回这一抽的正概率分支，不写会话。`guarantee` 是生效的保底名，没有保底时为 `None`。`session.apply(action, entry)` 在该条目确实可能被抽到时提交这一抽。`session.sample(action, seed)` 按精确权重无偏抽取并提交，种子必须落在 `u64` 内。计数的读写使用十进制字符串。`set_count` 超过上界时错误码是 `E_COUNTER_RANGE`。

`draw.branches` 里每个分支有 `entry`、`rarity`、`numer`、`denom`、`events`。概率是 `numer/denom`。`events` 的元素有 `name` 和 `args`。`args` 的元素有 `kind`、`text`、`bool_value`。

`session.analyze` 在副本上计算命中，不写回会话。`target` 是 `Item("名字")`、`AnyOf(["a", "b"])`、`AtLeastRarity("6")` 或 `Tag("limited")`。报告字段名与 Rust 的蛇形名字一致，例如 `probability_within`、`has_expected_draw`、`draws_distribution`。存活状态数是 Python 整数，对应 `u64`。`has_index` 为 1 时 `index` 有效。

`snapshot()` 导出全部槽。`restore` 接收字典列表，字段与 `Slot` 相同：`scope`、`group`、`name`、`kind`（`"count"` 或 `"bool"`）、`count_decimal` 或 `bool_value`。

合同测试 `sdk/python/tests/test_contract.py` 断言上面这次六星分析的 `probability_within` 接近 `29/500`，并且第一个分位数的 `index` 是 1。

`sdk/python/examples/draw_distribution.py` 用 `Engine` 读取一个目录。它把目录里的 `.gmr`、`.gml`、`.gmb` 先按扩展名（规则、名册、绑定）再按文件名排序。规范正例的三个规则文件名按字母序正好是规定的编译顺序。其他目录如果有跨文件引用，仍要保证被引用的规则和名册排在绑定前面。可选的三个参数是目录、卡池名、动作名；默认目录是规范正例，卡池 `endfield_event_a`，动作 `draw`，并在该卡池上把 `pity6` 设为 `"65"`。

## Node

包名 `gacha-engine`，绑定源码在 `sdk/typescript/src/lib.rs`。导出的是类：`Engine`、`Session`、`Item`、`AnyOf`、`AtLeastRarity`、`Tag`。方法名是驼峰形。失败抛出的 `Error.message` 以 `错误码: ` 开头。

`setCount` 的值是十进制字符串。`sample` 的种子也是十进制字符串，并且必须落在 `u64` 内。分析报告里的存活状态数是 32 位；超出时绑定写成 `2^32 - 1`。Rust、C 和 Python 这里使用 64 位。

合同测试在 `sdk/typescript/test/contract.cjs` 里写 `require("..")`，从而加载包目录。包本身是 `"private": true`。在该目录构建完成后：

```js
const { AtLeastRarity, Engine } = require(".");

const engine = Engine.compile(sources);
const session = engine.session("endfield_event_a");
session.setCount("pity6", "65");
const draw = session.draw("draw");
const report = session.analyze("draw", new AtLeastRarity("6"), {
  times: 1,
  actions: 1,
  copies: 1,
  quantiles: [29 / 500],
});
```

该合同测试断言六星分子分母约分后是 `29` 和 `500`，五星是 `3/500`，四星是 `51/1000`。

结果对象仍是生成的类型：`GachaDraw`、`GachaBranch`、`GachaEvent`、`GachaArg`、`GachaBannerInfo`、`GachaActionInfo`、`GachaStateInfo`、`GachaSlot`、`GachaHitReport`、`GachaHitPoint`、`GachaQuantile`。报告字段包括 `probabilityWithin`、`hasExpectedDraw`、`drawsDistribution`、`drawQuantiles`、`hasIndex`。

## 运行时错误码

编译失败经 `load` 或各语言绑定传出时，`code` 仍是 [语言参考](language-reference.md) 里的编译错误码。下面这些码由执行和分析产生：

| 错误码 | 何时 |
|---|---|
| `E_IO` | `load` 读文件失败 |
| `E_BANNER` | 没有这个卡池 |
| `E_ACTION` | 该卡池没有这个动作 |
| `E_STATE` | 没有这个状态，或快照里的域无法识别 |
| `E_STATE_TYPE` | 把计数槽当布尔槽，或相反；恢复时值的种类不对 |
| `E_COUNTER_RANGE` | 写入的计数超过上界 |
| `E_ENTRY` | `apply` 的条目不在本次正概率分支中；分析目标的条目不在该卡池 |
| `E_DRAW_EMPTY_MASS` | 这一抽没有正概率质量，包括条件重抽的可用质量为 0 |
| `E_RANGE` | 次数、种子、分位数或绑定层的数值不在范围内；分析区间的抽数乘积溢出 `u32` |
| `E_TARGET` | 目标种类未知、条目集合为空、稀有度为 0，或标签为空、在该卡池的名册标签中不存在 |
| `E_ANALYSIS` | 多步浮点质量不是有限的非负数 |

`apply` 在整次分布都没有正概率时也使用 `E_DRAW_EMPTY_MASS`，而不是 `E_ENTRY`。
