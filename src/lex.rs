use crate::diagnostic::{Diagnostic, Span, diag};
use crate::rational::{Rational, gcd};

pub const KEYWORDS: &[&str] = &[
    "and",
    "banner",
    "bind",
    "choose",
    "emit",
    "entry",
    "floor",
    "from",
    "grade",
    "guarantee",
    "in",
    "item",
    "linear",
    "not",
    "or",
    "pity_key",
    "progress_key",
    "promote",
    "rarity",
    "reset",
    "rest",
    "result",
    "roster",
    "rule",
    "rescale",
    "source_roster",
    "source_rule",
    "update",
    "weighted",
];

#[cfg(test)]
const TYPES: &[&str] = &[
    "Action",
    "Bool",
    "Chance",
    "Count",
    "Counter",
    "Distribution",
    "Event",
    "Item",
    "Pool",
    "Rarity",
    "Selector",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Newline,
    Indent,
    Dedent,
    Eof,
    Ident(String),
    String(String),
    P(Rational),
    W(Rational),
    C(u128),
    T(u128),
    R(u128),
    Bool(bool),
    Scope(Scope),
    Type(TypeName),
    Keyword(Keyword),
    Colon,
    Comma,
    LBrack,
    RBrack,
    LParen,
    RParen,
    Eq,
    PlusEq,
    Arrow,
    EqEq,
    GtEq,
    Percent,
    DotDot,
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Pity,
    Progress,
    Banner,
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeName {
    Action,
    Bool,
    Chance,
    Count,
    Counter,
    Distribution,
    Event,
    Item,
    Pool,
    Rarity,
    Selector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    And,
    Banner,
    Bind,
    Choose,
    Emit,
    Entry,
    Floor,
    From,
    Grade,
    Guarantee,
    In,
    Item,
    Linear,
    Not,
    Or,
    PityKey,
    ProgressKey,
    Promote,
    Rarity,
    Reset,
    Rest,
    Result,
    Roster,
    Rule,
    Rescale,
    SourceRoster,
    SourceRule,
    Update,
    Weighted,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) struct RawToken {
    pub kind: TokenKind,
    pub text: String,
    pub line: u32,
    pub column: u32,
    pub trailing: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum Flat {
    Token(RawToken),
    Comment(String),
}

pub fn tokenize(file_index: u16, file_name: &str, source: &str) -> Result<Vec<Token>, Diagnostic> {
    let chars = decode(file_name, source)?;
    let lines = split_lines(file_name, &chars)?;
    let mut tokens = Vec::new();
    let mut stack = vec![0u32];
    let mut open_block: Option<Span> = None;

    for line in &lines {
        if line_is_ignorable(&chars, line) {
            if let Some(col) = leading_tab(&chars, line) {
                return Err(indent_error(file_name, line.no, col));
            }
            continue;
        }
        if let Some(col) = leading_tab(&chars, line) {
            return Err(indent_error(file_name, line.no, col));
        }
        if let Some(col) = raw_tab_outside_string(&chars, line) {
            let in_indent = col <= indent_width(&chars, line);
            if in_indent {
                return Err(indent_error(file_name, line.no, col));
            }
            return Err(lex_error(file_name, line.no, col, "制表符"));
        }
        let width = indent_width(&chars, line);
        if width % 4 != 0 {
            return Err(indent_error(file_name, line.no, 1));
        }
        let depth = width / 4;
        let top = *stack.last().expect("indent stack");
        let content_col = width + 1;
        if depth > top {
            if depth != top + 1 || open_block.is_none() {
                return Err(indent_error(file_name, line.no, content_col));
            }
            stack.push(depth);
            tokens.push(Token {
                kind: TokenKind::Indent,
                span: span_at(file_index, line.no, content_col),
            });
        } else {
            while *stack.last().expect("indent stack") > depth {
                stack.pop();
                tokens.push(Token {
                    kind: TokenKind::Dedent,
                    span: span_at(file_index, line.no, content_col),
                });
            }
            if *stack.last().expect("indent stack") != depth {
                return Err(indent_error(file_name, line.no, content_col));
            }
        }
        let (line_tokens, ended_with_colon) = tokenize_line(file_index, file_name, &chars, line)?;
        let colon_span = line_tokens
            .last()
            .filter(|t| t.kind == TokenKind::Colon)
            .map(|t| t.span);
        tokens.extend(line_tokens);
        tokens.push(Token {
            kind: TokenKind::Newline,
            span: span_at(file_index, line.no, line_end_col(&chars, line)),
        });
        open_block = if ended_with_colon { colon_span } else { None };
    }
    let end_line = lines.last().map(|l| l.no).unwrap_or(1);
    let end_col = lines.last().map(|l| line_end_col(&chars, l)).unwrap_or(1);
    while stack.len() > 1 {
        stack.pop();
        tokens.push(Token {
            kind: TokenKind::Dedent,
            span: span_at(file_index, end_line, end_col),
        });
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: span_at(file_index, end_line, end_col),
    });
    Ok(tokens)
}

struct Line {
    no: u32,
    start: usize,
    len: usize,
}

fn decode(file_name: &str, source: &str) -> Result<Vec<char>, Diagnostic> {
    let mut chars: Vec<char> = source.chars().collect();
    if chars.first() == Some(&'\u{feff}') {
        chars.remove(0);
    }
    if chars.contains(&'\u{feff}') {
        let idx = chars.iter().position(|c| *c == '\u{feff}').unwrap();
        let (line, col) = line_col_of(&chars, idx);
        return Err(lex_error(file_name, line, col, "BOM"));
    }
    Ok(chars)
}

fn split_lines(file_name: &str, chars: &[char]) -> Result<Vec<Line>, Diagnostic> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut no = 1u32;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\r' {
            if chars.get(i + 1) == Some(&'\n') {
                lines.push(Line {
                    no,
                    start,
                    len: i - start,
                });
                i += 2;
                start = i;
                no += 1;
            } else {
                let (line, col) = line_col_of(chars, i);
                return Err(lex_error(file_name, line, col, "CR"));
            }
        } else if chars[i] == '\n' {
            lines.push(Line {
                no,
                start,
                len: i - start,
            });
            i += 1;
            start = i;
            no += 1;
        } else {
            i += 1;
        }
    }
    if start < chars.len() {
        lines.push(Line {
            no,
            start,
            len: chars.len() - start,
        });
    }
    if lines.is_empty() {
        lines.push(Line {
            no: 1,
            start: 0,
            len: 0,
        });
    }
    Ok(lines)
}

