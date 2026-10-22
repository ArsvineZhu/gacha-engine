use std::path::Path;

use crate::diagnostic::{Diagnostic, Span, diag};
use crate::lex::{Flat, Keyword, RawToken, TokenKind, TypeName, flatten, tokenize};
use crate::parse;

pub fn format_source(name: &str, source: &str) -> Result<String, Diagnostic> {
    let nodes = flatten(name, source)?;
    let mut printer = Printer {
        file_name: name,
        nodes,
        index: 0,
        indent: 0,
        out: String::new(),
        at_bol: true,
        prev: None,
        line_comments: Vec::new(),
        eof_line: 1,
        eof_column: 1,
    };
    printer.format()?;
    let formatted = printer.finish();
    verify(name, &formatted)?;
    Ok(formatted)
}

fn verify(name: &str, formatted: &str) -> Result<(), Diagnostic> {
    let ext = Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    let tokens = tokenize(0, name, formatted)?;
    parse::parse(name, &tokens, ext)?;
    Ok(())
}

struct Printer<'a> {
    file_name: &'a str,
    nodes: Vec<Flat>,
    index: usize,
    indent: usize,
    out: String,
    at_bol: bool,
    prev: Option<TokenKind>,
    line_comments: Vec<String>,
    eof_line: u32,
    eof_column: u32,
}

impl<'a> Printer<'a> {
    fn format(&mut self) -> Result<(), Diagnostic> {
        let ext = Path::new(self.file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        match ext {
            "gmr" => self.print_rule()?,
            "gml" => self.print_roster()?,
            "gmb" => self.print_banners()?,
            _ => return Err(self.fail_here("E_PARSE", "gmr/gml/gmb")),
        }
        self.expect_eof()?;
        Ok(())
    }

    fn finish(&mut self) -> String {
        self.flush_leading();
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        std::mem::take(&mut self.out)
    }

    fn print_rule(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Rule, "rule")?;
        self.expect_ident()?;
        let colon = self.expect_colon()?;
        self.finish_line();
        self.indent += 1;
        let mut any = false;
        while self.peek_ident() {
            self.print_decl()?;
            any = true;
        }
        while self.peek_kw(Keyword::Guarantee) {
            self.print_guarantee()?;
            any = true;
        }
        while self.peek_kw(Keyword::Update) {
            self.print_update()?;
            any = true;
        }
        if !any {
            return Err(self.fail_token("E_EMPTY_BLOCK", &colon, "缩进块", "空块"));
        }
        self.close_block();
        Ok(())
    }

