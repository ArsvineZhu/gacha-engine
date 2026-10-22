use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span, diag};
use crate::lex::{Keyword, Token, TokenKind, TypeName};

pub fn parse(file_name: &str, tokens: &[Token], ext: &str) -> Result<SourceFile, Diagnostic> {
    let mut p = Parser {
        file_name,
        tokens,
        index: 0,
    };
    let file = match ext {
        "gmr" => SourceFile::Rule(p.parse_rule()?),
        "gml" => SourceFile::Roster(p.parse_roster()?),
        "gmb" => SourceFile::Banners(p.parse_banners()?),
        _ => {
            return Err(p.fail("E_PARSE", p.peek().span, "gmr/gml/gmb", ext));
        }
    };
    p.expect_kind(&TokenKind::Eof, "文件结束")?;
    Ok(file)
}

struct Parser<'a> {
    file_name: &'a str,
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.index];
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
        token
    }

    fn fail(&self, code: &'static str, span: Span, expected: &str, actual: &str) -> Diagnostic {
        diag(code, self.file_name, span, expected, actual)
    }

    fn parse_error(&self, expected: &str) -> Diagnostic {
        let token = self.peek();
        self.fail("E_PARSE", token.span, expected, &describe(&token.kind))
    }

    fn expect_kind(&mut self, kind: &TokenKind, expected: &str) -> Result<Span, Diagnostic> {
        let token = self.peek();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(kind) {
            let span = token.span;
            self.advance();
            Ok(span)
        } else {
            Err(self.parse_error(expected))
        }
    }

    fn expect_kw(&mut self, kw: Keyword) -> Result<Span, Diagnostic> {
        let token = self.peek();
        if token.kind == TokenKind::Keyword(kw) {
            let span = token.span;
            self.advance();
            Ok(span)
        } else {
            Err(self.parse_error(keyword_name(kw)))
        }
    }

    fn eat_kw(&mut self, kw: Keyword) -> bool {
        if self.peek().kind == TokenKind::Keyword(kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_ident(&mut self) -> Result<Ident, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::Ident(name) = token.kind {
            self.advance();
            Ok(Ident {
                name,
                span: token.span,
            })
        } else {
            Err(self.parse_error("标识符"))
        }
    }

    fn expect_newline(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(&TokenKind::Newline, "换行")?;
        Ok(())
    }

    fn open_block(&mut self, colon: Span, code: &'static str) -> Result<(), Diagnostic> {
        self.expect_newline()?;
        if self.peek().kind == TokenKind::Indent {
            self.advance();
            Ok(())
        } else {
            Err(self.fail(code, colon, "缩进块", "空块"))
        }
    }

    fn close_block(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(&TokenKind::Dedent, "块结束")?;
        Ok(())
    }

    fn parse_rule(&mut self) -> Result<Rule, Diagnostic> {
        let keyword = self.expect_kw(Keyword::Rule)?;
        let name = self.expect_ident()?;
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        self.open_block(colon, "E_EMPTY_BLOCK")?;
        let mut decls = Vec::new();
        while matches!(self.peek().kind, TokenKind::Ident(_)) {
            decls.push(self.parse_decl()?);
        }
        let mut guarantees = Vec::new();
        while self.peek().kind == TokenKind::Keyword(Keyword::Guarantee) {
            guarantees.push(self.parse_guarantee()?);
        }
        let mut updates = Vec::new();
        while self.peek().kind == TokenKind::Keyword(Keyword::Update) {
            updates.push(self.parse_update()?);
        }
        self.close_block()?;
        Ok(Rule {
            keyword,
            name,
            decls,
            guarantees,
            updates,
        })
    }

    fn parse_roster(&mut self) -> Result<Roster, Diagnostic> {
        let keyword = self.expect_kw(Keyword::Roster)?;
        let name = self.expect_ident()?;
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        self.open_block(colon, "E_EMPTY_BLOCK")?;
        let mut entries = Vec::new();
        while self.peek().kind == TokenKind::Keyword(Keyword::Entry) {
            entries.push(self.parse_entry()?);
        }
        if entries.is_empty() {
            return Err(self.parse_error("entry"));
        }
        self.close_block()?;
        Ok(Roster {
            keyword,
            name,
            entries,
        })
    }

    fn parse_entry(&mut self) -> Result<Entry, Diagnostic> {
        self.expect_kw(Keyword::Entry)?;
        let name = self.expect_ident()?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let (rarity, rarity_span) = self.expect_r()?;
        self.expect_kind(&TokenKind::Comma, ",")?;
        let display = self.expect_string()?;
        self.expect_kind(&TokenKind::Comma, ",")?;
        let tags = self.parse_tags()?;
        self.expect_kind(&TokenKind::Comma, ",")?;
        let drawable = self.expect_bool()?;
        self.expect_newline()?;
        Ok(Entry {
            name,
            rarity,
            rarity_span,
            display,
            tags,
            drawable,
        })
    }

    fn parse_tags(&mut self) -> Result<Vec<String>, Diagnostic> {
        self.expect_kind(&TokenKind::LBrack, "[")?;
        let mut tags = Vec::new();
        if self.peek().kind != TokenKind::RBrack {
            tags.push(self.expect_string()?);
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                tags.push(self.expect_string()?);
            }
        }
        self.expect_kind(&TokenKind::RBrack, "]")?;
        Ok(tags)
    }

    fn parse_banners(&mut self) -> Result<Vec<Banner>, Diagnostic> {
        let mut banners = Vec::new();
        if self.peek().kind != TokenKind::Keyword(Keyword::Banner) {
            return Err(self.parse_error("banner"));
        }
        while self.peek().kind == TokenKind::Keyword(Keyword::Banner) {
            banners.push(self.parse_banner()?);
        }
        Ok(banners)
    }

    fn parse_banner(&mut self) -> Result<Banner, Diagnostic> {
        let keyword = self.expect_kw(Keyword::Banner)?;
        let name = self.expect_ident()?;
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        self.open_block(colon, "E_EMPTY_BLOCK")?;
        self.expect_kw(Keyword::SourceRule)?;
        let source_rule = self.expect_ident()?;
        self.expect_newline()?;
        self.expect_kw(Keyword::SourceRoster)?;
        let source_roster = self.expect_ident()?;
        self.expect_newline()?;
        let pity_key = if self.peek().kind == TokenKind::Keyword(Keyword::PityKey) {
            Some(self.parse_key(Keyword::PityKey)?)
        } else {
            None
        };
        let progress_key = if self.peek().kind == TokenKind::Keyword(Keyword::ProgressKey) {
            Some(self.parse_key(Keyword::ProgressKey)?)
        } else {
            None
        };
        let mut binds = Vec::new();
        while self.peek().kind == TokenKind::Keyword(Keyword::Bind) {
            binds.push(self.parse_bind()?);
        }
        self.close_block()?;
        Ok(Banner {
            keyword,
            name,
            source_rule,
            source_roster,
            pity_key,
            progress_key,
            binds,
        })
    }

    fn parse_key(&mut self, kw: Keyword) -> Result<Keyed, Diagnostic> {
        let keyword = self.expect_kw(kw)?;
        let value = self.expect_string()?;
        self.expect_newline()?;
        Ok(Keyed { keyword, value })
    }

    fn parse_bind(&mut self) -> Result<Bind, Diagnostic> {
        self.expect_kw(Keyword::Bind)?;
        let name = self.expect_ident()?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let ty = self.parse_bind_type()?;
        self.expect_kind(&TokenKind::Eq, "=")?;
        let rhs = if self.peek().kind == TokenKind::LBrack {
            let bracket = self.peek().span;
            self.advance();
            let mut names = Vec::new();
            if matches!(self.peek().kind, TokenKind::Ident(_)) {
                names.push(self.expect_ident()?);
                while self.peek().kind == TokenKind::Comma {
                    self.advance();
                    names.push(self.expect_ident()?);
                }
            }
            self.expect_kind(&TokenKind::RBrack, "]")?;
            BindRhs::Members { names, bracket }
        } else {
            BindRhs::Entry(self.expect_ident()?)
        };
        self.expect_newline()?;
        Ok(Bind { name, ty, rhs })
    }

    fn parse_bind_type(&mut self) -> Result<BindType, Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Type(TypeName::Pool) => {
                self.advance();
                self.expect_kind(&TokenKind::LBrack, "[")?;
                let (rarity, _) = self.expect_r()?;
                self.expect_kind(&TokenKind::RBrack, "]")?;
                Ok(BindType {
                    kind: BindKind::Pool,
                    rarity,
                    span: token.span,
                })
            }
            TokenKind::Type(TypeName::Item) => {
                self.advance();
                self.expect_kind(&TokenKind::LBrack, "[")?;
                let (rarity, _) = self.expect_r()?;
                self.expect_kind(&TokenKind::RBrack, "]")?;
                Ok(BindType {
                    kind: BindKind::Item,
                    rarity,
                    span: token.span,
                })
            }
            _ => Err(self.parse_error("Pool 或 Item")),
        }
    }

    fn parse_decl(&mut self) -> Result<Decl, Diagnostic> {
        let name = self.expect_ident()?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Type(TypeName::Counter) => self.parse_counter(name),
            TokenKind::Type(TypeName::Bool) => self.parse_bool(name),
            TokenKind::Type(TypeName::Pool) => {
                self.advance();
                self.expect_kind(&TokenKind::LBrack, "[")?;
                let (rarity, _) = self.expect_r()?;
                self.expect_kind(&TokenKind::RBrack, "]")?;
                self.expect_newline()?;
                Ok(Decl::Pool { name, rarity })
            }
            TokenKind::Type(TypeName::Item) => {
                self.advance();
                self.expect_kind(&TokenKind::LBrack, "[")?;
                let (rarity, _) = self.expect_r()?;
                self.expect_kind(&TokenKind::RBrack, "]")?;
                self.expect_newline()?;
                Ok(Decl::Item { name, rarity })
            }
            TokenKind::Type(TypeName::Event) => self.parse_event(name),
            TokenKind::Type(TypeName::Chance) => {
                self.advance();
                self.expect_kind(&TokenKind::Eq, "=")?;
                let expr = self.parse_chance()?;
                self.expect_newline()?;
                Ok(Decl::Chance { name, expr })
            }
            TokenKind::Type(TypeName::Selector) => self.parse_selector(name),
            TokenKind::Type(TypeName::Distribution) => self.parse_distribution(name),
            TokenKind::Type(TypeName::Action) => self.parse_action(name),
            _ => Err(self.parse_error("声明类型")),
        }
    }

    fn parse_counter(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Counter)?;
        self.expect_kind(&TokenKind::LBrack, "[")?;
        let upper = if let TokenKind::C(value) = self.peek().kind {
            self.advance();
            Some(value)
        } else {
            None
        };
        self.expect_kind(&TokenKind::RBrack, "]")?;
        let scope = self.expect_scope()?;
        self.expect_kind(&TokenKind::Eq, "=")?;
        let (initial, initial_span) = self.expect_c()?;
        self.expect_newline()?;
        Ok(Decl::Counter {
            name,
            upper,
            scope,
            initial,
            initial_span,
        })
    }

    fn parse_bool(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Bool)?;
        let scope = self.expect_scope()?;
        self.expect_kind(&TokenKind::Eq, "=")?;
        let initial = self.expect_bool()?;
        self.expect_newline()?;
        Ok(Decl::Bool {
            name,
            scope,
            initial,
        })
    }

    fn parse_event(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Event)?;
        self.expect_kind(&TokenKind::LBrack, "[")?;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RBrack {
            params.push(self.expect_param()?);
            while self.peek().kind == TokenKind::Comma {
                self.advance();
                params.push(self.expect_param()?);
            }
        }
        self.expect_kind(&TokenKind::RBrack, "]")?;
        self.expect_newline()?;
        Ok(Decl::Event { name, params })
    }

    fn expect_param(&mut self) -> Result<Param, Diagnostic> {
        let token = self.peek().clone();
        let kind = match token.kind {
            TokenKind::Type(TypeName::Count) => ParamKind::Count,
            TokenKind::Type(TypeName::Bool) => ParamKind::Bool,
            TokenKind::Type(TypeName::Rarity) => ParamKind::Rarity,
            TokenKind::Type(TypeName::Item) => ParamKind::Item,
            _ => return Err(self.parse_error("事件参数类型")),
        };
        self.advance();
        Ok(Param {
            kind,
            span: token.span,
        })
    }

    fn parse_selector(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Selector)?;
        self.expect_kind(&TokenKind::LBrack, "[")?;
        let (rarity, _) = self.expect_r()?;
        self.expect_kind(&TokenKind::RBrack, "]")?;
        self.expect_kind(&TokenKind::Eq, "=")?;
        let rhs = if self.peek().kind == TokenKind::Keyword(Keyword::Weighted) {
            self.advance();
            let colon = self.expect_kind(&TokenKind::Colon, ":")?;
            self.open_block(colon, "E_WEIGHT_EMPTY")?;
            let mut rows = Vec::new();
            while self.peek().kind != TokenKind::Dedent {
                rows.push(self.parse_weight_row()?);
            }
            if rows.is_empty() {
                return Err(self.fail("E_WEIGHT_EMPTY", colon, "权重分支", "空"));
            }
            self.close_block()?;
            SelectorRhs::Weighted { colon, rows }
        } else {
            let inline = self.parse_selector_inline()?;
            self.expect_newline()?;
            inline
        };
        Ok(Decl::Selector { name, rarity, rhs })
    }

    fn parse_weight_row(&mut self) -> Result<WeightRow, Diagnostic> {
        let token = self.peek().clone();
        let TokenKind::W(weight) = token.kind else {
            return Err(self.parse_error("权重"));
        };
        self.advance();
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        if !self.starts_selector() {
            return Err(self.fail("E_PARSE", colon, "子选择器", "空"));
        }
        let child = self.parse_selector_inline()?;
        self.expect_newline()?;
        Ok(WeightRow {
            weight,
            weight_span: token.span,
            child,
        })
    }

    fn starts_selector(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Keyword(Keyword::From)
                | TokenKind::Keyword(Keyword::Choose)
                | TokenKind::Ident(_)
        )
    }

    fn parse_selector_inline(&mut self) -> Result<SelectorRhs, Diagnostic> {
        if self.eat_kw(Keyword::From) {
            return Ok(SelectorRhs::From(self.expect_ident()?));
        }
        if self.eat_kw(Keyword::Choose) {
            self.expect_kind(&TokenKind::LParen, "(")?;
            let cond = self.parse_cond()?;
            self.expect_kind(&TokenKind::Comma, ",")?;
            let yes = self.parse_selector_inline()?;
            self.expect_kind(&TokenKind::Comma, ",")?;
            let no = self.parse_selector_inline()?;
            self.expect_kind(&TokenKind::RParen, ")")?;
            return Ok(SelectorRhs::Choose {
                cond,
                yes: Box::new(yes),
                no: Box::new(no),
            });
        }
        if matches!(self.peek().kind, TokenKind::Ident(_)) {
            return Ok(SelectorRhs::Name(self.expect_ident()?));
        }
        Err(self.parse_error("选择器"))
    }

    fn parse_distribution(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Distribution)?;
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        self.open_block(colon, "E_EMPTY_BLOCK")?;
        let mut rows = Vec::new();
        while self.peek().kind != TokenKind::Dedent {
            rows.push(self.parse_dist_row()?);
        }
        if rows.is_empty() {
            return Err(self.fail("E_EMPTY_BLOCK", colon, "分布行", "空"));
        }
        self.close_block()?;
        Ok(Decl::Distribution { name, rows })
    }

    fn parse_dist_row(&mut self) -> Result<DistRow, Diagnostic> {
        let (rarity, rarity_span) = self.expect_r()?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let amount = self.parse_amount()?;
        self.expect_kind(&TokenKind::Arrow, "->")?;
        let selector = self.expect_ident()?;
        self.expect_newline()?;
        Ok(DistRow {
            rarity,
            rarity_span,
            amount,
            selector,
        })
    }

    fn parse_amount(&mut self) -> Result<Amount, Diagnostic> {
        let token = self.peek().clone();
        if token.kind == TokenKind::Keyword(Keyword::Rest) {
            self.advance();
            return Ok(Amount::Rest(token.span));
        }
        if let TokenKind::W(weight) = token.kind {
            self.advance();
            return Ok(Amount::Weight(weight, token.span));
        }
        Ok(Amount::Chance(self.parse_chance()?))
    }

    fn parse_chance(&mut self) -> Result<ChanceExpr, Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::P(value) => {
                self.advance();
                Ok(ChanceExpr::Lit(value, token.span))
            }
            TokenKind::Ident(_) => Ok(ChanceExpr::Name(self.expect_ident()?)),
            TokenKind::Keyword(Keyword::Linear) => {
                self.advance();
                self.expect_kind(&TokenKind::LParen, "(")?;
                let counter = self.expect_ident()?;
                self.expect_kind(&TokenKind::Comma, ",")?;
                let (start, _) = self.expect_c()?;
                self.expect_kind(&TokenKind::Comma, ",")?;
                let (base, base_span) = self.expect_p()?;
                self.expect_kind(&TokenKind::Comma, ",")?;
                let (step, step_span) = self.expect_p()?;
                self.expect_kind(&TokenKind::Comma, ",")?;
                let (cap, cap_span) = self.expect_p()?;
                self.expect_kind(&TokenKind::RParen, ")")?;
                Ok(ChanceExpr::Linear {
                    counter,
                    start,
                    base,
                    base_span,
                    step,
                    step_span,
                    cap,
                    cap_span,
                })
            }
            _ => Err(self.parse_error("Chance")),
        }
    }

    fn parse_action(&mut self, name: Ident) -> Result<Decl, Diagnostic> {
        self.expect_type(TypeName::Action)?;
        self.expect_kind(&TokenKind::LBrack, "[")?;
        let (lo, lo_span) = self.expect_t()?;
        self.expect_kind(&TokenKind::DotDot, "..")?;
        let (hi, hi_span) = self.expect_t()?;
        self.expect_kind(&TokenKind::RBrack, "]")?;
        self.expect_kind(&TokenKind::Eq, "=")?;
        let target = self.expect_ident()?;
        self.expect_newline()?;
        Ok(Decl::Action {
            name,
            lo,
            lo_span,
            hi,
            hi_span,
            target,
        })
    }

    fn parse_guarantee(&mut self) -> Result<Guarantee, Diagnostic> {
        self.expect_kw(Keyword::Guarantee)?;
        let name = self.expect_ident()?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let cond = self.parse_cond()?;
        self.expect_kind(&TokenKind::Arrow, "->")?;
        let effect = self.parse_effect()?;
        self.expect_newline()?;
        Ok(Guarantee { name, cond, effect })
    }

    fn parse_effect(&mut self) -> Result<Effect, Diagnostic> {
        if self.eat_kw(Keyword::Item) {
            return Ok(Effect::Item(self.expect_ident()?));
        }
        if self.eat_kw(Keyword::Rarity) {
            let (value, span) = self.expect_r()?;
            return Ok(Effect::Rarity { value, span });
        }
        let keyword = self.expect_kw(Keyword::Floor)?;
        let (value, _) = self.expect_r()?;
        let mode = if self.eat_kw(Keyword::Rescale) {
            FloorMode::Rescale
        } else if self.eat_kw(Keyword::Promote) {
            FloorMode::Promote
        } else {
            return Err(self.parse_error("rescale 或 promote"));
        };
        Ok(Effect::Floor {
            keyword,
            value,
            mode,
        })
    }

    fn parse_update(&mut self) -> Result<Update, Diagnostic> {
        self.expect_kw(Keyword::Update)?;
        let cond = self.parse_cond()?;
        let colon = self.expect_kind(&TokenKind::Colon, ":")?;
        self.open_block(colon, "E_EMPTY_BLOCK")?;
        let mut ops = Vec::new();
        while self.peek().kind != TokenKind::Dedent {
            ops.push(self.parse_op()?);
        }
        if ops.is_empty() {
            return Err(self.fail("E_EMPTY_BLOCK", colon, "更新", "空"));
        }
        self.close_block()?;
        Ok(Update { cond, ops })
    }

    fn parse_op(&mut self) -> Result<Op, Diagnostic> {
        if self.eat_kw(Keyword::Reset) {
            let name = self.expect_ident()?;
            self.expect_newline()?;
            return Ok(Op::Reset { name });
        }
        if self.eat_kw(Keyword::Emit) {
            let name = self.expect_ident()?;
            self.expect_kind(&TokenKind::LParen, "(")?;
            let mut args = Vec::new();
            if self.peek().kind != TokenKind::RParen {
                args.push(self.parse_arg()?);
                while self.peek().kind == TokenKind::Comma {
                    self.advance();
                    args.push(self.parse_arg()?);
                }
            }
            self.expect_kind(&TokenKind::RParen, ")")?;
            self.expect_newline()?;
            return Ok(Op::Emit { name, args });
        }
        let name = self.expect_ident()?;
        if self.peek().kind == TokenKind::Eq {
            self.advance();
            let value = self.expect_bool()?;
            self.expect_newline()?;
            return Ok(Op::SetBool { name, value });
        }
        if self.peek().kind == TokenKind::PlusEq {
            self.advance();
            let (delta, delta_span) = self.expect_c()?;
            self.expect_newline()?;
            return Ok(Op::Add {
                name,
                delta,
                delta_span,
            });
        }
        Err(self.parse_error("= 或 +="))
    }

    fn parse_arg(&mut self) -> Result<Arg, Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::C(value) => {
                self.advance();
                Ok(Arg::Count(value, token.span))
            }
            TokenKind::R(value) => {
                self.advance();
                Ok(Arg::Rarity(value, token.span))
            }
            TokenKind::Bool(value) => {
                self.advance();
                Ok(Arg::Bool(value, token.span))
            }
            TokenKind::Keyword(Keyword::Result) => {
                self.advance();
                if self.peek().kind == TokenKind::Dot {
                    self.advance();
                    let grade = self.expect_kw(Keyword::Grade)?;
                    Ok(Arg::Grade(grade))
                } else {
                    Ok(Arg::Result(token.span))
                }
            }
            TokenKind::Ident(_) => Ok(Arg::Name(self.expect_ident()?)),
            _ => Err(self.parse_error("事件参数")),
        }
    }

    fn parse_cond(&mut self) -> Result<Cond, Diagnostic> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Cond, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.eat_kw(Keyword::Or) {
            let right = self.parse_and()?;
            left = Cond::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Cond, Diagnostic> {
        let mut left = self.parse_not()?;
        while self.eat_kw(Keyword::And) {
            let right = self.parse_not()?;
            left = Cond::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Cond, Diagnostic> {
        if self.eat_kw(Keyword::Not) {
            return Ok(Cond::Not(Box::new(self.parse_not()?)));
        }
        if self.peek().kind == TokenKind::LParen {
            self.advance();
            let cond = self.parse_or()?;
            self.expect_kind(&TokenKind::RParen, ")")?;
            return Ok(cond);
        }
        self.parse_predicate()
    }

    fn parse_predicate(&mut self) -> Result<Cond, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::Bool(value) = token.kind {
            self.advance();
            return Ok(Cond::BoolLit(value, token.span));
        }
        if token.kind == TokenKind::Keyword(Keyword::Result) {
            self.advance();
            if self.peek().kind == TokenKind::Dot {
                self.advance();
                self.expect_kw(Keyword::Grade)?;
                let op_token = self.peek().clone();
                if op_token.kind == TokenKind::EqEq {
                    self.advance();
                    let (value, _) = self.expect_r()?;
                    return Ok(Cond::GradeEq {
                        result: token.span,
                        value,
                    });
                }
                if op_token.kind == TokenKind::GtEq {
                    self.advance();
                    let (value, _) = self.expect_r()?;
                    return Ok(Cond::GradeGe {
                        result: token.span,
                        value,
                    });
                }
                return Err(self.parse_error("== 或 >="));
            }
            if self.eat_kw(Keyword::In) {
                let pool = self.expect_ident()?;
                return Ok(Cond::InPool {
                    result: token.span,
                    pool,
                });
            }
            return Err(self.parse_error("result 条件"));
        }
        let name = self.expect_ident()?;
        let op = self.peek().clone();
        if op.kind == TokenKind::EqEq {
            self.advance();
            return self.parse_eq_rhs(name);
        }
        if op.kind == TokenKind::GtEq {
            self.advance();
            let (value, value_span) = self.expect_c()?;
            return Ok(Cond::CountGe {
                name,
                value,
                value_span,
            });
        }
        if op.kind == TokenKind::Percent {
            self.advance();
            let (modulus, modulus_span) = self.expect_c()?;
            self.expect_kind(&TokenKind::EqEq, "==")?;
            let (remainder, remainder_span) = self.expect_c()?;
            return Ok(Cond::CountMod {
                name,
                modulus,
                modulus_span,
                remainder,
                remainder_span,
            });
        }
        Err(self.parse_error("比较"))
    }

    fn parse_eq_rhs(&mut self, name: Ident) -> Result<Cond, Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::C(value) => {
                self.advance();
                Ok(Cond::CountEq {
                    name,
                    value,
                    value_span: token.span,
                })
            }
            TokenKind::Bool(value) => {
                self.advance();
                Ok(Cond::BoolEq {
                    name,
                    value,
                    value_span: token.span,
                })
            }
            _ => Err(self.parse_error("计数或布尔")),
        }
    }

    fn expect_type(&mut self, ty: TypeName) -> Result<(), Diagnostic> {
        let token = self.peek().clone();
        if token.kind == TokenKind::Type(ty) {
            self.advance();
            Ok(())
        } else {
            Err(self.parse_error("类型"))
        }
    }

    fn expect_scope(&mut self) -> Result<crate::lex::Scope, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::Scope(scope) = token.kind {
            self.advance();
            Ok(scope)
        } else {
            Err(self.parse_error("作用域"))
        }
    }

    fn expect_string(&mut self) -> Result<String, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::String(value) = token.kind {
            self.advance();
            Ok(value)
        } else {
            Err(self.parse_error("字符串"))
        }
    }

    fn expect_bool(&mut self) -> Result<bool, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::Bool(value) = token.kind {
            self.advance();
            Ok(value)
        } else {
            Err(self.parse_error("布尔值"))
        }
    }

    fn expect_c(&mut self) -> Result<(u128, Span), Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::C(value) = token.kind {
            self.advance();
            Ok((value, token.span))
        } else {
            Err(self.parse_error("计数"))
        }
    }

    fn expect_t(&mut self) -> Result<(u128, Span), Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::T(value) = token.kind {
            self.advance();
            Ok((value, token.span))
        } else {
            Err(self.parse_error("次数"))
        }
    }

    fn expect_r(&mut self) -> Result<(u128, Span), Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::R(value) = token.kind {
            self.advance();
            Ok((value, token.span))
        } else {
            Err(self.parse_error("稀有度"))
        }
    }

    fn expect_p(&mut self) -> Result<(crate::rational::Rational, Span), Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::P(value) = token.kind {
            self.advance();
            Ok((value, token.span))
        } else {
            Err(self.parse_error("概率"))
        }
    }
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(name) => name.clone(),
        TokenKind::Keyword(kw) => keyword_name(*kw).to_string(),
        TokenKind::Type(ty) => format!("{ty:?}"),
        TokenKind::Eof => "文件结束".to_string(),
        other => format!("{other:?}"),
    }
}

fn keyword_name(kw: Keyword) -> &'static str {
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
