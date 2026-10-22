use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use gacha_engine_core::{
    Branch, Draw, FiredArg, HitPoint, HitReport, Quantile, RuntimeError, Source, Target,
};

fn py_err(error: RuntimeError) -> PyErr {
    PyRuntimeError::new_err(format!("{}: {}", error.code, error.message))
}

fn decimal(value: u128) -> String {
    value.to_string()
}

/// 事件的一个参数。
///
/// `kind` 决定读哪个字段：1 是计数字符串 `text`，2 是布尔 `bool_value`（1 为真），3 是稀有度字符串 `text`，4 是条目名 `text`。
#[pyclass(name = "Arg", module = "gacha_engine")]
struct GachaArg {
    #[pyo3(get)]
    kind: i32,
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    bool_value: i32,
}

/// 规则在这一抽里发出的事件。引擎不解释货币或库存，只按源码顺序把事件交给调用方。
#[pyclass(name = "Event", module = "gacha_engine")]
struct GachaEvent {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    args: Vec<Py<GachaArg>>,
}

/// 一次抽取里概率大于 0 的一条结果。
///
/// `numer` 和 `denom` 是约分后的十进制字符串，概率是 `numer/denom`。`rarity` 也是十进制字符串。
#[pyclass(name = "Branch", module = "gacha_engine")]
struct GachaBranch {
    #[pyo3(get)]
    entry: String,
    #[pyo3(get)]
    rarity: String,
    #[pyo3(get)]
    numer: String,
    #[pyo3(get)]
    denom: String,
    #[pyo3(get)]
    events: Vec<Py<GachaEvent>>,
}

/// 当前状态下一次抽取的全部分支。这次查询不修改会话。
///
/// `guarantee` 是唯一生效的保底名。这一抽没有保底时为 `None`。其余同时满足的保底不会出现在结果里。
#[pyclass(name = "Draw", module = "gacha_engine")]
struct GachaDraw {
    #[pyo3(get)]
    guarantee: Option<String>,
    #[pyo3(get)]
    branches: Vec<Py<GachaBranch>>,
}

/// 卡池上的一个动作。`min_times` 和 `max_times` 是一次调用允许的抽数闭区间。
#[pyclass(name = "Action", module = "gacha_engine")]
struct GachaActionInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    min_times: u32,
    #[pyo3(get)]
    max_times: u32,
}

/// 编译结果里的一个卡池，以及它能调用的动作。
#[pyclass(name = "Banner", module = "gacha_engine")]
struct GachaBannerInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    actions: Vec<Py<GachaActionInfo>>,
}

/// 卡池上的一个状态槽。`kind` 是 `count` 或 `bool`，`scope` 是 `pity`、`progress`、`banner` 或 `action`。
#[pyclass(name = "State", module = "gacha_engine")]
struct GachaStateInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    scope: String,
    #[pyo3(get)]
    kind: String,
}

/// 会话快照里的一个槽。恢复时域、组、名字和值类型必须与快照一致。
#[pyclass(name = "Slot", module = "gacha_engine")]
struct GachaSlot {
    #[pyo3(get)]
    scope: String,
    #[pyo3(get)]
    group: String,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    kind: String,
    #[pyo3(get)]
    count_decimal: Option<String>,
    #[pyo3(get)]
    bool_value: i32,
}

/// 命中分布上的一个点。`index` 从 1 计。三条概率是这一位的命中、累计和仍未命中的质量。
#[pyclass(name = "HitPoint", module = "gacha_engine")]
struct GachaHitPoint {
    #[pyo3(get)]
    index: u32,
    #[pyo3(get)]
    hit_probability: f64,
    #[pyo3(get)]
    cumulative_probability: f64,
    #[pyo3(get)]
    survival_probability: f64,
    #[pyo3(get)]
    surviving_states: u64,
}