    fn print_roster(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Roster, "roster")?;
        self.expect_ident()?;
        self.expect_colon()?;
        self.finish_line();
        self.indent += 1;
        if !self.peek_kw(Keyword::Entry) {
            return Err(self.fail_here("E_PARSE", "entry"));
        }
        while self.peek_kw(Keyword::Entry) {
            self.print_entry()?;
        }
        self.close_block();
        Ok(())
    }

    fn print_banners(&mut self) -> Result<(), Diagnostic> {
        if !self.peek_kw(Keyword::Banner) {
            return Err(self.fail_here("E_PARSE", "banner"));
        }
        let mut first = true;
        while self.peek_kw(Keyword::Banner) {
            if !first {
                self.blank_line();
            }
            first = false;
            self.print_banner()?;
        }
        Ok(())
    }

    fn print_banner(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Banner, "banner")?;
        self.expect_ident()?;
        self.expect_colon()?;
        self.finish_line();
        self.indent += 1;
        self.start_line();
        self.expect_kw(Keyword::SourceRule, "source_rule")?;
        self.expect_ident()?;
        self.finish_line();
        self.start_line();
        self.expect_kw(Keyword::SourceRoster, "source_roster")?;
        self.expect_ident()?;
        self.finish_line();
        if self.peek_kw(Keyword::PityKey) {
            self.print_key(Keyword::PityKey, "pity_key")?;
        }
        if self.peek_kw(Keyword::ProgressKey) {
            self.print_key(Keyword::ProgressKey, "progress_key")?;
        }
        while self.peek_kw(Keyword::Bind) {
            self.print_bind()?;
        }
        self.close_block();
        Ok(())
    }

    fn print_key(&mut self, kw: Keyword, name: &str) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(kw, name)?;
        self.expect_string()?;
        self.finish_line();
        Ok(())
    }

    fn print_bind(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Bind, "bind")?;
        self.expect_ident()?;
        self.expect_colon()?;
        self.print_pool_or_item()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
        if self.peek_kind(|kind| matches!(kind, TokenKind::LBrack)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
            if self.peek_ident() {
                self.expect_ident()?;
                while self.peek_kind(|kind| matches!(kind, TokenKind::Comma)) {
                    self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
                    self.expect_ident()?;
                }
            }
            self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
        } else {
            self.expect_ident()?;
        }
        self.finish_line();
        Ok(())
    }

    fn print_entry(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Entry, "entry")?;
        self.expect_ident()?;
        self.expect_colon()?;
        self.expect_r()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
        self.expect_string()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
        self.print_strings()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
        self.expect_bool()?;
        self.finish_line();
        Ok(())
    }

    fn print_decl(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_ident()?;
        self.expect_colon()?;
        let token = self.take_token()?;
        match token.kind {
            TokenKind::Type(TypeName::Counter) => {
                self.emit(&token);
                self.print_counter_tail()?;
                self.finish_line();
            }
            TokenKind::Type(TypeName::Bool) => {
                self.emit(&token);
                self.expect_scope()?;
                self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
                self.expect_bool()?;
                self.finish_line();
            }
            TokenKind::Type(TypeName::Pool) | TokenKind::Type(TypeName::Item) => {
                self.emit(&token);
                self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
                self.expect_r()?;
                self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
                self.finish_line();
            }
            TokenKind::Type(TypeName::Event) => {
                self.emit(&token);
                self.print_params()?;
                self.finish_line();
            }
            TokenKind::Type(TypeName::Chance) => {
                self.emit(&token);
                self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
                self.print_chance()?;
                self.finish_line();
            }
            TokenKind::Type(TypeName::Selector) => {
                self.emit(&token);
                self.print_selector_tail()?;
            }
            TokenKind::Type(TypeName::Distribution) => {
                self.emit(&token);
                let colon = self.expect_colon()?;
                self.finish_line();
                self.print_rows(
                    &colon,
                    "E_EMPTY_BLOCK",
                    "分布行",
                    |kind| matches!(kind, TokenKind::R(_)),
                    |this| this.print_dist_row(),
                )?;
            }
            TokenKind::Type(TypeName::Action) => {
                self.emit(&token);
                self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
                self.expect_t()?;
                self.expect_kind(|kind| matches!(kind, TokenKind::DotDot), "..")?;
                self.expect_t()?;
                self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
                self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
                self.expect_ident()?;
                self.finish_line();
            }
            _ => {
                let actual = describe(&token.kind);
                return Err(self.fail_token("E_PARSE", &token, "声明类型", &actual));
            }
        }
        Ok(())
    }

    fn print_counter_tail(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
        if self.peek_kind(|kind| matches!(kind, TokenKind::C(_))) {
            self.expect_c()?;
        }
        self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
        self.expect_scope()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
        self.expect_c()?;
        Ok(())
    }

    fn print_params(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
        if !self.peek_kind(|kind| matches!(kind, TokenKind::RBrack)) {
            self.expect_param()?;
            while self.peek_kind(|kind| matches!(kind, TokenKind::Comma)) {
                self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
                self.expect_param()?;
            }
        }
        self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
        Ok(())
    }

    fn expect_param(&mut self) -> Result<(), Diagnostic> {
        self.expect(
            |kind| {
                matches!(
                    kind,
                    TokenKind::Type(
                        TypeName::Count | TypeName::Bool | TypeName::Rarity | TypeName::Item
                    )
                )
            },
            "事件参数类型",
        )?;
        Ok(())
    }

    fn print_selector_tail(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
        self.expect_r()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
        if self.peek_kw(Keyword::Weighted) {
            self.expect_kw(Keyword::Weighted, "weighted")?;
            let colon = self.expect_colon()?;
            self.finish_line();
            self.print_rows(
                &colon,
                "E_WEIGHT_EMPTY",
                "权重分支",
                |kind| matches!(kind, TokenKind::W(_)),
                |this| this.print_weight_row(),
            )?;
        } else {
            self.print_selector_inline()?;
            self.finish_line();
        }
        Ok(())
    }

    fn print_rows(
        &mut self,
        colon: &RawToken,
        code: &'static str,
        expected: &str,
        more: fn(&TokenKind) -> bool,
        row: fn(&mut Self) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        self.indent += 1;
        let mut count = 0;
        while self.peek_kind(more) {
            row(self)?;
            count += 1;
        }
        if count == 0 {
            return Err(self.fail_token(code, colon, expected, "空"));
        }
        self.close_block();
        Ok(())
    }

    fn print_weight_row(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect(|kind| matches!(kind, TokenKind::W(_)), "权重")?;
        self.expect_colon()?;
        self.print_selector_inline()?;
        self.finish_line();
        Ok(())
    }

    fn print_dist_row(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_r()?;
        self.expect_colon()?;
        self.print_amount()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Arrow), "->")?;
        self.expect_ident()?;
        self.finish_line();
        Ok(())
    }

    fn print_amount(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kw(Keyword::Rest) {
            self.expect_kw(Keyword::Rest, "rest")?;
            return Ok(());
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::W(_))) {
            self.expect(|kind| matches!(kind, TokenKind::W(_)), "权重")?;
            return Ok(());
        }
        self.print_chance()
    }

    fn print_chance(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kind(|kind| matches!(kind, TokenKind::P(_))) {
            self.expect(|kind| matches!(kind, TokenKind::P(_)), "概率")?;
            return Ok(());
        }
        if self.peek_ident() {
            self.expect_ident()?;
            return Ok(());
        }
        if self.peek_kw(Keyword::Linear) {
            self.expect_kw(Keyword::Linear, "linear")?;
            self.expect_kind(|kind| matches!(kind, TokenKind::LParen), "(")?;
            self.expect_ident()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.expect_c()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.expect_p()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.expect_p()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.expect_p()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::RParen), ")")?;
            return Ok(());
        }
        Err(self.fail_here("E_PARSE", "Chance"))
    }

    fn print_selector_inline(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kw(Keyword::From) {
            self.expect_kw(Keyword::From, "from")?;
            self.expect_ident()?;
            return Ok(());
        }
        if self.peek_kw(Keyword::Choose) {
            self.expect_kw(Keyword::Choose, "choose")?;
            self.expect_kind(|kind| matches!(kind, TokenKind::LParen), "(")?;
            self.print_cond()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.print_selector_inline()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
            self.print_selector_inline()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::RParen), ")")?;
            return Ok(());
        }
        if self.peek_ident() {
            self.expect_ident()?;
            return Ok(());
        }
        Err(self.fail_here("E_PARSE", "选择器"))
    }

    fn print_guarantee(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Guarantee, "guarantee")?;
        self.expect_ident()?;
        self.expect_colon()?;
        self.print_cond()?;
        self.expect_kind(|kind| matches!(kind, TokenKind::Arrow), "->")?;
        self.print_effect()?;
        self.finish_line();
        Ok(())
    }

    fn print_effect(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kw(Keyword::Item) {
            self.expect_kw(Keyword::Item, "item")?;
            self.expect_ident()?;
            return Ok(());
        }
        if self.peek_kw(Keyword::Rarity) {
            self.expect_kw(Keyword::Rarity, "rarity")?;
            self.expect_r()?;
            return Ok(());
        }
        self.expect_kw(Keyword::Floor, "floor")?;
        self.expect_r()?;
        if self.peek_kw(Keyword::Rescale) {
            self.expect_kw(Keyword::Rescale, "rescale")?;
        } else if self.peek_kw(Keyword::Promote) {
            self.expect_kw(Keyword::Promote, "promote")?;
        } else {
            return Err(self.fail_here("E_PARSE", "rescale 或 promote"));
        }
        Ok(())
    }

    fn print_update(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        self.expect_kw(Keyword::Update, "update")?;
        self.print_cond()?;
        let colon = self.expect_colon()?;
        self.finish_line();
        self.indent += 1;
        let mut count = 0;
        while self.starts_mutation() {
            self.print_op()?;
            count += 1;
        }
        if count == 0 {
            return Err(self.fail_token("E_EMPTY_BLOCK", &colon, "更新", "空"));
        }
        self.close_block();
        Ok(())
    }

    fn print_op(&mut self) -> Result<(), Diagnostic> {
        self.start_line();
        if self.peek_kw(Keyword::Reset) {
            self.expect_kw(Keyword::Reset, "reset")?;
            self.expect_ident()?;
            self.finish_line();
            return Ok(());
        }
        if self.peek_kw(Keyword::Emit) {
            self.expect_kw(Keyword::Emit, "emit")?;
            self.expect_ident()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::LParen), "(")?;
            if !self.peek_kind(|kind| matches!(kind, TokenKind::RParen)) {
                self.print_arg()?;
                while self.peek_kind(|kind| matches!(kind, TokenKind::Comma)) {
                    self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
                    self.print_arg()?;
                }
            }
            self.expect_kind(|kind| matches!(kind, TokenKind::RParen), ")")?;
            self.finish_line();
            return Ok(());
        }
        self.expect_ident()?;
        if self.peek_kind(|kind| matches!(kind, TokenKind::Eq)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::Eq), "=")?;
            self.expect_bool()?;
        } else if self.peek_kind(|kind| matches!(kind, TokenKind::PlusEq)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::PlusEq), "+=")?;
            self.expect_c()?;
        } else {
            return Err(self.fail_here("E_PARSE", "= 或 +="));
        }
        self.finish_line();
        Ok(())
    }

    fn print_arg(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kind(|kind| matches!(kind, TokenKind::C(_))) {
            self.expect_c()?;
            return Ok(());
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::R(_))) {
            self.expect_r()?;
            return Ok(());
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::Bool(_))) {
            self.expect_bool()?;
            return Ok(());
        }
        if self.peek_kw(Keyword::Result) {
            self.expect_kw(Keyword::Result, "result")?;
            if self.peek_kind(|kind| matches!(kind, TokenKind::Dot)) {
                self.expect_kind(|kind| matches!(kind, TokenKind::Dot), ".")?;
                self.expect_kw(Keyword::Grade, "grade")?;
            }
            return Ok(());
        }
        if self.peek_ident() {
            self.expect_ident()?;
            return Ok(());
        }
        Err(self.fail_here("E_PARSE", "事件参数"))
    }

    fn print_cond(&mut self) -> Result<(), Diagnostic> {
        self.print_or()
    }

    fn print_or(&mut self) -> Result<(), Diagnostic> {
        self.print_and()?;
        while self.peek_kw(Keyword::Or) {
            self.expect_kw(Keyword::Or, "or")?;
            self.print_and()?;
        }
        Ok(())
    }

    fn print_and(&mut self) -> Result<(), Diagnostic> {
        self.print_not()?;
        while self.peek_kw(Keyword::And) {
            self.expect_kw(Keyword::And, "and")?;
            self.print_not()?;
        }
        Ok(())
    }

    fn print_not(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kw(Keyword::Not) {
            self.expect_kw(Keyword::Not, "not")?;
            return self.print_not();
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::LParen)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::LParen), "(")?;
            self.print_or()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::RParen), ")")?;
            return Ok(());
        }
        self.print_predicate()
    }

    fn print_predicate(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kind(|kind| matches!(kind, TokenKind::Bool(_))) {
            self.expect_bool()?;
            return Ok(());
        }
        if self.peek_kw(Keyword::Result) {
            self.expect_kw(Keyword::Result, "result")?;
            if self.peek_kind(|kind| matches!(kind, TokenKind::Dot)) {
                self.expect_kind(|kind| matches!(kind, TokenKind::Dot), ".")?;
                self.expect_kw(Keyword::Grade, "grade")?;
                if self.peek_kind(|kind| matches!(kind, TokenKind::EqEq)) {
                    self.expect_kind(|kind| matches!(kind, TokenKind::EqEq), "==")?;
                } else if self.peek_kind(|kind| matches!(kind, TokenKind::GtEq)) {
                    self.expect_kind(|kind| matches!(kind, TokenKind::GtEq), ">=")?;
                } else {
                    return Err(self.fail_here("E_PARSE", "== 或 >="));
                }
                self.expect_r()?;
                return Ok(());
            }
            if self.peek_kw(Keyword::In) {
                self.expect_kw(Keyword::In, "in")?;
                self.expect_ident()?;
                return Ok(());
            }
            return Err(self.fail_here("E_PARSE", "result 条件"));
        }
        self.expect_ident()?;
        if self.peek_kind(|kind| matches!(kind, TokenKind::EqEq)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::EqEq), "==")?;
            if self.peek_kind(|kind| matches!(kind, TokenKind::C(_))) {
                self.expect_c()?;
            } else if self.peek_kind(|kind| matches!(kind, TokenKind::Bool(_))) {
                self.expect_bool()?;
            } else {
                return Err(self.fail_here("E_PARSE", "计数或布尔"));
            }
            return Ok(());
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::GtEq)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::GtEq), ">=")?;
            self.expect_c()?;
            return Ok(());
        }
        if self.peek_kind(|kind| matches!(kind, TokenKind::Percent)) {
            self.expect_kind(|kind| matches!(kind, TokenKind::Percent), "%")?;
            self.expect_c()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::EqEq), "==")?;
            self.expect_c()?;
            return Ok(());
        }
        Err(self.fail_here("E_PARSE", "比较"))
    }

    fn print_pool_or_item(&mut self) -> Result<(), Diagnostic> {
        if self.peek_type(TypeName::Pool) || self.peek_type(TypeName::Item) {
            let token = self.take_token()?;
            self.emit(&token);
            self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
            self.expect_r()?;
            self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
            return Ok(());
        }
        Err(self.fail_here("E_PARSE", "Pool 或 Item"))
    }

    fn print_strings(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(|kind| matches!(kind, TokenKind::LBrack), "[")?;
        if !self.peek_kind(|kind| matches!(kind, TokenKind::RBrack)) {
            self.expect_string()?;
            while self.peek_kind(|kind| matches!(kind, TokenKind::Comma)) {
                self.expect_kind(|kind| matches!(kind, TokenKind::Comma), ",")?;
                self.expect_string()?;
            }
        }
        self.expect_kind(|kind| matches!(kind, TokenKind::RBrack), "]")?;
        Ok(())
    }

    fn starts_mutation(&self) -> bool {
        match self.peek_token().map(|token| &token.kind) {
            Some(TokenKind::Ident(_)) => true,
            Some(TokenKind::Keyword(Keyword::Reset | Keyword::Emit)) => true,
            _ => false,
        }
    }

    fn expect_eof(&mut self) -> Result<(), Diagnostic> {
        if self.peek_token().is_some() {
            return Err(self.fail_here("E_PARSE", "文件结束"));
        }
        Ok(())
    }

    fn close_block(&mut self) {
        if self.peek_token().is_none() {
            self.flush_leading();
        }
        self.indent = self.indent.saturating_sub(1);
    }

    fn start_line(&mut self) {
        self.flush_leading();
        self.out.push_str(&"    ".repeat(self.indent));
        self.at_bol = true;
        self.prev = None;
    }

    fn finish_line(&mut self) {
        for comment in self.line_comments.drain(..) {
            self.out.push(' ');
            self.out.push_str(&comment);
        }
        self.out.push('\n');
        self.at_bol = true;
        self.prev = None;
    }

    fn blank_line(&mut self) {
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out.push('\n');
    }

    fn flush_leading(&mut self) {
        while self.index < self.nodes.len() {
            let Flat::Comment(text) = &self.nodes[self.index] else {
                break;
            };
            let text = text.clone();
            self.index += 1;
            if !self.out.is_empty() && !self.out.ends_with('\n') {
                self.out.push('\n');
            }
            self.out.push_str(&"    ".repeat(self.indent));
            self.out.push_str(&text);
            self.out.push('\n');
        }
    }

    fn emit(&mut self, token: &RawToken) {
        if !self.at_bol
            && let Some(prev) = &self.prev
            && needs_space(prev, &token.kind)
        {
            self.out.push(' ');
        }
        self.out.push_str(&token.text);
        self.prev = Some(token.kind.clone());
        self.at_bol = false;
    }

    fn take_token(&mut self) -> Result<RawToken, Diagnostic> {
        loop {
            if self.index >= self.nodes.len() {
                return Err(self.fail_eof("token"));
            }
            match &self.nodes[self.index] {
                Flat::Comment(text) => {
                    self.line_comments.push(text.clone());
                    self.index += 1;
                }
                Flat::Token(token) => {
                    let mut token = token.clone();
                    self.index += 1;
                    if let Some(comment) = token.trailing.take() {
                        self.line_comments.push(comment);
                    }
                    self.eof_line = token.line;
                    self.eof_column = token.column;
                    return Ok(token);
                }
            }
        }
    }

    fn expect(
        &mut self,
        ok: impl Fn(&TokenKind) -> bool,
        expected: &str,
    ) -> Result<RawToken, Diagnostic> {
        let token = self.take_token()?;
        if ok(&token.kind) {
            self.emit(&token);
            Ok(token)
        } else {
            let actual = describe(&token.kind);
            Err(self.fail_token("E_PARSE", &token, expected, &actual))
        }
    }

    fn expect_kind(
        &mut self,
        ok: impl Fn(&TokenKind) -> bool,
        expected: &str,
    ) -> Result<(), Diagnostic> {
        self.expect(ok, expected)?;
        Ok(())
    }

    fn expect_kw(&mut self, kw: Keyword, expected: &str) -> Result<(), Diagnostic> {
        self.expect(|kind| is_kw(kind, kw), expected)?;
        Ok(())
    }

    fn expect_ident(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::Ident(_)), "标识符")?;
        Ok(())
    }

    fn expect_colon(&mut self) -> Result<RawToken, Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::Colon), ":")
    }

    fn expect_string(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::String(_)), "字符串")?;
        Ok(())
    }

    fn expect_bool(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::Bool(_)), "布尔值")?;
        Ok(())
    }

    fn expect_scope(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::Scope(_)), "作用域")?;
        Ok(())
    }

    fn expect_c(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::C(_)), "计数")?;
        Ok(())
    }

    fn expect_t(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::T(_)), "次数")?;
        Ok(())
    }

    fn expect_r(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::R(_)), "稀有度")?;
        Ok(())
    }

    fn expect_p(&mut self) -> Result<(), Diagnostic> {
        self.expect(|kind| matches!(kind, TokenKind::P(_)), "概率")?;
        Ok(())
    }

    fn peek_token(&self) -> Option<&RawToken> {
        self.nodes[self.index..].iter().find_map(|node| match node {
            Flat::Token(token) => Some(token),
            Flat::Comment(_) => None,
        })
    }

    fn peek_kind(&self, ok: impl Fn(&TokenKind) -> bool) -> bool {
        self.peek_token().is_some_and(|token| ok(&token.kind))
    }

    fn peek_kw(&self, kw: Keyword) -> bool {
        self.peek_kind(|kind| is_kw(kind, kw))
    }

    fn peek_ident(&self) -> bool {
        self.peek_kind(|kind| matches!(kind, TokenKind::Ident(_)))
    }

    fn peek_type(&self, ty: TypeName) -> bool {
        self.peek_kind(|kind| matches!(kind, TokenKind::Type(found) if *found == ty))
    }

    fn fail_token(
        &self,
        code: &'static str,
        token: &RawToken,
        expected: &str,
        actual: &str,
    ) -> Diagnostic {
        diag(
            code,
            self.file_name,
            Span {
                file: 0,
                line: token.line,
                column: token.column,
            },
            expected,
            actual,
        )
    }

    fn fail_here(&self, code: &'static str, expected: &str) -> Diagnostic {
        match self.peek_token() {
            Some(token) => {
                let actual = describe(&token.kind);
                self.fail_token(code, token, expected, &actual)
            }
            None => self.fail_eof(expected),
        }
    }

    fn fail_eof(&self, expected: &str) -> Diagnostic {
        diag(
            "E_PARSE",
            self.file_name,
            Span {
                file: 0,
                line: self.eof_line,
                column: self.eof_column,
            },
            expected,
            "文件结束",
        )
    }
}

