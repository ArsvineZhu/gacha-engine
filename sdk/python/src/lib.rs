use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use gacha_engine_core::{
    Branch, Draw, FiredArg, HitPoint, HitQuery, HitReport, Quantile, RuntimeError, Source, Target,
};

fn py_err(error: RuntimeError) -> PyErr {
    PyRuntimeError::new_err(format!("{}: {}", error.code, error.message))
}

fn decimal(value: u128) -> String {
    value.to_string()
}

#[pyclass]
struct GachaArg {
    #[pyo3(get)]
    kind: i32,
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    bool_value: i32,
}

#[pyclass]
struct GachaEvent {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    args: Vec<Py<GachaArg>>,
}

#[pyclass]
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

#[pyclass]
struct GachaDraw {
    #[pyo3(get)]
    guarantee: Option<String>,
    #[pyo3(get)]
    branches: Vec<Py<GachaBranch>>,
}

#[pyclass]
struct GachaActionInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    min_times: u32,
    #[pyo3(get)]
    max_times: u32,
}

#[pyclass]
struct GachaBannerInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    actions: Vec<Py<GachaActionInfo>>,
}

#[pyclass]
struct GachaStateInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    scope: String,
    #[pyo3(get)]
    kind: String,
}

#[pyclass]
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

#[pyclass]
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

#[pyclass]
struct GachaQuantile {
    #[pyo3(get)]
    quantile: f64,
    #[pyo3(get)]
    index: u32,
    #[pyo3(get)]
    has_index: i32,
}

#[pyclass]
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

#[pyclass]
struct GachaEngine {
    inner: gacha_engine_core::Compiled,
}