fn line_col_of(chars: &[char], index: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, ch) in chars.iter().enumerate() {
        if i == index {
            break;
        }
        if *ch == '\n' {
            line += 1;
            col = 1;
        } else if *ch != '\r' {
            col += 1;
        }
    }
    (line, col)
}

fn line_is_ignorable(chars: &[char], line: &Line) -> bool {
    let text = &chars[line.start..line.start + line.len];
    let mut i = 0;
    while i < text.len() && text[i] == ' ' {
        i += 1;
    }
    i == text.len() || text[i] == '#'
}

fn leading_tab(chars: &[char], line: &Line) -> Option<u32> {
    let text = &chars[line.start..line.start + line.len];
    for (i, ch) in text.iter().enumerate() {
        if *ch == ' ' {
            continue;
        }
        if *ch == '\t' {
            return Some(i as u32 + 1);
        }
        return None;
    }
    None
}

fn raw_tab_outside_string(chars: &[char], line: &Line) -> Option<u32> {
    let text = &chars[line.start..line.start + line.len];
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in text.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if *ch == '\\' {
                escaped = true;
                continue;
            }
            if *ch == '"' {
                in_string = false;
            }
            continue;
        }
        if *ch == '"' {
            in_string = true;
            continue;
        }
        if *ch == '#' {
            break;
        }
        if *ch == '\t' {
            return Some(i as u32 + 1);
        }
    }
    None
}

fn indent_width(chars: &[char], line: &Line) -> u32 {
    let text = &chars[line.start..line.start + line.len];
    text.iter().take_while(|c| **c == ' ').count() as u32
}

fn line_end_col(_chars: &[char], line: &Line) -> u32 {
    line.len as u32 + 1
}

fn span_at(file: u16, line: u32, column: u32) -> Span {
    Span { file, line, column }
}

fn indent_error(file: &str, line: u32, column: u32) -> Diagnostic {
    diag(
        "E_INDENT",
        file,
        Span {
            file: 0,
            line,
            column,
        },
        "四空格缩进",
        "非法缩进",
    )
}

fn lex_error(file: &str, line: u32, column: u32, actual: &str) -> Diagnostic {
    diag(
        "E_LEX",
        file,
        Span {
            file: 0,
            line,
            column,
        },
        "合法 token",
        actual,
    )
}

fn number_error(
    code: &'static str,
    file: &str,
    line: u32,
    column: u32,
    actual: &str,
) -> Diagnostic {
    diag(
        code,
        file,
        Span {
            file: 0,
            line,
            column,
        },
        "规范化单位字面量",
        actual,
    )
}

