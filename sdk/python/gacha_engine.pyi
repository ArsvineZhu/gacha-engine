"""GachaRule 的 Python 接口。

先构造 `Engine`，再在一个卡池上打开 `Session`。单步概率用十进制分子和分母表示。
运行时模块由 maturin 编译的本地扩展提供；这个文件只给编辑器显示说明和类型。
"""

from collections.abc import Sequence
from typing import TypedDict


class Arg:
    """事件的一个参数。

    `kind` 决定读哪个字段：1 是计数字符串 `text`，2 是布尔 `bool_value`（1 为真），
    3 是稀有度字符串 `text`，4 是条目名 `text`。
    """

    kind: int
    text: str
    bool_value: int


class Event:
    """规则在这一抽里发出的事件。

    引擎不解释货币或库存，只按源码顺序把事件交给调用方。
    """

    name: str
    args: list[Arg]


class Branch:
    """一次抽取里概率大于 0 的一条结果。

    `numer` 和 `denom` 是约分后的十进制字符串，概率是 `numer/denom`。
    `rarity` 也是十进制字符串。
    """

    entry: str
    rarity: str
    numer: str
    denom: str
    events: list[Event]


class Draw:
    """当前状态下一次抽取的全部分支。这次查询不修改会话。

    `guarantee` 是唯一生效的保底名。这一抽没有保底时为 None。
    其余同时满足的保底不会出现在结果里。
    """

    guarantee: str | None
    branches: list[Branch]


class Action:
    """卡池上的一个动作。`min_times` 和 `max_times` 是一次调用允许的抽数闭区间。"""

    name: str
    min_times: int
    max_times: int


class Banner:
    """编译结果里的一个卡池，以及它能调用的动作。"""

    name: str
    actions: list[Action]


class State:
    """卡池上的一个状态槽。

    `kind` 是 `count` 或 `bool`。`scope` 是 `pity`、`progress`、`banner` 或 `action`。
    """

    name: str
    scope: str
    kind: str


class Slot:
    """会话快照里的一个槽。恢复时域、组、名字和值类型必须与快照一致。"""

    scope: str
    group: str
    name: str
    kind: str
    count_decimal: str | None
    bool_value: int


class HitPoint:
    """命中分布上的一个点。`index` 从 1 计。

    三条概率分别是这一位的命中、累计和仍未命中的质量。
    """

    index: int
    hit_probability: float
    cumulative_probability: float
    survival_probability: float
    surviving_states: int


class Quantile:
    """一个分位数。

    `has_index` 为 1 时，`index` 是累计概率第一次达到该分位的抽或动作序号。
    区间内达不到时 `has_index` 为 0。
    """

    quantile: float
    index: int
    has_index: int


class HitReport:
    """在一份会话副本上算出的命中报告。计算不写回原会话。

    `probability_within` 是区间内至少命中指定次数的概率。
    `probability_not_hit` 是区间结束时仍未达到的质量。
    `has_expected_draw` 或 `has_expected_action` 为 0 时，对应的条件期望没有定义。
    """

    draws: int
    actions: int
    times: int
    probability_within: float
    probability_not_hit: float
    expected_draw_if_hit: float
    has_expected_draw: int
    expected_draws_capped: float
    expected_action_if_hit: float
    has_expected_action: int
    expected_actions_capped: float
    peak_surviving_states: int
    final_surviving_states: int
    draws_distribution: list[HitPoint]
    actions_distribution: list[HitPoint]
    draw_quantiles: list[Quantile]
    action_quantiles: list[Quantile]


class Item:
    """指定一个条目。分析时只把抽到这个条目算作命中。"""

    def __init__(self, name: str) -> None: ...


class AnyOf:
    """指定一组条目。抽到其中任何一个都算命中。列表不能为空。"""

    def __init__(self, names: Sequence[str]) -> None: ...


class AtLeastRarity:
    """指定最低稀有度。抽到这个稀有度或更高的条目都算命中。

    `rarity` 可以是正整数，或它的十进制文本。
    """

    def __init__(self, rarity: str | int) -> None: ...


