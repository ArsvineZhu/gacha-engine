use napi::bindgen_prelude::*;
use napi_derive::napi;

use gacha_engine::{Branch, Draw, FiredArg, RuntimeError, SlotState, Source, Target};

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

fn report_of(report: gacha_engine::HitReport) -> GachaHitReport {
    let (expected_draw_if_hit, has_expected_draw) = match report.expected_draw_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
    let (expected_action_if_hit, has_expected_action) = match report.expected_action_if_hit {
        Some(value) => (value, 1),
        None => (0.0, 0),
    };
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
        peak_surviving_states: u32::try_from(report.peak_surviving_states).unwrap_or(u32::MAX),
        final_surviving_states: u32::try_from(report.final_surviving_states).unwrap_or(u32::MAX),
        draws_distribution: report.draws_distribution.iter().map(point_of).collect(),
        actions_distribution: report.actions_distribution.iter().map(point_of).collect(),
        draw_quantiles: report.draw_quantiles.iter().map(quantile_of).collect(),
        action_quantiles: report.action_quantiles.iter().map(quantile_of).collect(),
    }
}

/// 指定一个条目。分析时只把抽到这个条目算作命中。
#[napi]
pub struct Item {
    name: String,
}

#[napi]
impl Item {
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

/// 指定一组条目。抽到其中任何一个都算命中。
#[napi]
pub struct AnyOf {
    names: Vec<String>,
}

#[napi]
impl AnyOf {
    #[napi(constructor)]
    pub fn new(names: Vec<String>) -> Result<Self> {
        if names.is_empty() {
            return Err(failure(RuntimeError::new("E_TARGET", "条目列表不能为空")));
        }
        Ok(Self { names })
    }
}

/// 指定最低稀有度。可以传数字或十进制文本。
#[napi]
pub struct AtLeastRarity {
    rarity: String,
}

#[napi]
impl AtLeastRarity {
    #[napi(constructor)]
    pub fn new(rarity: Either<String, u32>) -> Result<Self> {
        let text = match rarity {
            Either::A(text) => text,
            Either::B(number) => number.to_string(),
        };
        parse_u128(&text)?;
        Ok(Self { rarity: text })
    }
}

/// 指定名册标签。抽到带这个标签的条目算命中。
#[napi]
pub struct Tag {
    tag: String,
}

#[napi]
impl Tag {
    #[napi(constructor)]
    pub fn new(tag: String) -> Self {
        Self { tag }
    }
}

/// 一份编译好的规则。`new Engine(paths)` 从文件加载，`Engine.compile(sources)` 从内存中的源文本加载。
#[napi]
pub struct Engine {
    inner: gacha_engine::Engine,
}

#[napi]
impl Engine {
    /// 从文件加载并编译。顺序就是编译顺序。
    #[napi(constructor)]
    pub fn new(paths: Vec<String>) -> Result<Self> {
        let files: Vec<std::path::PathBuf> =
            paths.into_iter().map(std::path::PathBuf::from).collect();
        Ok(Self {
            inner: gacha_engine::Engine::load(&files).map_err(failure)?,
        })
    }

    /// 从内存中的源文本编译。每一项含 `name` 和 `text`，扩展名决定文法。
    #[napi(factory)]
    pub fn compile(sources: Vec<GachaSource>) -> Result<Self> {
        let names: Vec<String> = sources.iter().map(|source| source.name.clone()).collect();
        let texts: Vec<String> = sources.iter().map(|source| source.text.clone()).collect();
        let views: Vec<Source<'_>> = names
            .iter()
            .zip(texts.iter())
            .map(|(name, text)| Source { name, text })
            .collect();
        Ok(Self {
            inner: gacha_engine::Engine::compile(&views).map_err(|error| failure(error.into()))?,
        })
    }

    /// 打开一个卡池上的会话。之后的方法不再重复卡池名。
    #[napi]
    pub fn session(&self, banner: String) -> Result<Session> {
        Ok(Session {
            inner: self.inner.play(&banner).map_err(failure)?,
        })
    }

    /// 列出一个卡池声明的状态槽。
    #[napi]
    pub fn states(&self, banner: String) -> Result<Vec<GachaStateInfo>> {
        Ok(self
            .inner
            .states(&banner)
            .map_err(failure)?
            .into_iter()
            .map(|state| GachaStateInfo {
                name: state.name,
                scope: state.scope,
                kind: state.kind,
            })
            .collect())
    }