fn is_kw(kind: &TokenKind, kw: Keyword) -> bool {
    matches!(kind, TokenKind::Keyword(found) if *found == kw)
}

fn needs_space(prev: &TokenKind, next: &TokenKind) -> bool {
    if matches!(
        next,
        TokenKind::RParen
            | TokenKind::RBrack
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::DotDot
    ) {
        return false;
    }
    if matches!(
        prev,
        TokenKind::LParen | TokenKind::LBrack | TokenKind::Dot | TokenKind::DotDot
    ) {
        return false;
    }
    if matches!(
        prev,
        TokenKind::Colon
            | TokenKind::Comma
            | TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::Arrow
            | TokenKind::EqEq
            | TokenKind::GtEq
            | TokenKind::Percent
    ) {
        return true;
    }
    match next {
        TokenKind::Eq
        | TokenKind::PlusEq
        | TokenKind::Arrow
        | TokenKind::EqEq
        | TokenKind::GtEq
        | TokenKind::Percent => true,
        TokenKind::LParen => !matches!(
            prev,
            TokenKind::Keyword(Keyword::Choose | Keyword::Linear) | TokenKind::Ident(_)
        ),
        TokenKind::LBrack => false,
        _ => true,
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(name) => name.clone(),
        TokenKind::Keyword(kw) => keyword_text(*kw).to_string(),
        TokenKind::Type(ty) => type_text(*ty).to_string(),
        TokenKind::String(_) => "字符串".to_string(),
        TokenKind::P(_) => "概率".to_string(),
        TokenKind::W(_) => "权重".to_string(),
        TokenKind::C(_) => "计数".to_string(),
        TokenKind::T(_) => "次数".to_string(),
        TokenKind::R(_) => "稀有度".to_string(),
        TokenKind::Bool(true) => "true".to_string(),
        TokenKind::Bool(false) => "false".to_string(),
        TokenKind::Scope(_) => "作用域".to_string(),
        TokenKind::Colon => ":".to_string(),
        TokenKind::Comma => ",".to_string(),
        TokenKind::LBrack => "[".to_string(),
        TokenKind::RBrack => "]".to_string(),
        TokenKind::LParen => "(".to_string(),
        TokenKind::RParen => ")".to_string(),
        TokenKind::Eq => "=".to_string(),
        TokenKind::PlusEq => "+=".to_string(),
        TokenKind::Arrow => "->".to_string(),
        TokenKind::EqEq => "==".to_string(),
        TokenKind::GtEq => ">=".to_string(),
        TokenKind::Percent => "%".to_string(),
        TokenKind::DotDot => "..".to_string(),
        TokenKind::Dot => ".".to_string(),
        TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::Eof => {
            "文件结束".to_string()
        }
    }
}

