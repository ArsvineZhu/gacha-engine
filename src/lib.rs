#![forbid(unsafe_code)]

mod analysis;
mod ast;
mod check;
mod diagnostic;
mod exec;
mod fmt;
mod image;
mod lex;
mod parse;
mod rational;

pub use analysis::{HitPoint, HitQuery, HitReport, Quantile, Target};

impl Session {
    pub fn analyze(&self, query: &HitQuery) -> Result<HitReport, RuntimeError> {
        analysis::analyze(self.image(), self.memory(), query)
    }
}
pub use diagnostic::Diagnostic;
pub use exec::{
    ActionInfo, Branch, Draw, Fired, FiredArg, Rng, RuntimeError, Session, SlotState, StateInfo,
};
pub use fmt::format_source;
pub use lex::KEYWORDS;
pub use rational::Rational;

use std::path::Path;
use std::sync::Arc;

pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

pub fn compile(sources: &[Source<'_>]) -> Result<Compiled, Diagnostic> {
    let mut parsed = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let ext = Path::new(source.name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        let tokens = lex::tokenize(index as u16, source.name, source.text)?;
        let file = parse::parse(source.name, &tokens, ext)?;
        parsed.push((source.name.to_string(), file));
    }
    let image = check::check(&parsed)?;
    Ok(Compiled {
        image: Arc::new(image),
    })
}

pub fn load(paths: &[impl AsRef<Path>]) -> Result<Compiled, RuntimeError> {
    let mut names = Vec::new();
    let mut texts = Vec::new();
    for path in paths {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|error| RuntimeError::new("E_IO", format!("{}: {error}", path.display())))?;
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        names.push(name);
        texts.push(text);
    }
    let sources: Vec<Source<'_>> = names
        .iter()
        .zip(texts.iter())
        .map(|(name, text)| Source { name, text })
        .collect();
    compile(&sources).map_err(RuntimeError::from)
}

/// 一份编译好的规则，可以包含多个卡池。
///
/// 用 [`Engine::load`] 从文件读取，或用 [`Engine::compile`] 从内存中的源文本编译。
/// `name` 的扩展名决定文法。被引用的规则和名册必须排在卡池绑定前面。
pub struct Compiled {
    image: Arc<image::Image>,
}

/// [`Compiled`] 的公开名字。
pub type Engine = Compiled;

impl Compiled {
    /// 从文件加载并编译。读失败的错误码是 `E_IO`。
    pub fn load(paths: &[impl AsRef<Path>]) -> Result<Self, RuntimeError> {
        load(paths)
    }

    /// 从内存中的源文本编译。`name` 的扩展名决定文法。
    pub fn compile(sources: &[Source<'_>]) -> Result<Self, Diagnostic> {
        compile(sources)
    }

    /// 打开某个卡池上的会话。
    ///
    /// 之后的读写和抽取都作用在这个卡池上，不必再重复卡池名。未知卡池的错误码是 `E_BANNER`。
    pub fn play(&self, banner: &str) -> Result<Play, RuntimeError> {
        self.actions(banner)?;
        Ok(Play {
            banner: banner.to_string(),
            inner: self.session(),
        })
    }

    pub fn session(&self) -> Session {
        Session::new(Arc::clone(&self.image))
    }

    pub fn banners(&self) -> impl Iterator<Item = &str> {
        self.image.banners.iter().map(|banner| banner.name.as_str())
    }

    pub fn actions(&self, banner: &str) -> Result<Vec<ActionInfo>, RuntimeError> {
        self.image
            .banners
            .iter()
            .find(|item| item.name == banner)
            .map(|item| {
                item.actions
                    .iter()
                    .map(|action| ActionInfo {
                        name: action.name.clone(),
                        min_times: action.lo,
                        max_times: action.hi,
                    })
                    .collect()
            })
            .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {banner}")))
    }

    pub fn states(&self, banner: &str) -> Result<Vec<StateInfo>, RuntimeError> {
        self.image
            .banners
            .iter()
            .find(|item| item.name == banner)
            .map(|item| item.states.iter().map(StateInfo::from_state).collect())
            .ok_or_else(|| RuntimeError::new("E_BANNER", format!("未知卡池 {banner}")))
    }
}

/// 绑定在一个卡池上的会话。计数、保底和 `@action` 都留在这里。
pub struct Play {
    banner: String,
    inner: Session,
}

impl Play {
    pub fn banner(&self) -> &str {
        &self.banner
    }

    /// 读取计数器，值是任意精度整数。
    pub fn get_count(&self, name: &str) -> Result<u128, RuntimeError> {
        self.inner.get_count(&self.banner, name)
    }

    /// 写入计数器。超过声明上界时错误码是 `E_COUNTER_RANGE`。
    pub fn set_count(&mut self, name: &str, value: u128) -> Result<(), RuntimeError> {
        self.inner.set_count(&self.banner, name, value)
    }

    pub fn get_bool(&self, name: &str) -> Result<bool, RuntimeError> {
        self.inner.get_bool(&self.banner, name)
    }

    pub fn set_bool(&mut self, name: &str, value: bool) -> Result<(), RuntimeError> {
        self.inner.set_bool(&self.banner, name, value)
    }

    pub fn snapshot(&self) -> Vec<SlotState> {
        self.inner.snapshot()
    }

    pub fn restore(&mut self, slots: &[SlotState]) -> Result<(), RuntimeError> {
        self.inner.restore(slots)
    }

    /// 查看当前状态下这个动作的一次抽取。不写会话。
    pub fn draw(&self, action: &str) -> Result<Draw, RuntimeError> {
        self.inner.draw(&self.banner, action)
    }

    /// 若 `entry` 在本次正概率分支中，提交这一抽。
    pub fn apply(&mut self, action: &str, entry: &str) -> Result<Branch, RuntimeError> {
        self.inner.apply(&self.banner, action, entry)
    }

    /// 按精确权重无偏抽取并提交一抽。同一个种子得到同一个分支。
    pub fn sample_with_seed(&mut self, action: &str, seed: u64) -> Result<Branch, RuntimeError> {
        self.inner.sample_with_seed(&self.banner, action, seed)
    }

    pub fn sample(&mut self, action: &str, rng: &mut Rng) -> Result<Branch, RuntimeError> {
        self.inner.sample(&self.banner, action, rng)
    }

    /// 在副本上计算第 `copies` 次命中。不写回会话。
    pub fn analyze(
        &self,
        action: &str,
        times: u32,
        actions: u32,
        copies: u32,
        target: Target,
        quantiles: Vec<f64>,
    ) -> Result<HitReport, RuntimeError> {
        self.inner.analyze(&HitQuery {
            banner: self.banner.clone(),
            action: action.to_string(),
            times,
            actions,
            copies,
            target,
            quantiles,
        })
    }
}