    /// 列出卡池和每个卡池的动作。
    #[napi]
    pub fn banners(&self) -> Result<Vec<GachaBannerInfo>> {
        let mut banners = Vec::new();
        for name in self.inner.banners() {
            let actions = self
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
}

/// 某个卡池上的会话。计数、保底和动作状态都留在这里。
#[napi]
pub struct Session {
    inner: gacha_engine::Play,
}

#[napi(object)]
pub struct AnalyzeOptions {
    pub times: u32,
    pub actions: u32,
    pub copies: Option<u32>,
    pub quantiles: Option<Vec<f64>>,
}

#[napi]
impl Session {
    /// 读取计数器，返回十进制字符串。
    #[napi]
    pub fn get_count(&self, name: String) -> Result<String> {
        self.inner.get_count(&name).map(decimal).map_err(failure)
    }

    /// 把计数器设为十进制文本。超过上界时错误码是 `E_COUNTER_RANGE`。
    #[napi]
    pub fn set_count(&mut self, name: String, value: String) -> Result<()> {
        self.inner
            .set_count(&name, parse_u128(&value)?)
            .map_err(failure)
    }

    #[napi]
    pub fn get_bool(&self, name: String) -> Result<bool> {
        self.inner.get_bool(&name).map_err(failure)
    }

    #[napi]
    pub fn set_bool(&mut self, name: String, value: bool) -> Result<()> {
        self.inner.set_bool(&name, value).map_err(failure)
    }

    /// 导出全部槽，供以后 `restore`。
    #[napi]
    pub fn snapshot(&self) -> Vec<GachaSlot> {
        self.inner
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

    /// 按快照写回。字段与 `GachaSlot` 相同。
    #[napi]
    pub fn restore(&mut self, slots: Vec<GachaSlot>) -> Result<()> {
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
            parsed.push(SlotState {
                scope: slot.scope,
                group: slot.group,
                name: slot.name,
                count,
                flag,
            });
        }
        self.inner.restore(&parsed).map_err(failure)
    }

    /// 查看当前状态下这个动作的一次抽取。不写会话。
    #[napi]
    pub fn draw(&self, action: String) -> Result<GachaDraw> {
        self.inner
            .draw(&action)
            .map(|draw| draw_of(&draw))
            .map_err(failure)
    }

    /// 若条目在本次正概率分支中，提交这一抽。
    #[napi]
    pub fn apply(&mut self, action: String, entry: String) -> Result<GachaBranch> {
        self.inner
            .apply(&action, &entry)
            .map(|branch| branch_of(&branch))
            .map_err(failure)
    }

    /// 按精确权重无偏抽取并提交一抽。种子是十进制文本，必须落在 64 位内。
    #[napi]
    pub fn sample(&mut self, action: String, seed: String) -> Result<GachaBranch> {
        let seed = parse_u128(&seed)?;
        let seed = u64::try_from(seed)
            .map_err(|_| failure(RuntimeError::new("E_RANGE", "种子超出 64 位")))?;
        self.inner
            .sample_with_seed(&action, seed)
            .map(|branch| branch_of(&branch))
            .map_err(failure)
    }

    /// 在副本上计算命中。`target` 是 `Item`、`AnyOf`、`AtLeastRarity` 或 `Tag`。
    #[napi]
    pub fn analyze(
        &self,
        action: String,
        target: Either4<&Item, &AnyOf, &AtLeastRarity, &Tag>,
        options: AnalyzeOptions,
    ) -> Result<GachaHitReport> {
        let parsed = match target {
            Either4::A(item) => Target::Item(item.name.clone()),
            Either4::B(any) => Target::Any(any.names.clone()),
            Either4::C(rarity) => Target::RarityAtLeast(parse_u128(&rarity.rarity)?),
            Either4::D(tag) => Target::Tag(tag.tag.clone()),
        };
        self.inner
            .analyze(
                &action,
                options.times,
                options.actions,
                options.copies.unwrap_or(1),
                parsed,
                options.quantiles.unwrap_or_default(),
            )
            .map(report_of)
            .map_err(failure)
    }
}