fn tokenize_line(
    file_index: u16,
    file_name: &str,
    chars: &[char],
    line: &Line,
) -> Result<(Vec<Token>, bool), Diagnostic> {
    let text = &chars[line.start..line.start + line.len];
    let mut i = indent_width(chars, line) as usize;
    let mut tokens = Vec::new();
    while i < text.len() {
        if text[i] == ' ' {
            i += 1;
            continue;
        }
        if text[i] == '#' {
            break;
        }
        let col = i as u32 + 1;
        let (kind, next) = read_token(file_name, line.no, text, i)?;
        tokens.push(Token {
            kind,
            span: span_at(file_index, line.no, col),
        });
        i = next;
    }
    let ended = tokens.last().is_some_and(|t| t.kind == TokenKind::Colon);
    Ok((tokens, ended))
}

fn classify_word(file: &str, line: u32, col: u32, word: &str) -> Result<TokenKind, Diagnostic> {
    if let Some(kind) = keyword(word) {
        return Ok(TokenKind::Keyword(kind));
    }
    if let Some(kind) = type_name(word) {
        return Ok(TokenKind::Type(kind));
    }
    if word == "true" {
        return Ok(TokenKind::Bool(true));
    }
    if word == "false" {
        return Ok(TokenKind::Bool(false));
    }
    if word
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && word.starts_with(|c: char| c.is_ascii_lowercase())
    {
        return Ok(TokenKind::Ident(word.to_string()));
    }
    Err(lex_error(file, line, col, word))
}

fn keyword(word: &str) -> Option<Keyword> {
    Some(match word {
        "and" => Keyword::And,
        "banner" => Keyword::Banner,
        "bind" => Keyword::Bind,
        "choose" => Keyword::Choose,
        "emit" => Keyword::Emit,
        "entry" => Keyword::Entry,
        "floor" => Keyword::Floor,
        "from" => Keyword::From,
        "grade" => Keyword::Grade,
        "guarantee" => Keyword::Guarantee,
        "in" => Keyword::In,
        "item" => Keyword::Item,
        "linear" => Keyword::Linear,
        "not" => Keyword::Not,
        "or" => Keyword::Or,
        "pity_key" => Keyword::PityKey,
        "progress_key" => Keyword::ProgressKey,
        "promote" => Keyword::Promote,
        "rarity" => Keyword::Rarity,
        "reset" => Keyword::Reset,
        "rest" => Keyword::Rest,
        "result" => Keyword::Result,
        "roster" => Keyword::Roster,
        "rule" => Keyword::Rule,
        "rescale" => Keyword::Rescale,
        "source_roster" => Keyword::SourceRoster,
        "source_rule" => Keyword::SourceRule,
        "update" => Keyword::Update,
        "weighted" => Keyword::Weighted,
        _ => return None,
    })
}

fn type_name(word: &str) -> Option<TypeName> {
    Some(match word {
        "Action" => TypeName::Action,
        "Bool" => TypeName::Bool,
        "Chance" => TypeName::Chance,
        "Count" => TypeName::Count,
        "Counter" => TypeName::Counter,
        "Distribution" => TypeName::Distribution,
        "Event" => TypeName::Event,
        "Item" => TypeName::Item,
        "Pool" => TypeName::Pool,
        "Rarity" => TypeName::Rarity,
        "Selector" => TypeName::Selector,
        _ => return None,
    })
}

fn scan_scope(
    file: &str,
    line: u32,
    text: &[char],
    start: usize,
) -> Result<(Scope, usize), Diagnostic> {
    let rest: String = text[start + 1..]
        .iter()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let scope = match rest.as_str() {
        "pity" => Scope::Pity,
        "progress" => Scope::Progress,
        "banner" => Scope::Banner,
        "action" => Scope::Action,
        _ => return Err(lex_error(file, line, start as u32 + 1, &format!("@{rest}"))),
    };
    Ok((scope, start + 1 + rest.len()))
}