#[pyclass]
struct GachaSession {
    inner: gacha_engine_core::Session,
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

#[pyfunction]
fn gacha_engine_compile(
    py: Python<'_>,
    sources: Vec<Bound<'_, PyDict>>,
) -> PyResult<Py<GachaEngine>> {
    let mut names = Vec::new();
    let mut texts = Vec::new();
    for source in &sources {
        names.push(require_str(source, "name")?);
        texts.push(require_str(source, "text")?);
    }
    let views: Vec<Source<'_>> = names
        .iter()
        .zip(texts.iter())
        .map(|(name, text)| Source { name, text })
        .collect();
    let inner = gacha_engine_core::compile(&views).map_err(|error| py_err(error.into()))?;
    Py::new(py, GachaEngine { inner })
}

#[pyfunction]
fn gacha_engine_load(py: Python<'_>, paths: Vec<std::path::PathBuf>) -> PyResult<Py<GachaEngine>> {
    let inner = gacha_engine_core::load(&paths).map_err(py_err)?;
    Py::new(py, GachaEngine { inner })
}

#[pyfunction]
fn gacha_engine_session(py: Python<'_>, engine: &GachaEngine) -> PyResult<Py<GachaSession>> {
    Py::new(
        py,
        GachaSession {
            inner: engine.inner.session(),
        },
    )
}

#[pyfunction]
fn gacha_engine_banners(
    py: Python<'_>,
    engine: &GachaEngine,
) -> PyResult<Vec<Py<GachaBannerInfo>>> {
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

#[pyfunction]
fn gacha_engine_states(
    py: Python<'_>,
    engine: &GachaEngine,
    banner: &str,
) -> PyResult<Vec<Py<GachaStateInfo>>> {
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

#[pyfunction]
fn gacha_session_get_count(session: &GachaSession, banner: &str, name: &str) -> PyResult<String> {
    session
        .inner
        .get_count(banner, name)
        .map(decimal)
        .map_err(py_err)
}

#[pyfunction]
fn gacha_session_set_count(
    session: &mut GachaSession,
    banner: &str,
    name: &str,
    value: &str,
) -> PyResult<()> {
    session
        .inner
        .set_count(banner, name, parse_u128(value)?)
        .map_err(py_err)
}

#[pyfunction]
fn gacha_session_get_bool(session: &GachaSession, banner: &str, name: &str) -> PyResult<bool> {
    session.inner.get_bool(banner, name).map_err(py_err)
}

#[pyfunction]
fn gacha_session_set_bool(
    session: &mut GachaSession,
    banner: &str,
    name: &str,
    value: bool,
) -> PyResult<()> {
    session.inner.set_bool(banner, name, value).map_err(py_err)
}

#[pyfunction]
fn gacha_session_snapshot(py: Python<'_>, session: &GachaSession) -> PyResult<Vec<Py<GachaSlot>>> {
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

#[pyfunction]
fn gacha_session_restore(
    session: &mut GachaSession,
    slots: Vec<Bound<'_, PyDict>>,
) -> PyResult<()> {
    let mut parsed = Vec::new();
    for slot in &slots {
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

#[pyfunction]
fn gacha_session_draw(
    py: Python<'_>,
    session: &GachaSession,
    banner: &str,
    action: &str,
) -> PyResult<Py<GachaDraw>> {
    let draw = session.inner.draw(banner, action).map_err(py_err)?;
    draw_py(py, &draw)
}

#[pyfunction]
fn gacha_session_apply(
    py: Python<'_>,
    session: &mut GachaSession,
    banner: &str,
    action: &str,
    entry: &str,
) -> PyResult<Py<GachaBranch>> {
    let branch = session.inner.apply(banner, action, entry).map_err(py_err)?;
    branch_py(py, &branch)
}

#[pyfunction]
fn gacha_session_sample(
    py: Python<'_>,
    session: &mut GachaSession,
    banner: &str,
    action: &str,
    seed: u64,
) -> PyResult<Py<GachaBranch>> {
    let branch = session
        .inner
        .sample_with_seed(banner, action, seed)
        .map_err(py_err)?;
    branch_py(py, &branch)
}

#[pyfunction]
fn gacha_session_analyze(
    py: Python<'_>,
    session: &GachaSession,
    query: &Bound<'_, PyDict>,
) -> PyResult<Py<GachaHitReport>> {
    let kind = require_str(query, "target_kind")?;
    let target = match kind.as_str() {
        "item" => Target::Item(require_str(query, "target_item")?),
        "any_item" => Target::Any(
            query
                .get_item("target_items")?
                .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", "缺少 target_items")))?
                .extract()?,
        ),
        "rarity_at_least" => {
            Target::RarityAtLeast(parse_u128(&require_str(query, "target_rarity")?)?)
        }
        "tag" => Target::Tag(require_str(query, "target_tag")?),
        _ => {
            return Err(py_err(RuntimeError::new("E_TARGET", "未知目标类型")));
        }
    };
    let quantiles = match query.get_item("quantiles")? {
        Some(value) if !value.is_none() => value.extract()?,
        _ => Vec::new(),
    };
    let parsed = HitQuery {
        banner: require_str(query, "banner")?,
        action: require_str(query, "action")?,
        times: query
            .get_item("times")?
            .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", "缺少 times")))?
            .extract()?,
        actions: query
            .get_item("actions")?
            .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", "缺少 actions")))?
            .extract()?,
        copies: query
            .get_item("copies")?
            .ok_or_else(|| py_err(RuntimeError::new("E_RANGE", "缺少 copies")))?
            .extract()?,
        target,
        quantiles,
    };
    let report = session.inner.analyze(&parsed).map_err(py_err)?;
    report_py(py, &report)
}

#[pymodule]
fn gacha_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gacha_engine_compile, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_engine_load, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_engine_session, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_engine_banners, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_engine_states, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_get_count, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_set_count, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_get_bool, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_set_bool, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_restore, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_draw, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_apply, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_sample, m)?)?;
    m.add_function(wrap_pyfunction!(gacha_session_analyze, m)?)?;
    Ok(())
}
