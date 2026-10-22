mod ast;
mod check;
mod diagnostic;
mod exec;
mod image;
mod lex;
mod parse;
mod rational;

pub use diagnostic::Diagnostic;
pub use exec::{Branch, Draw, Fired, FiredArg, Rng, RuntimeError, Session};
pub use lex::KEYWORDS;
pub use rational::Rational;

use std::path::Path;

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
    Ok(Compiled { image })
}

pub struct Compiled {
    image: image::Image,
}

impl Compiled {
    pub fn session(&self) -> Session<'_> {
        Session::new(&self.image)
    }

    pub fn banners(&self) -> impl Iterator<Item = &str> {
        self.image.banners.iter().map(|banner| banner.name.as_str())
    }
}
