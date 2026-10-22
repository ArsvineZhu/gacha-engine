use napi::bindgen_prelude::*;
use napi_derive::napi;

use gacha_engine::{Branch, Draw, FiredArg, HitQuery, RuntimeError, Source, Target};

fn failure(error: RuntimeError) -> Error {
    Error::new(
        Status::GenericFailure,
        format!("{}: {}", error.code, error.message),
    )
}

fn decimal(value: u128) -> String {
    value.to_string()
}

fn times(value: u128) -> Result<u32> {
    u32::try_from(value).map_err(|_| failure(RuntimeError::new("E_RANGE", "动作次数超出 32 位")))
}

fn parse_u128(text: &str) -> Result<u128> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(failure(RuntimeError::new(
            "E_RANGE",
            format!("不是十进制整数 {text}"),
        )));
    }
    text.parse()
        .map_err(|_| failure(RuntimeError::new("E_RANGE", format!("整数超出范围 {text}"))))
}

#[napi(object)]
pub struct GachaSource {
    pub name: String,
    pub text: String,
}

#[napi(object)]
pub struct GachaArg {
    pub kind: i32,
    pub text: String,
    pub bool_value: i32,
}

#[napi(object)]
pub struct GachaEvent {
    pub name: String,
    pub args: Vec<GachaArg>,
}

#[napi(object)]
pub struct GachaBranch {
    pub entry: String,
    pub rarity: String,
    pub numer: String,
    pub denom: String,
    pub events: Vec<GachaEvent>,
}

#[napi(object)]
pub struct GachaDraw {
    pub guarantee: Option<String>,
    pub branches: Vec<GachaBranch>,
}

#[napi(object)]
pub struct GachaActionInfo {
    pub name: String,
    pub min_times: u32,
    pub max_times: u32,
}

#[napi(object)]
pub struct GachaBannerInfo {
    pub name: String,
    pub actions: Vec<GachaActionInfo>,
}

#[napi(object)]
pub struct GachaStateInfo {
    pub name: String,
    pub scope: String,
    pub kind: String,
}

#[napi(object)]
pub struct GachaSlot {
    pub scope: String,
    pub group: String,
    pub name: String,
    pub kind: String,
    pub count_decimal: Option<String>,
    pub bool_value: i32,
}

#[napi(object)]
pub struct GachaHitPoint {
    pub index: u32,
    pub hit_probability: f64,
    pub cumulative_probability: f64,
    pub survival_probability: f64,
    pub surviving_states: u32,
}

#[napi(object)]
pub struct GachaQuantile {
    pub quantile: f64,
    pub index: u32,
    pub has_index: i32,
}

#[napi(object)]
pub struct GachaHitReport {
    pub draws: u32,
    pub actions: u32,
    pub times: u32,
    pub probability_within: f64,
    pub probability_not_hit: f64,
    pub expected_draw_if_hit: f64,
    pub has_expected_draw: i32,
    pub expected_draws_capped: f64,
    pub expected_action_if_hit: f64,
    pub has_expected_action: i32,
    pub expected_actions_capped: f64,
    pub peak_surviving_states: u32,
    pub final_surviving_states: u32,
    pub draws_distribution: Vec<GachaHitPoint>,
    pub actions_distribution: Vec<GachaHitPoint>,
    pub draw_quantiles: Vec<GachaQuantile>,
    pub action_quantiles: Vec<GachaQuantile>,
}

#[napi(object)]
pub struct GachaHitQuery {
    pub banner: String,
    pub action: String,
    pub times: u32,
    pub actions: u32,
    pub copies: u32,
    pub target_kind: String,
    pub target_item: Option<String>,
    pub target_items: Option<Vec<String>>,
    pub target_rarity: Option<String>,
    pub target_tag: Option<String>,
    pub quantiles: Option<Vec<f64>>,
}

#[napi]
pub struct GachaEngine {
    inner: gacha_engine::Compiled,
}

#[napi]
pub struct GachaSession {
    inner: gacha_engine::Session,
}

fn arg_of(arg: &FiredArg) -> GachaArg {
    match arg {
        FiredArg::Count(value) => GachaArg {
            kind: 1,
            text: decimal(*value),
            bool_value: 0,
        },
        FiredArg::Bool(value) => GachaArg {
            kind: 2,
            text: String::new(),
            bool_value: i32::from(*value),
        },
        FiredArg::Rarity(value) => GachaArg {
            kind: 3,
            text: decimal(*value),
            bool_value: 0,
        },
        FiredArg::Item(value) => GachaArg {
            kind: 4,
            text: value.clone(),
            bool_value: 0,
        },
    }
}

