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

pub struct Compiled {
    image: Arc<image::Image>,
}

impl Compiled {
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