fn scan_number(
    file: &str,
    line: u32,
    text: &[char],
    start: usize,
) -> Result<(TokenKind, usize), Diagnostic> {
    let col = start as u32 + 1;
    let (num, mut i) = scan_uint(file, line, text, start, col)?;
    let (numer, denom) = if text.get(i) == Some(&'/') {
        let den_start = i + 1;
        if den_start >= text.len() || !text[den_start].is_ascii_digit() {
            return Err(number_error(
                "E_NONCANONICAL_NUMBER",
                file,
                line,
                col,
                "分数",
            ));
        }
        let (den, next) = scan_uint(file, line, text, den_start, col)?;
        i = next;
        if den < 2 || num == 0 || gcd(num, den) != 1 {
            return Err(number_error(
                "E_NONCANONICAL_NUMBER",
                file,
                line,
                col,
                &text[start..i].iter().collect::<String>(),
            ));
        }
        (num, den)
    } else {
        (num, 1u128)
    };
    let suffix = text.get(i).copied();
    let fractional = denom != 1;
    let kind = match suffix {
        Some('p') => {
            if numer > denom {
                return Err(number_error("E_LITERAL_RANGE", file, line, col, "Chance"));
            }
            TokenKind::P(Rational::new(numer, denom))
        }
        Some('w') => {
            if numer == 0 {
                return Err(number_error("E_LITERAL_RANGE", file, line, col, "Weight"));
            }
            TokenKind::W(Rational::new(numer, denom))
        }
        Some('c') if !fractional => TokenKind::C(numer),
        Some('t') if !fractional => TokenKind::T(numer),
        Some('r') if !fractional => {
            if numer == 0 {
                return Err(number_error("E_LITERAL_RANGE", file, line, col, "Rarity"));
            }
            TokenKind::R(numer)
        }
        _ => {
            return Err(lex_error(
                file,
                line,
                col,
                &text[start..i.min(text.len())].iter().collect::<String>(),
            ));
        }
    };
    Ok((kind, i + 1))
}

fn scan_uint(
    file: &str,
    line: u32,
    text: &[char],
    start: usize,
    literal_col: u32,
) -> Result<(u128, usize), Diagnostic> {
    if text[start] == '0' {
        if text.get(start + 1).is_some_and(|c| c.is_ascii_digit()) {
            return Err(number_error(
                "E_NONCANONICAL_NUMBER",
                file,
                line,
                literal_col,
                "前导零",
            ));
        }
        return Ok((0, start + 1));
    }
    let mut value: u128 = 0;
    let mut i = start;
    while i < text.len() && text[i].is_ascii_digit() {
        let digit = text[i] as u128 - '0' as u128;
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
            .ok_or_else(|| lex_error(file, line, literal_col, "整数过大"))?;
        i += 1;
    }
    if i == start {
        return Err(lex_error(file, line, literal_col, "数字"));
    }
    Ok((value, i))
}