fn branch_of(branch: &Branch) -> GachaBranch {
    GachaBranch {
        entry: branch.entry.clone(),
        rarity: decimal(branch.rarity),
        numer: decimal(branch.probability.numer()),
        denom: decimal(branch.probability.denom()),
        events: branch
            .events
            .iter()
            .map(|event| GachaEvent {
                name: event.name.clone(),
                args: event.args.iter().map(arg_of).collect(),
            })
            .collect(),
    }
}

fn draw_of(draw: &Draw) -> GachaDraw {
    GachaDraw {
        guarantee: draw.guarantee.clone(),
        branches: draw.branches.iter().map(branch_of).collect(),
    }
}

fn point_of(point: &gacha_engine::HitPoint) -> GachaHitPoint {
    GachaHitPoint {
        index: point.index,
        hit_probability: point.hit_probability,
        cumulative_probability: point.cumulative_probability,
        survival_probability: point.survival_probability,
        surviving_states: u32::try_from(point.surviving_states).unwrap_or(u32::MAX),
    }
}

fn quantile_of(quantile: &gacha_engine::Quantile) -> GachaQuantile {
    GachaQuantile {
        quantile: quantile.quantile,
        index: quantile.index.unwrap_or(0),
        has_index: i32::from(quantile.index.is_some()),
    }
}

#[napi(js_name = "gacha_engine_compile")]
pub fn gacha_engine_compile(sources: Vec<GachaSource>) -> Result<GachaEngine> {
    let names: Vec<String> = sources.iter().map(|source| source.name.clone()).collect();
    let texts: Vec<String> = sources.iter().map(|source| source.text.clone()).collect();
    let views: Vec<Source<'_>> = names
        .iter()
        .zip(texts.iter())
        .map(|(name, text)| Source { name, text })
        .collect();
    let inner = gacha_engine::compile(&views).map_err(|error| failure(error.into()))?;
    Ok(GachaEngine { inner })
}

#[napi(js_name = "gacha_engine_load")]
pub fn gacha_engine_load(paths: Vec<String>) -> Result<GachaEngine> {
    let files: Vec<std::path::PathBuf> = paths.into_iter().map(std::path::PathBuf::from).collect();
    let inner = gacha_engine::load(&files).map_err(failure)?;
    Ok(GachaEngine { inner })
}

#[napi(js_name = "gacha_engine_session")]
pub fn gacha_engine_session(engine: &GachaEngine) -> GachaSession {
    GachaSession {
        inner: engine.inner.session(),
    }
}