fn keyword_text(kw: Keyword) -> &'static str {
    match kw {
        Keyword::And => "and",
        Keyword::Banner => "banner",
        Keyword::Bind => "bind",
        Keyword::Choose => "choose",
        Keyword::Emit => "emit",
        Keyword::Entry => "entry",
        Keyword::Floor => "floor",
        Keyword::From => "from",
        Keyword::Grade => "grade",
        Keyword::Guarantee => "guarantee",
        Keyword::In => "in",
        Keyword::Item => "item",
        Keyword::Linear => "linear",
        Keyword::Not => "not",
        Keyword::Or => "or",
        Keyword::PityKey => "pity_key",
        Keyword::ProgressKey => "progress_key",
        Keyword::Promote => "promote",
        Keyword::Rarity => "rarity",
        Keyword::Reset => "reset",
        Keyword::Rest => "rest",
        Keyword::Result => "result",
        Keyword::Roster => "roster",
        Keyword::Rule => "rule",
        Keyword::Rescale => "rescale",
        Keyword::SourceRoster => "source_roster",
        Keyword::SourceRule => "source_rule",
        Keyword::Update => "update",
        Keyword::Weighted => "weighted",
    }
}

fn type_text(ty: TypeName) -> &'static str {
    match ty {
        TypeName::Action => "Action",
        TypeName::Bool => "Bool",
        TypeName::Chance => "Chance",
        TypeName::Count => "Count",
        TypeName::Counter => "Counter",
        TypeName::Distribution => "Distribution",
        TypeName::Event => "Event",
        TypeName::Item => "Item",
        TypeName::Pool => "Pool",
        TypeName::Rarity => "Rarity",
        TypeName::Selector => "Selector",
    }
}