class Tag:
    """指定名册标签。抽到带这个标签的条目算命中。"""

    def __init__(self, tag: str) -> None: ...


class Engine:
    """一份编译好的规则。

    直接构造时从文件加载。`Engine.compile` 从内存中的源文本加载。
    一次编译可以包含多个卡池。卡池之间是否共享保底，由规则里的 `@pity`
    和绑定文件的 `pity_key` 决定。
    """

    def __init__(self, paths: Sequence[str]) -> None:
        """从文件加载并编译。

        `paths` 的顺序就是编译顺序。被引用的规则和名册必须排在卡池绑定前面。
        读文件失败时错误码是 `E_IO`。
        """

    @staticmethod
    def compile(sources: Sequence[SourceText]) -> Engine:
        """从内存中的源文本编译。

        每一项是字典，必须含有 `name` 和 `text`。`name` 的扩展名决定文法：
        `.gmr` 是机制，`.gml` 是名册，`.gmb` 是卡池绑定。
        """

    def session(self, banner: str) -> Session:
        """打开一个卡池上的会话。

        计数和保底从声明初值开始。随后对这个会话调用 `set_count`、`draw`、
        `analyze` 时不用再写卡池名。
        """

    def banners(self) -> list[Banner]:
        """列出这次编译里的卡池，以及每个卡池能调用的动作。"""

    def states(self, banner: str) -> list[State]:
        """列出一个卡池声明的状态槽。"""


class Session:
    """某个卡池上的可写会话。计数、保底和 `@action` 都留在这里。

    会话在创建时记住卡池名。同一个 `Engine` 打开的会话互不影响。
    """

    def get_count(self, name: str) -> str:
        """读取计数器的当前值，返回十进制字符串。"""

    def set_count(self, name: str, value: str) -> None:
        """把计数器设为十进制文本 `value`。超过声明上界时错误码是 `E_COUNTER_RANGE`。"""

    def get_bool(self, name: str) -> bool:
        """读取布尔状态。"""

    def set_bool(self, name: str, value: bool) -> None:
        """写入布尔状态。"""

    def snapshot(self) -> list[Slot]:
        """导出全部槽，供以后 `restore`。"""

    def restore(self, slots: Sequence[SlotRecord]) -> None:
        """按快照写回。

        字典字段与 `Slot` 相同：`scope`、`group`、`name`、`kind`，
        以及 `count_decimal` 或 `bool_value`。
        """

    def draw(self, action: str) -> Draw:
        """查看当前状态下这个动作的一次抽取。

        返回每条概率大于 0 的分支，不写会话，也不重置 `@action`。
        `action` 是规则里的动作名，例如 `draw`。
        """

    def apply(self, action: str, entry: str) -> Branch:
        """如果 `entry` 在这一抽的正概率分支里，就提交这一抽并返回该分支。"""

    def sample(self, action: str, seed: int) -> Branch:
        """按精确权重无偏抽取并提交一抽。

        同一个种子得到同一个分支。种子必须落在 `u64` 内。
        """

    def analyze(
        self,
        action: str,
        *,
        times: int,
        actions: int,
        target: Item | AnyOf | AtLeastRarity | Tag,
        copies: int = 1,
        quantiles: Sequence[float] | None = None,
    ) -> HitReport:
        """在会话副本上计算首次或第 `copies` 次命中。不写回这个会话。

        `times` 是每个动作里的抽数，必须落在该动作的闭区间内。
        `actions` 是连续几个动作。`copies` 从 1 计。
        `quantiles` 的每个数必须在 (0, 1] 内。
        """


class SourceText(TypedDict):
    """`Engine.compile` 的一项源文本。`name` 的扩展名决定文法。"""

    name: str
    text: str


class SlotRecord(TypedDict, total=False):
    """`Session.restore` 接受的槽。字段与 `Slot` 相同。"""

    scope: str
    group: str
    name: str
    kind: str
    count_decimal: str
    bool_value: int