fn scan_string(
    file: &str,
    line: u32,
    text: &[char],
    start: usize,
) -> Result<(String, usize), Diagnostic> {
    let mut i = start + 1;
    let mut out = String::new();
    while i < text.len() {
        let ch = text[i];
        if ch == '"' {
            return Ok((out, i + 1));
        }
        if ch == '\\' {
            let esc = text
                .get(i + 1)
                .copied()
                .ok_or_else(|| lex_error(file, line, i as u32 + 1, "转义"))?;
            match esc {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    if text.get(i + 2) != Some(&'{') {
                        return Err(lex_error(file, line, i as u32 + 1, "\\u"));
                    }
                    let hex_start = i + 3;
                    let hex_end = text[hex_start..]
                        .iter()
                        .position(|c| *c == '}')
                        .map(|p| hex_start + p)
                        .ok_or_else(|| lex_error(file, line, i as u32 + 1, "\\u"))?;
                    let hex: String = text[hex_start..hex_end].iter().collect();
                    if !(1..=6).contains(&hex.len()) || !hex.chars().all(|c| c.is_ascii_hexdigit())
                    {
                        return Err(lex_error(file, line, i as u32 + 1, "\\u"));
                    }
                    let code = u32::from_str_radix(&hex, 16)
                        .map_err(|_| lex_error(file, line, i as u32 + 1, "\\u"))?;
                    let ch = char::from_u32(code)
                        .ok_or_else(|| lex_error(file, line, i as u32 + 1, "\\u"))?;
                    out.push(ch);
                    i = hex_end + 1;
                    continue;
                }
                _ => return Err(lex_error(file, line, i as u32 + 1, "转义")),
            }
            i += 2;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    Err(lex_error(file, line, start as u32 + 1, "字符串"))
}

fn read_token(
    file_name: &str,
    line_no: u32,
    text: &[char],
    start: usize,
) -> Result<(TokenKind, usize), Diagnostic> {
    let ch = text[start];
    let col = start as u32 + 1;
    if ch == '"' {
        let (value, next) = scan_string(file_name, line_no, text, start)?;
        return Ok((TokenKind::String(value), next));
    }
    if ch.is_ascii_digit() {
        return scan_number(file_name, line_no, text, start);
    }
    if ch == '@' {
        let (scope, next) = scan_scope(file_name, line_no, text, start)?;
        return Ok((TokenKind::Scope(scope), next));
    }
    if ch.is_ascii_alphabetic() {
        let mut i = start + 1;
        while i < text.len() && (text[i].is_ascii_alphanumeric() || text[i] == '_') {
            i += 1;
        }
        let word: String = text[start..i].iter().collect();
        let kind = classify_word(file_name, line_no, col, &word)?;
        return Ok((kind, i));
    }
    match ch {
        ':' => Ok((TokenKind::Colon, start + 1)),
        ',' => Ok((TokenKind::Comma, start + 1)),
        '[' => Ok((TokenKind::LBrack, start + 1)),
        ']' => Ok((TokenKind::RBrack, start + 1)),
        '(' => Ok((TokenKind::LParen, start + 1)),
        ')' => Ok((TokenKind::RParen, start + 1)),
        '%' => Ok((TokenKind::Percent, start + 1)),
        '+' => {
            if text.get(start + 1) == Some(&'=') {
                Ok((TokenKind::PlusEq, start + 2))
            } else {
                Err(lex_error(file_name, line_no, col, "+"))
            }
        }
        '-' => {
            if text.get(start + 1) == Some(&'>') {
                Ok((TokenKind::Arrow, start + 2))
            } else {
                Err(lex_error(file_name, line_no, col, "-"))
            }
        }
        '=' => {
            if text.get(start + 1) == Some(&'=') {
                Ok((TokenKind::EqEq, start + 2))
            } else {
                Ok((TokenKind::Eq, start + 1))
            }
        }
        '>' => {
            if text.get(start + 1) == Some(&'=') {
                Ok((TokenKind::GtEq, start + 2))
            } else {
                Err(lex_error(file_name, line_no, col, ">"))
            }
        }
        '.' => {
            if text.get(start + 1) == Some(&'.') {
                Ok((TokenKind::DotDot, start + 2))
            } else {
                Ok((TokenKind::Dot, start + 1))
            }
        }
        _ => Err(lex_error(file_name, line_no, col, &ch.to_string())),
    }
}

pub(crate) fn flatten(file_name: &str, source: &str) -> Result<Vec<Flat>, Diagnostic> {
    let chars = decode(file_name, source)?;
    let lines = split_lines(file_name, &chars)?;
    let mut out = Vec::new();
    for line in &lines {
        let text = &chars[line.start..line.start + line.len];
        let mut i = 0;
        while i < text.len() && (text[i] == ' ' || text[i] == '\t') {
            i += 1;
        }
        if i >= text.len() {
            continue;
        }
        if text[i] == '#' {
            out.push(Flat::Comment(comment_text(text, i)));
            continue;
        }
        let mut tokens = Vec::new();
        let mut trailing = None;
        while i < text.len() {
            while i < text.len() && (text[i] == ' ' || text[i] == '\t') {
                i += 1;
            }
            if i >= text.len() {
                break;
            }
            if text[i] == '#' {
                trailing = Some(comment_text(text, i));
                break;
            }
            let column = i as u32 + 1;
            let (kind, next) = read_token(file_name, line.no, text, i)?;
            tokens.push(RawToken {
                kind,
                text: text[i..next].iter().collect(),
                line: line.no,
                column,
                trailing: None,
            });
            i = next;
        }
        if let Some(comment) = trailing {
            if let Some(last) = tokens.last_mut() {
                last.trailing = Some(comment);
            } else {
                out.push(Flat::Comment(comment));
            }
        }
        out.extend(tokens.into_iter().map(Flat::Token));
    }
    Ok(out)
}

fn comment_text(text: &[char], start: usize) -> String {
    let mut end = text.len();
    while end > start && (text[end - 1] == ' ' || text[end - 1] == '\t') {
        end -= 1;
    }
    text[start..end].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_inventory_matches_spec() {
        let mut listed = KEYWORDS.to_vec();
        listed.sort_unstable();
        let mut found = Vec::new();
        for word in KEYWORDS {
            assert!(keyword(word).is_some(), "{word}");
            found.push(*word);
        }
        found.sort_unstable();
        assert_eq!(found, listed);
        assert_eq!(KEYWORDS.len(), 29);
        assert_eq!(TYPES.len(), 11);
    }
}