/// 一个分位数。`has_index` 为 1 时，`index` 是累计概率第一次达到该分位的抽或动作序号；区间内达不到时 `has_index` 为 0。
#[pyclass(name = "Quantile", module = "gacha_engine")]
struct GachaQuantile {
    #[pyo3(get)]
    quantile: f64,
    #[pyo3(get)]
    index: u32,
    #[pyo3(get)]
    has_index: i32,
}

/// 在一份会话副本上算出的命中报告。计算不写回原会话。
///
/// `probability_within` 是区间内至少命中指定次数的概率。`probability_not_hit` 是区间结束时仍未达到的质量。`has_expected_draw` 或 `has_expected_action` 为 0 时，对应的条件期望没有定义。
#[pyclass(name = "HitReport", module = "gacha_engine")]
struct GachaHitReport {
    #[pyo3(get)]
    draws: u32,
    #[pyo3(get)]
    actions: u32,
    #[pyo3(get)]
    times: u32,
    #[pyo3(get)]
    probability_within: f64,
    #[pyo3(get)]
    probability_not_hit: f64,
    #[pyo3(get)]
    expected_draw_if_hit: f64,
    #[pyo3(get)]
    has_expected_draw: i32,
    #[pyo3(get)]
    expected_draws_capped: f64,
    #[pyo3(get)]
    expected_action_if_hit: f64,
    #[pyo3(get)]
    has_expected_action: i32,
    #[pyo3(get)]
    expected_actions_capped: f64,
    #[pyo3(get)]
    peak_surviving_states: u64,
    #[pyo3(get)]
    final_surviving_states: u64,
    #[pyo3(get)]
    draws_distribution: Vec<Py<GachaHitPoint>>,
    #[pyo3(get)]
    actions_distribution: Vec<Py<GachaHitPoint>>,
    #[pyo3(get)]
    draw_quantiles: Vec<Py<GachaQuantile>>,
    #[pyo3(get)]
    action_quantiles: Vec<Py<GachaQuantile>>,
}

/// 指定一个条目。分析时只把抽到这个条目算作命中。
#[pyclass(name = "Item", module = "gacha_engine")]
struct Item {
    name: String,
}

/// 指定一组条目。抽到其中任何一个都算命中。
#[pyclass(name = "AnyOf", module = "gacha_engine")]
struct AnyOf {
    names: Vec<String>,
}

/// 指定最低稀有度。抽到这个稀有度或更高的条目都算命中。
#[pyclass(name = "AtLeastRarity", module = "gacha_engine")]
struct AtLeastRarity {
    rarity: String,
}

/// 指定名册标签。抽到带这个标签的条目算命中。
#[pyclass(name = "Tag", module = "gacha_engine")]
struct Tag {
    tag: String,
}

/// 一份编译好的规则。构造时从文件加载；`compile` 从内存中的源文本加载。
///
/// 一次编译可以包含多个卡池。卡池之间是否共享保底，由规则里的 `@pity` 和绑定文件的 `pity_key` 决定。
#[pyclass(name = "Engine", module = "gacha_engine")]
struct Engine {
    inner: gacha_engine_core::Compiled,
}

/// 某个卡池上的可写会话。计数、保底和 `@action` 都留在这里。
///
/// 会话在创建时记住卡池名，后面的读取、抽取和分析不用再重复它。同一个 `Engine` 打开的会话互不影响。
#[pyclass(name = "Session", module = "gacha_engine")]
struct Session {
    inner: gacha_engine_core::Play,
}

fn arg_py(py: Python<'_>, arg: &FiredArg) -> PyResult<Py<GachaArg>> {
    let (kind, text, bool_value) = match arg {
        FiredArg::Count(value) => (1, decimal(*value), 0),
        FiredArg::Bool(value) => (2, String::new(), i32::from(*value)),
        FiredArg::Rarity(value) => (3, decimal(*value), 0),
        FiredArg::Item(value) => (4, value.clone(), 0),
    };
    Py::new(
        py,
        GachaArg {
            kind,
            text,
            bool_value,
        },
    )
}