#[napi(js_name = "gacha_engine_banners")]
pub fn gacha_engine_banners(engine: &GachaEngine) -> Result<Vec<GachaBannerInfo>> {
    let mut banners = Vec::new();
    for name in engine.inner.banners() {
        let actions = engine
            .inner
            .actions(name)
            .map_err(failure)?
            .into_iter()
            .map(|action| {
                Ok(GachaActionInfo {
                    name: action.name,
                    min_times: times(action.min_times)?,
                    max_times: times(action.max_times)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        banners.push(GachaBannerInfo {
            name: name.to_string(),
            actions,
        });
    }
    Ok(banners)
}

#[napi(js_name = "gacha_engine_states")]
pub fn gacha_engine_states(engine: &GachaEngine, banner: String) -> Result<Vec<GachaStateInfo>> {
    engine.inner.states(&banner).map_err(failure).map(|states| {
        states
            .into_iter()
            .map(|state| GachaStateInfo {
                name: state.name,
                scope: state.scope,
                kind: state.kind,
            })
            .collect()
    })
}

#[napi(js_name = "gacha_session_get_count")]
pub fn gacha_session_get_count(
    session: &GachaSession,
    banner: String,
    name: String,
) -> Result<String> {
    session
        .inner
        .get_count(&banner, &name)
        .map(decimal)
        .map_err(failure)
}

#[napi(js_name = "gacha_session_set_count")]
pub fn gacha_session_set_count(
    session: &mut GachaSession,
    banner: String,
    name: String,
    value: String,
) -> Result<()> {
    session
        .inner
        .set_count(&banner, &name, parse_u128(&value)?)
        .map_err(failure)
}

#[napi(js_name = "gacha_session_get_bool")]
pub fn gacha_session_get_bool(
    session: &GachaSession,
    banner: String,
    name: String,
) -> Result<bool> {
    session.inner.get_bool(&banner, &name).map_err(failure)
}

#[napi(js_name = "gacha_session_set_bool")]
pub fn gacha_session_set_bool(
    session: &mut GachaSession,
    banner: String,
    name: String,
    value: bool,
) -> Result<()> {
    session
        .inner
        .set_bool(&banner, &name, value)
        .map_err(failure)
}

#[napi(js_name = "gacha_session_snapshot")]
pub fn gacha_session_snapshot(session: &GachaSession) -> Vec<GachaSlot> {
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
            GachaSlot {
                scope: slot.scope,
                group: slot.group,
                name: slot.name,
                kind: kind.to_string(),
                count_decimal: slot.count.map(decimal),
                bool_value: i32::from(slot.flag.unwrap_or(false)),
            }
        })
        .collect()
}

#[napi(js_name = "gacha_session_restore")]
pub fn gacha_session_restore(session: &mut GachaSession, slots: Vec<GachaSlot>) -> Result<()> {
    let mut parsed = Vec::new();
    for slot in slots {
        let (count, flag) = if slot.kind == "count" {
            (
                Some(parse_u128(slot.count_decimal.as_deref().unwrap_or(""))?),
                None,
            )
        } else if slot.kind == "bool" {
            (None, Some(slot.bool_value != 0))
        } else {
            return Err(failure(RuntimeError::new(
                "E_STATE_TYPE",
                format!("未知状态类型 {}", slot.kind),
            )));
        };
        parsed.push(gacha_engine::SlotState {
            scope: slot.scope,
            group: slot.group,
            name: slot.name,
            count,
            flag,
        });
    }
    session.inner.restore(&parsed).map_err(failure)
}

#[napi(js_name = "gacha_session_draw")]
pub fn gacha_session_draw(
    session: &GachaSession,
    banner: String,
    action: String,
) -> Result<GachaDraw> {
    session
        .inner
        .draw(&banner, &action)
        .map(|draw| draw_of(&draw))
        .map_err(failure)
}

#[napi(js_name = "gacha_session_apply")]
pub fn gacha_session_apply(
    session: &mut GachaSession,
    banner: String,
    action: String,
    entry: String,
) -> Result<GachaBranch> {
    session
        .inner
        .apply(&banner, &action, &entry)
        .map(|branch| branch_of(&branch))
        .map_err(failure)
}

#[napi(js_name = "gacha_session_sample")]
pub fn gacha_session_sample(
    session: &mut GachaSession,
    banner: String,
    action: String,
    seed: String,
) -> Result<GachaBranch> {
    let seed = parse_u128(&seed)?;
    let seed =
        u64::try_from(seed).map_err(|_| failure(RuntimeError::new("E_RANGE", "种子超出 64 位")))?;
    session
        .inner
        .sample_with_seed(&banner, &action, seed)
        .map(|branch| branch_of(&branch))
        .map_err(failure)
}

#[napi(js_name = "gacha_session_analyze")]
pub fn gacha_session_analyze(
    session: &GachaSession,
    query: GachaHitQuery,
) -> Result<GachaHitReport> {
    let target = match query.target_kind.as_str() {
        "item" => Target::Item(query.target_item.unwrap_or_default()),
        "any_item" => Target::Any(query.target_items.unwrap_or_default()),
        "rarity_at_least" => {
            Target::RarityAtLeast(parse_u128(query.target_rarity.as_deref().unwrap_or(""))?)
        }
        "tag" => Target::Tag(query.target_tag.unwrap_or_default()),
        _ => return Err(failure(RuntimeError::new("E_TARGET", "未知目标类型"))),
    };
    let parsed = HitQuery {
        banner: query.banner,
        action: query.action,
        times: query.times,
        actions: query.actions,
        copies: query.copies,
        target,
        quantiles: query.quantiles.unwrap_or_default(),
    };
    let report = session.inner.analyze(&parsed).map_err(failure)?;
    let (expected_draw_if_hit, has_expected_draw) = match report.expected_draw_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
    let (expected_action_if_hit, has_expected_action) = match report.expected_action_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
    Ok(GachaHitReport {
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
        peak_surviving_states: u32::try_from(report.peak_surviving_states).unwrap_or(u32::MAX),
        final_surviving_states: u32::try_from(report.final_surviving_states).unwrap_or(u32::MAX),
        draws_distribution: report.draws_distribution.iter().map(point_of).collect(),
        actions_distribution: report.actions_distribution.iter().map(point_of).collect(),
        draw_quantiles: report.draw_quantiles.iter().map(quantile_of).collect(),
        action_quantiles: report.action_quantiles.iter().map(quantile_of).collect(),
    })
}
