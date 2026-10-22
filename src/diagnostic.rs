#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: u16,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub expected: String,
    pub actual: String,
}

pub(crate) fn diag(
    code: &'static str,
    file: &str,
    span: Span,
    expected: impl Into<String>,
    actual: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code,
        file: file.to_string(),
        line: span.line,
        column: span.column,
        expected: expected.into(),
        actual: actual.into(),
    }
}