fn event_py(py: Python<'_>, event: &gacha_engine_core::Fired) -> PyResult<Py<GachaEvent>> {
    let args = event
        .args
        .iter()
        .map(|arg| arg_py(py, arg))
        .collect::<PyResult<Vec<_>>>()?;
    Py::new(
        py,
        GachaEvent {
            name: event.name.clone(),
            args,
        },
    )
}

fn branch_py(py: Python<'_>, branch: &Branch) -> PyResult<Py<GachaBranch>> {
    let events = branch
        .events
        .iter()
        .map(|event| event_py(py, event))
        .collect::<PyResult<Vec<_>>>()?;
    Py::new(
        py,
        GachaBranch {
            entry: branch.entry.clone(),
            rarity: decimal(branch.rarity),
            numer: decimal(branch.probability.numer()),
            denom: decimal(branch.probability.denom()),
            events,
        },
    )
}

fn draw_py(py: Python<'_>, draw: &Draw) -> PyResult<Py<GachaDraw>> {
    let branches = draw
        .branches
        .iter()
        .map(|branch| branch_py(py, branch))
        .collect::<PyResult<Vec<_>>>()?;
    Py::new(
        py,
        GachaDraw {
            guarantee: draw.guarantee.clone(),
            branches,
        },
    )
}

fn point_py(py: Python<'_>, point: &HitPoint) -> PyResult<Py<GachaHitPoint>> {
    Py::new(
        py,
        GachaHitPoint {
            index: point.index,
            hit_probability: point.hit_probability,
            cumulative_probability: point.cumulative_probability,
            survival_probability: point.survival_probability,
            surviving_states: point.surviving_states,
        },
    )
}

fn quantile_py(py: Python<'_>, quantile: &Quantile) -> PyResult<Py<GachaQuantile>> {
    Py::new(
        py,
        GachaQuantile {
            quantile: quantile.quantile,
            index: quantile.index.unwrap_or(0),
            has_index: i32::from(quantile.index.is_some()),
        },
    )
}

fn report_py(py: Python<'_>, report: &HitReport) -> PyResult<Py<GachaHitReport>> {
    let (expected_draw_if_hit, has_expected_draw) = match report.expected_draw_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
    let (expected_action_if_hit, has_expected_action) = match report.expected_action_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
    Py::new(
        py,
        GachaHitReport {
            draws: report.draws,
            actions: report.actions,
            times: report.times,
            probability_within: report.probability_within,
            probability_not_hit: report.probability_not_hit,
            expected_draw_if_hit,
            has_expected_draw,
            expected_draws_capped: report.expected_draws_capped,
            expected_action_if_hit,
            has_expected_action,
            expected_actions_capped: report.expected_actions_capped,
            peak_surviving_states: report.peak_surviving_states,
            final_surviving_states: report.final_surviving_states,
            draws_distribution: report
                .draws_distribution
                .iter()
                .map(|point| point_py(py, point))
                .collect::<PyResult<_>>()?,
            actions_distribution: report
                .actions_distribution
                .iter()
                .map(|point| point_py(py, point))
                .collect::<PyResult<_>>()?,
            draw_quantiles: report
                .draw_quantiles
                .iter()
                .map(|quantile| quantile_py(py, quantile))
                .collect::<PyResult<_>>()?,
            action_quantiles: report
                .action_quantiles
                .iter()
                .map(|quantile| quantile_py(py, quantile))
                .collect::<PyResult<_>>()?,
        },
    )
}

fn parse_u128(text: &str) -> PyResult<u128> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(py_err(RuntimeError::new(
            "E_RANGE",
            format!("不是十进制整数 {text}"),
        )));
    }
    text.parse()
        .map_err(|_| py_err(RuntimeError::new("E_RANGE", format!("整数超出范围 {text}"))))
}

fn require_str(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    dict.get_item(key)?
        .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", format!("缺少 {key}"))))?
        .extract()
}

fn compile_sources(sources: &[Bound<'_, PyDict>]) -> PyResult<gacha_engine_core::Compiled> {
    let mut names = Vec::new();
    let mut texts = Vec::new();
    for source in sources {
        names.push(require_str(source, "name")?);
        texts.push(require_str(source, "text")?);
    }
    let views: Vec<Source<'_>> = names
        .iter()
        .zip(texts.iter())
        .map(|(name, text)| Source { name, text })
        .collect();
    gacha_engine_core::compile(&views).map_err(|error| py_err(error.into()))
}

fn banner_list(py: Python<'_>, engine: &Engine) -> PyResult<Vec<Py<GachaBannerInfo>>> {
    let mut banners = Vec::new();
    for name in engine.inner.banners() {
        let actions = engine
            .inner
            .actions(name)
            .map_err(py_err)?
            .into_iter()
            .map(|action| {
                Py::new(
                    py,
                    GachaActionInfo {
                        name: action.name,
                        min_times: u32::try_from(action.min_times).unwrap_or(u32::MAX),
                        max_times: u32::try_from(action.max_times).unwrap_or(u32::MAX),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        banners.push(Py::new(
            py,
            GachaBannerInfo {
                name: name.to_string(),
                actions,
            },
        )?);
    }
    Ok(banners)
}

fn state_list(py: Python<'_>, engine: &Engine, banner: &str) -> PyResult<Vec<Py<GachaStateInfo>>> {
    engine
        .inner
        .states(banner)
        .map_err(py_err)?
        .into_iter()
        .map(|state| {
            Py::new(
                py,
                GachaStateInfo {
                    name: state.name,
                    scope: state.scope,
                    kind: state.kind,
                },
            )
        })
        .collect()
}

fn snapshot_list(py: Python<'_>, session: &Session) -> PyResult<Vec<Py<GachaSlot>>> {
    session
        .inner
        .snapshot()
        .into_iter()
        .map(|slot| {
            let kind = if slot.count.is_some() {
                "count"
            } else {
                "bool"
            };
            Py::new(
                py,
                GachaSlot {
                    scope: slot.scope,
                    group: slot.group,
                    name: slot.name,
                    kind: kind.to_string(),
                    count_decimal: slot.count.map(decimal),
                    bool_value: i32::from(slot.flag.unwrap_or(false)),
                },
            )
        })
        .collect()
}

fn restore_slots(session: &mut Session, slots: &[Bound<'_, PyDict>]) -> PyResult<()> {
    let mut parsed = Vec::new();
    for slot in slots {
        let kind = require_str(slot, "kind")?;
        let (count, flag) = if kind == "count" {
            (
                Some(parse_u128(&require_str(slot, "count_decimal")?)?),
                None,
            )
        } else if kind == "bool" {
            let value: i32 = slot
                .get_item("bool_value")?
                .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", "缺少 bool_value")))?
                .extract()?;
            (None, Some(value != 0))
        } else {
            return Err(py_err(RuntimeError::new(
                "E_STATE_TYPE",
                format!("未知状态类型 {kind}"),
            )));
        };
        parsed.push(gacha_engine_core::SlotState {
            scope: require_str(slot, "scope")?,
            group: require_str(slot, "group")?,
            name: require_str(slot, "name")?,
            count,
            flag,
        });
    }
    session.inner.restore(&parsed).map_err(py_err)
}

fn read_target(target: &Bound<'_, PyAny>) -> PyResult<Target> {
    if let Ok(item) = target.downcast::<Item>() {
        return Ok(Target::Item(item.borrow().name.clone()));
    }
    if let Ok(any) = target.downcast::<AnyOf>() {
        return Ok(Target::Any(any.borrow().names.clone()));
    }
    if let Ok(rarity) = target.downcast::<AtLeastRarity>() {
        return Ok(Target::RarityAtLeast(parse_u128(&rarity.borrow().rarity)?));
    }
    if let Ok(tag) = target.downcast::<Tag>() {
        return Ok(Target::Tag(tag.borrow().tag.clone()));
    }
    Err(py_err(RuntimeError::new(
        "E_TARGET",
        "目标必须是 Item、AnyOf、AtLeastRarity 或 Tag",
    )))
}

#[pymethods]
impl Item {
    #[new]
    fn new(name: String) -> Self {
        Self { name }
    }
}

#[pymethods]
impl AnyOf {
    #[new]
    fn new(names: Vec<String>) -> PyResult<Self> {
        if names.is_empty() {
            return Err(py_err(RuntimeError::new("E_TARGET", "条目列表不能为空")));
        }
        Ok(Self { names })
    }
}

#[pymethods]
impl AtLeastRarity {
    #[new]
    fn new(rarity: &Bound<'_, PyAny>) -> PyResult<Self> {
        let text = if let Ok(text) = rarity.extract::<String>() {
            text
        } else if let Ok(number) = rarity.extract::<u64>() {
            number.to_string()
        } else {
            return Err(py_err(RuntimeError::new(
                "E_RANGE",
                "稀有度必须是正整数或它的十进制文本",
            )));
        };
        parse_u128(&text)?;
        Ok(Self { rarity: text })
    }
}

#[pymethods]
impl Tag {
    #[new]
    fn new(tag: String) -> Self {
        Self { tag }
    }
}

#[pymethods]
impl Engine {
    /// 从文件加载并编译。
    ///
    /// `paths` 的顺序就是编译顺序。被引用的规则和名册必须排在卡池绑定前面。读文件失败时错误码是 `E_IO`。
    #[new]
    fn new(paths: Vec<std::path::PathBuf>) -> PyResult<Self> {
        Ok(Self {
            inner: gacha_engine_core::load(&paths).map_err(py_err)?,
        })
    }

    /// 从内存中的源文本编译。
    ///
    /// 每一项是字典，必须含有 `name` 和 `text`。`name` 的扩展名决定文法：`.gmr` 是机制，`.gml` 是名册，`.gmb` 是卡池绑定。
    #[classmethod]
    fn compile(_cls: &Bound<'_, PyType>, sources: Vec<Bound<'_, PyDict>>) -> PyResult<Self> {
        Ok(Self {
            inner: compile_sources(&sources)?,
        })
    }

    /// 打开一个卡池上的会话。
    ///
    /// 计数和保底从声明初值开始。随后对这个会话调用 `set_count`、`draw`、`analyze` 时不用再写卡池名。
    fn session(&self, banner: &str) -> PyResult<Session> {
        Ok(Session {
            inner: self.inner.play(banner).map_err(py_err)?,
        })
    }

    /// 列出这次编译里的卡池，以及每个卡池能调用的动作。
    fn banners(&self, py: Python<'_>) -> PyResult<Vec<Py<GachaBannerInfo>>> {
        banner_list(py, self)
    }

    /// 列出一个卡池声明的状态槽。
    fn states(&self, py: Python<'_>, banner: &str) -> PyResult<Vec<Py<GachaStateInfo>>> {
        state_list(py, self, banner)
    }
}

#[pymethods]
impl Session {
    /// 读取计数器的当前值，返回十进制字符串。
    fn get_count(&self, name: &str) -> PyResult<String> {
        self.inner.get_count(name).map(decimal).map_err(py_err)
    }

    /// 把计数器设为十进制文本 `value`。超过声明上界时错误码是 `E_COUNTER_RANGE`。
    fn set_count(&mut self, name: &str, value: &str) -> PyResult<()> {
        self.inner
            .set_count(name, parse_u128(value)?)
            .map_err(py_err)
    }

    /// 读取布尔状态。
    fn get_bool(&self, name: &str) -> PyResult<bool> {
        self.inner.get_bool(name).map_err(py_err)
    }

    /// 写入布尔状态。
    fn set_bool(&mut self, name: &str, value: bool) -> PyResult<()> {
        self.inner.set_bool(name, value).map_err(py_err)
    }

    /// 导出全部槽，供以后 `restore`。
    fn snapshot(&self, py: Python<'_>) -> PyResult<Vec<Py<GachaSlot>>> {
        snapshot_list(py, self)
    }

    /// 按快照写回。字典字段与 `Slot` 相同：`scope`、`group`、`name`、`kind`，以及 `count_decimal` 或 `bool_value`。
    fn restore(&mut self, slots: Vec<Bound<'_, PyDict>>) -> PyResult<()> {
        restore_slots(self, &slots)
    }

    /// 查看当前状态下这个动作的一次抽取。
    ///
    /// 返回每条概率大于 0 的分支，不写会话，也不重置 `@action`。`action` 是规则里的动作名，例如 `draw`。
    fn draw(&self, py: Python<'_>, action: &str) -> PyResult<Py<GachaDraw>> {
        let draw = self.inner.draw(action).map_err(py_err)?;
        draw_py(py, &draw)
    }

    /// 如果 `entry` 在这一抽的正概率分支里，就提交这一抽并返回该分支。
    fn apply(&mut self, py: Python<'_>, action: &str, entry: &str) -> PyResult<Py<GachaBranch>> {
        let branch = self.inner.apply(action, entry).map_err(py_err)?;
        branch_py(py, &branch)
    }

    /// 按精确权重无偏抽取并提交一抽。同一个种子得到同一个分支。种子必须落在 `u64` 内。
    fn sample(&mut self, py: Python<'_>, action: &str, seed: u64) -> PyResult<Py<GachaBranch>> {
        let branch = self.inner.sample_with_seed(action, seed).map_err(py_err)?;
        branch_py(py, &branch)
    }

    /// 在会话副本上计算首次或第 `copies` 次命中。不写回这个会话。
    ///
    /// `times` 是每个动作里的抽数，必须落在该动作的闭区间内。`actions` 是连续几个动作。`copies` 从 1 计。`target` 是 `Item`、`AnyOf`、`AtLeastRarity` 或 `Tag`。`quantiles` 的每个数必须在 (0, 1] 内。
    #[pyo3(signature = (action, *, times, actions, target, copies=1, quantiles=None))]
    fn analyze(
        &self,
        py: Python<'_>,
        action: &str,
        times: u32,
        actions: u32,
        target: &Bound<'_, PyAny>,
        copies: u32,
        quantiles: Option<Vec<f64>>,
    ) -> PyResult<Py<GachaHitReport>> {
        let report = self
            .inner
            .analyze(
                action,
                times,
                actions,
                copies,
                read_target(target)?,
                quantiles.unwrap_or_default(),
            )
            .map_err(py_err)?;
        report_py(py, &report)
    }
}

/// GachaRule 的 Python 接口。先用 `Engine` 编译规则，再在指定卡池上打开 `Session`。单步概率用十进制分子和分母表示。
#[pymodule]
fn gacha_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Engine>()?;
    m.add_class::<Session>()?;
    m.add_class::<Item>()?;
    m.add_class::<AnyOf>()?;
    m.add_class::<AtLeastRarity>()?;
    m.add_class::<Tag>()?;
    m.add_class::<GachaDraw>()?;
    m.add_class::<GachaBranch>()?;
    m.add_class::<GachaEvent>()?;
    m.add_class::<GachaArg>()?;
    m.add_class::<GachaBannerInfo>()?;
    m.add_class::<GachaActionInfo>()?;
    m.add_class::<GachaStateInfo>()?;
    m.add_class::<GachaSlot>()?;
    m.add_class::<GachaHitReport>()?;
    m.add_class::<GachaHitPoint>()?;
    m.add_class::<GachaQuantile>()?;
    Ok(())
}
