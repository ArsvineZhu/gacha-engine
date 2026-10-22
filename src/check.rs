use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span, diag};
use crate::image::*;
use crate::lex::Scope;
use crate::rational::Rational;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Name,
    Type,
    Counter,
    Linear,
    Distribution,
    Rest,
    Shape,
    Guarantee,
    Bind,
    Global,
}

struct Issue {
    phase: Phase,
    file: u16,
    line: u32,
    column: u32,
    diag: Diagnostic,
}

#[derive(Clone)]
enum Kind {
    Counter { upper: Option<u128> },
    Bool,
    Pool { rarity: u128 },
    Item { rarity: u128 },
    Event(Vec<ParamKind>),
    Chance(Option<ChanceRt>),
    Selector { rarity: u128, index: Option<usize> },
    Distribution(Option<usize>),
    Action,
    Guarantee,
}

struct Sym {
    name_at: (u32, u32),
    rhs_end: (u32, u32),
    kind: Kind,
}

struct RuleBuild {
    syms: BTreeMap<String, Sym>,
    states: Vec<StateRt>,
    selectors: Vec<Sel>,
    distributions: Vec<Dist>,
    actions: Vec<ActionRt>,
    guarantees: Vec<Guar>,
    updates: Vec<UpdateRt>,
    pools: Vec<String>,
    items: Vec<String>,
    has_pity: bool,
    has_progress: bool,
    dist_rows: Vec<Vec<RowView>>,
}

struct RowView {
    rarity: u128,
    amount: RowAmount,
    amount_span: Span,
    selector: usize,
}

enum RowAmount {
    Fixed(ChanceRt),
    Rest,
    Weight(Rational),
}

struct EntryInfo {
    rarity: u128,
    drawable: bool,
    tags: Vec<String>,
}

struct RosterBuild {
    entries: BTreeMap<String, EntryInfo>,
}

pub fn check(files: &[(String, SourceFile)]) -> Result<Image, Diagnostic> {
    let mut issues = Vec::new();
    let mut roots: BTreeMap<String, Span> = BTreeMap::new();
    let mut rules: BTreeMap<String, RuleBuild> = BTreeMap::new();
    let mut rosters: BTreeMap<String, RosterBuild> = BTreeMap::new();
    let mut banners = Vec::new();

    for (name, file) in files {
        match file {
            SourceFile::Rule(rule) => {
                note_root(&mut issues, name, &mut roots, &rule.name);
                rules.insert(rule.name.name.clone(), build_rule(name, rule, &mut issues));
            }
            SourceFile::Roster(roster) => {
                note_root(&mut issues, name, &mut roots, &roster.name);
                rosters.insert(
                    roster.name.name.clone(),
                    build_roster(name, roster, &mut issues),
                );
            }
            SourceFile::Banners(list) => {
                for banner in list {
                    note_root(&mut issues, name, &mut roots, &banner.name);
                    banners.push((name.clone(), banner));
                }
            }
        }
    }

    let mut schemas: BTreeMap<(u8, String, String), Schema> = BTreeMap::new();
    let mut image_banners = Vec::new();
    for (file_name, banner) in &banners {
        if let Some(built) = bind_banner(
            file_name,
            banner,
            &rules,
            &rosters,
            &mut schemas,
            &mut issues,
        ) {
            image_banners.push(built);
        }
    }

    if let Some(issue) = issues.into_iter().min_by(|a, b| {
        (a.phase, a.file, a.line, a.column).cmp(&(b.phase, b.file, b.line, b.column))
    }) {
        return Err(issue.diag);
    }
    Ok(Image {
        banners: image_banners,
    })
}

fn note_root(
    issues: &mut Vec<Issue>,
    file: &str,
    roots: &mut BTreeMap<String, Span>,
    name: &Ident,
) {
    if roots.insert(name.name.clone(), name.span).is_some() {
        push(
            issues,
            file,
            Phase::Name,
            name.span,
            "E_DUP_NAME",
            "唯一根名字",
            &name.name,
        );
    }
}

fn build_roster(file: &str, roster: &Roster, issues: &mut Vec<Issue>) -> RosterBuild {
    let mut entries = BTreeMap::new();
    for entry in &roster.entries {
        if entries
            .insert(
                entry.name.name.clone(),
                EntryInfo {
                    rarity: entry.rarity,
                    drawable: entry.drawable,
                    tags: entry.tags.clone(),
                },
            )
            .is_some()
        {
            push(
                issues,
                file,
                Phase::Name,
                entry.name.span,
                "E_DUP_NAME",
                "唯一条目",
                &entry.name.name,
            );
        }
    }
    RosterBuild { entries }
}

fn build_rule(file: &str, rule: &Rule, issues: &mut Vec<Issue>) -> RuleBuild {
    let mut syms = BTreeMap::new();
    for decl in &rule.decls {
        let (name, kind, rhs_end) = decl_sym(decl);
        insert_sym(file, issues, &mut syms, name, kind, rhs_end);
    }
    for guarantee in &rule.guarantees {
        insert_sym(
            file,
            issues,
            &mut syms,
            &guarantee.name,
            Kind::Guarantee,
            at(&guarantee.name),
        );
    }

    let mut out = RuleBuild {
        syms,
        states: Vec::new(),
        selectors: Vec::new(),
        distributions: Vec::new(),
        actions: Vec::new(),
        guarantees: Vec::new(),
        updates: Vec::new(),
        pools: Vec::new(),
        items: Vec::new(),
        has_pity: false,
        has_progress: false,
        dist_rows: Vec::new(),
    };

    for decl in &rule.decls {
        lower_decl(file, issues, &mut out, decl);
    }
    let mut action_dists = Vec::new();
    for action in &out.actions {
        action_dists.push(action.distribution);
    }
    if out.actions.is_empty() || out.distributions.is_empty() {
        push(
            issues,
            file,
            Phase::Shape,
            rule.keyword,
            "E_RULE_SHAPE",
            "至少一个 Action 和 Distribution",
            "规则形状不完整",
        );
    }
    for guarantee in &rule.guarantees {
        lower_guarantee(file, issues, &mut out, guarantee, &action_dists);
    }
    for update in &rule.updates {
        lower_update(file, issues, &mut out, update);
    }
    out
}

fn decl_sym(decl: &Decl) -> (&Ident, Kind, (u32, u32)) {
    match decl {
        Decl::Counter { name, upper, .. } => (name, Kind::Counter { upper: *upper }, at(name)),
        Decl::Bool { name, .. } => (name, Kind::Bool, at(name)),
        Decl::Pool { name, rarity } => (name, Kind::Pool { rarity: *rarity }, at(name)),
        Decl::Item { name, rarity } => (name, Kind::Item { rarity: *rarity }, at(name)),
        Decl::Event { name, params } => (
            name,
            Kind::Event(params.iter().map(|p| p.kind).collect()),
            at(name),
        ),
        Decl::Chance { name, expr } => (name, Kind::Chance(None), chance_end(name, expr)),
        Decl::Selector { name, rarity, rhs } => (
            name,
            Kind::Selector {
                rarity: *rarity,
                index: None,
            },
            selector_end(name, rhs),
        ),
        Decl::Distribution { name, .. } => (name, Kind::Distribution(None), at(name)),
        Decl::Action { name, .. } => (name, Kind::Action, at(name)),
    }
}

fn insert_sym(
    file: &str,
    issues: &mut Vec<Issue>,
    syms: &mut BTreeMap<String, Sym>,
    name: &Ident,
    kind: Kind,
    rhs_end: (u32, u32),
) {
    let sym = Sym {
        name_at: at(name),
        rhs_end,
        kind,
    };
    if syms.insert(name.name.clone(), sym).is_some() {
        push(
            issues,
            file,
            Phase::Name,
            name.span,
            "E_DUP_NAME",
            "唯一名字",
            &name.name,
        );
    }
}

fn lower_decl(file: &str, issues: &mut Vec<Issue>, rule: &mut RuleBuild, decl: &Decl) {
    match decl {
        Decl::Counter {
            name,
            upper,
            scope,
            initial,
            initial_span,
        } => {
            if let Some(limit) = upper {
                if initial > limit {
                    push(
                        issues,
                        file,
                        Phase::Counter,
                        *initial_span,
                        "E_COUNTER_INIT",
                        "不超过上界的初值",
                        "初值越界",
                    );
                }
            }
            if *scope == Scope::Pity {
                rule.has_pity = true;
            }
            if *scope == Scope::Progress {
                rule.has_progress = true;
            }
            rule.states.push(StateRt {
                name: name.name.clone(),
                scope: *scope,
                count: Some(CountRt {
                    initial: *initial,
                    upper: *upper,
                }),
                bool_initial: None,
            });
        }
        Decl::Bool {
            name,
            scope,
            initial,
        } => {
            if *scope == Scope::Pity {
                rule.has_pity = true;
            }
            if *scope == Scope::Progress {
                rule.has_progress = true;
            }
            rule.states.push(StateRt {
                name: name.name.clone(),
                scope: *scope,
                count: None,
                bool_initial: Some(*initial),
            });
        }
        Decl::Pool { name, .. } => rule.pools.push(name.name.clone()),
        Decl::Item { name, .. } => rule.items.push(name.name.clone()),
        Decl::Event { .. } => {}
        Decl::Chance { name, expr } => {
            if let Some(node) = lower_chance(file, issues, rule, expr) {
                if let Some(sym) = rule.syms.get_mut(&name.name) {
                    sym.kind = Kind::Chance(Some(node));
                }
            }
        }
        Decl::Selector { name, rarity, rhs } => {
            if let Some(sel) = lower_selector(file, issues, rule, *rarity, rhs) {
                let index = rule.selectors.len();
                rule.selectors.push(sel);
                if let Some(sym) = rule.syms.get_mut(&name.name) {
                    if let Kind::Selector { index: slot, .. } = &mut sym.kind {
                        *slot = Some(index);
                    }
                }
            }
        }
        Decl::Distribution { name, rows } => {
            let (index, views) = lower_distribution(file, issues, rule, rows);
            if let Some(sym) = rule.syms.get_mut(&name.name) {
                sym.kind = Kind::Distribution(Some(index));
            }
            rule.dist_rows.push(views);
        }
        Decl::Action {
            name,
            lo,
            lo_span,
            hi,
            hi_span,
            target,
        } => {
            if *lo < 1 || *lo > 1000 {
                push(
                    issues,
                    file,
                    Phase::Shape,
                    *lo_span,
                    "E_ACTION_RANGE",
                    "1 到 1000",
                    "动作次数",
                );
            } else if *hi < 1 || *hi > 1000 || hi < lo {
                push(
                    issues,
                    file,
                    Phase::Shape,
                    *hi_span,
                    "E_ACTION_RANGE",
                    "1 到 1000 且下界不超过上界",
                    "动作次数",
                );
            }
            let dist = match resolve(file, issues, rule, target) {
                Some(Kind::Distribution(Some(index))) => Some(index),
                Some(Kind::Distribution(None)) => None,
                Some(other) => {
                    push(
                        issues,
                        file,
                        Phase::Type,
                        target.span,
                        "E_TYPE",
                        "Distribution",
                        &kind_text(&other),
                    );
                    None
                }
                None => None,
            };
            if let Some(distribution) = dist {
                rule.actions.push(ActionRt {
                    name: name.name.clone(),
                    lo: *lo,
                    hi: *hi,
                    distribution,
                });
            }
        }
    }
}

fn lower_chance(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    expr: &ChanceExpr,
) -> Option<ChanceRt> {
    match expr {
        ChanceExpr::Lit(value, _) => Some(ChanceRt::Const(*value)),
        ChanceExpr::Name(name) => match resolve(file, issues, rule, name) {
            Some(Kind::Chance(Some(node))) => Some(node),
            Some(Kind::Chance(None)) => None,
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Chance",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        ChanceExpr::Linear {
            counter,
            start,
            base,
            base_span: _,
            step,
            step_span,
            cap,
            cap_span,
        } => match resolve(file, issues, rule, counter) {
            Some(Kind::Counter { upper, .. }) => {
                if !(*base < *cap
                    && *cap <= Rational::one()
                    && *step > Rational::zero()
                    && *step <= Rational::one())
                {
                    let span = if base >= cap { *cap_span } else { *step_span };
                    push(
                        issues,
                        file,
                        Phase::Linear,
                        span,
                        "E_LINEAR_ARGS",
                        "0 <= base < cap <= 1 且 0 < step <= 1",
                        "曲线参数",
                    );
                    return None;
                }
                Some(ChanceRt::Linear {
                    state: counter.name.clone(),
                    start: *start,
                    base: *base,
                    step: *step,
                    cap: *cap,
                    upper,
                })
            }
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    counter.span,
                    "E_TYPE",
                    "Counter",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
    }
}

fn lower_selector(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    rarity: u128,
    rhs: &SelectorRhs,
) -> Option<Sel> {
    match rhs {
        SelectorRhs::From(pool) => match resolve(file, issues, rule, pool) {
            Some(Kind::Pool { rarity: got }) if got == rarity => {
                Some(Sel::Uniform(pool.name.clone()))
            }
            Some(Kind::Pool { rarity: got }) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    pool.span,
                    "E_TYPE",
                    &format!("Pool[{rarity}r]"),
                    &format!("Pool[{got}r]"),
                );
                None
            }
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    pool.span,
                    "E_TYPE",
                    &format!("Pool[{rarity}r]"),
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        SelectorRhs::Name(name) => match resolve(file, issues, rule, name) {
            Some(Kind::Selector {
                rarity: got,
                index: Some(index),
            }) if got == rarity => Some(Sel::Ref(index)),
            Some(Kind::Selector { rarity: got, .. }) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    &format!("Selector[{rarity}r]"),
                    &format!("Selector[{got}r]"),
                );
                None
            }
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    &format!("Selector[{rarity}r]"),
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        SelectorRhs::Choose { cond, yes, no } => {
            let cond = lower_cond(file, issues, rule, cond, true)?;
            let yes = lower_selector(file, issues, rule, rarity, yes)?;
            let no = lower_selector(file, issues, rule, rarity, no)?;
            let yes_i = rule.selectors.len();
            rule.selectors.push(yes);
            let no_i = rule.selectors.len();
            rule.selectors.push(no);
            Some(Sel::If(cond, yes_i, no_i))
        }
        SelectorRhs::Weighted { rows, .. } => {
            let mut branches = Vec::new();
            for row in rows {
                let child = lower_selector(file, issues, rule, rarity, &row.child)?;
                let index = rule.selectors.len();
                rule.selectors.push(child);
                branches.push((row.weight, index));
            }
            Some(Sel::Weighted(branches))
        }
    }
}

fn lower_distribution(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    rows: &[DistRow],
) -> (usize, Vec<RowView>) {
    let mut seen = BTreeSet::new();
    let mut views = Vec::new();
    let mut typed_ok = true;
    for row in rows {
        if !seen.insert(row.rarity) {
            push(
                issues,
                file,
                Phase::Distribution,
                row.rarity_span,
                "E_DISTRIBUTION",
                "不重复的稀有度",
                "重复稀有度",
            );
            typed_ok = false;
        }
        let selector = match resolve(file, issues, rule, &row.selector) {
            Some(Kind::Selector {
                rarity,
                index: Some(index),
            }) if rarity == row.rarity => Some(index),
            Some(Kind::Selector { rarity, .. }) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    row.selector.span,
                    "E_TYPE",
                    &format!("Selector[{}r]", row.rarity),
                    &format!("Selector[{rarity}r]"),
                );
                typed_ok = false;
                None
            }
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    row.selector.span,
                    "E_TYPE",
                    &format!("Selector[{}r]", row.rarity),
                    &kind_text(&other),
                );
                typed_ok = false;
                None
            }
            None => {
                typed_ok = false;
                None
            }
        };
        let (amount, amount_span, amount_ok) = match &row.amount {
            Amount::Rest(span) => (RowAmount::Rest, *span, true),
            Amount::Weight(value, span) => (RowAmount::Weight(*value), *span, true),
            Amount::Chance(expr) => match lower_chance(file, issues, rule, expr) {
                Some(node) => (RowAmount::Fixed(node), chance_span(expr), true),
                None => {
                    typed_ok = false;
                    (RowAmount::Rest, chance_span(expr), false)
                }
            },
        };
        if !amount_ok {
            typed_ok = false;
        }
        views.push(RowView {
            rarity: row.rarity,
            amount,
            amount_span,
            selector: selector.unwrap_or(0),
        });
    }
    if typed_ok {
        check_distribution_shape(file, issues, &views);
    }
    let dist = Dist {
        rows: views
            .iter()
            .map(|row| DistRt {
                rarity: row.rarity,
                amount: match &row.amount {
                    RowAmount::Fixed(node) => AmountRt::Fixed(node.clone()),
                    RowAmount::Rest => AmountRt::Rest,
                    RowAmount::Weight(value) => AmountRt::Weight(*value),
                },
                selector: row.selector,
            })
            .collect(),
    };
    let index = rule.distributions.len();
    rule.distributions.push(dist);
    (index, views)
}

fn check_distribution_shape(file: &str, issues: &mut Vec<Issue>, rows: &[RowView]) {
    let mut max_sum = Rational::zero();
    let mut min_sum = Rational::zero();
    let mut fixed_spans = Vec::new();
    for row in rows {
        if let RowAmount::Fixed(node) = &row.amount {
            let (min, max) = chance_bounds(node);
            min_sum = min_sum.checked_add(min).expect("概率和");
            let next = max_sum.checked_add(max).expect("概率和");
            if next > Rational::one() && max_sum <= Rational::one() {
                push(
                    issues,
                    file,
                    Phase::Distribution,
                    row.amount_span,
                    "E_DISTRIBUTION",
                    "固定概率之和不超过 1",
                    "固定概率超过 1",
                );
            }
            max_sum = next;
            fixed_spans.push(row.amount_span);
        }
    }
    let rests: Vec<&RowView> = rows
        .iter()
        .filter(|r| matches!(r.amount, RowAmount::Rest))
        .collect();
    let weights: Vec<&RowView> = rows
        .iter()
        .filter(|r| matches!(r.amount, RowAmount::Weight(_)))
        .collect();
    if !rests.is_empty() && !weights.is_empty() {
        let span = if rests[0].amount_span.line < weights[0].amount_span.line
            || (rests[0].amount_span.line == weights[0].amount_span.line
                && rests[0].amount_span.column < weights[0].amount_span.column)
        {
            weights[0].amount_span
        } else {
            rests[0].amount_span
        };
        push(
            issues,
            file,
            Phase::Rest,
            span,
            "E_REST_MIX",
            "单一剩余档或至少两个权重档",
            "rest 与权重混用",
        );
        return;
    }
    if rests.len() > 1 {
        push(
            issues,
            file,
            Phase::Rest,
            rests[1].amount_span,
            "E_REST_FORM",
            "恰好一个 rest",
            "多个 rest",
        );
        return;
    }
    if weights.len() == 1 {
        push(
            issues,
            file,
            Phase::Rest,
            weights[0].amount_span,
            "E_REST_FORM",
            "rest 或至少两个权重档",
            "单独的权重档",
        );
        return;
    }
    if rests.is_empty()
        && weights.is_empty()
        && (min_sum != Rational::one() || max_sum != Rational::one())
    {
        if let Some(span) = fixed_spans.first() {
            if max_sum <= Rational::one() {
                push(
                    issues,
                    file,
                    Phase::Distribution,
                    *span,
                    "E_DISTRIBUTION",
                    "固定概率之和恰为 1",
                    "固定概率不足 1",
                );
            }
        }
    }
}

fn lower_guarantee(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    guarantee: &Guarantee,
    action_dists: &[usize],
) {
    let Some(when) = lower_cond(file, issues, rule, &guarantee.cond, true) else {
        return;
    };
    let effect = match &guarantee.effect {
        Effect::Item(name) => match resolve(file, issues, rule, name) {
            Some(Kind::Item { .. }) => Some(EffectRt::Item(name.name.clone())),
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Item",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        Effect::Rarity { value, span } => {
            for dist in action_dists {
                let rows = &rule.dist_rows[*dist];
                if !rows.iter().any(|row| row.rarity == *value) {
                    push(
                        issues,
                        file,
                        Phase::Guarantee,
                        *span,
                        "E_GUARANTEE_EFFECT",
                        "分布中的稀有度",
                        "缺少对应分布行",
                    );
                }
            }
            Some(EffectRt::Rarity(*value))
        }
        Effect::Floor {
            keyword,
            value,
            mode,
        } => {
            for dist in action_dists {
                let rows = &rule.dist_rows[*dist];
                match mode {
                    FloorMode::Promote => {
                        if !rows.iter().any(|row| row.rarity == *value) {
                            push(
                                issues,
                                file,
                                Phase::Guarantee,
                                *keyword,
                                "E_GUARANTEE_EFFECT",
                                "门槛稀有度行",
                                "缺少门槛行",
                            );
                        }
                    }
                    FloorMode::Rescale => {
                        if always_zero(rows, *value) {
                            push(
                                issues,
                                file,
                                Phase::Guarantee,
                                *keyword,
                                "E_FLOOR_ZERO",
                                "门槛及以上的正概率",
                                "重抽质量恒为零",
                            );
                        }
                    }
                }
            }
            Some(match mode {
                FloorMode::Promote => EffectRt::FloorPromote(*value),
                FloorMode::Rescale => EffectRt::FloorRescale(*value),
            })
        }
    };
    if let Some(effect) = effect {
        rule.guarantees.push(Guar {
            name: guarantee.name.name.clone(),
            when,
            effect,
        });
    }
}

fn always_zero(rows: &[RowView], k: u128) -> bool {
    let relevant: Vec<&RowView> = rows.iter().filter(|row| row.rarity >= k).collect();
    if relevant.is_empty() {
        return true;
    }
    let mut fixed_max = Rational::zero();
    let mut other_min = Rational::zero();
    let mut has_rest = false;
    let mut weight_hit = Rational::zero();
    let mut weight_all = Rational::zero();
    for row in rows {
        if let RowAmount::Weight(value) = &row.amount {
            weight_all = weight_all.checked_add(*value).expect("权重");
        }
    }
    for row in rows {
        match &row.amount {
            RowAmount::Fixed(node) => {
                let (min, max) = chance_bounds(node);
                if row.rarity >= k {
                    fixed_max = fixed_max.checked_add(max).expect("概率");
                } else {
                    other_min = other_min.checked_add(min).expect("概率");
                }
            }
            RowAmount::Rest if row.rarity >= k => has_rest = true,
            RowAmount::Weight(value) if row.rarity >= k => {
                weight_hit = weight_hit.checked_add(*value).expect("权重");
            }
            _ => {}
        }
    }
    let mut zmax = fixed_max;
    let remainder_max = Rational::one()
        .checked_sub(other_min)
        .unwrap_or(Rational::zero());
    if has_rest {
        zmax = zmax.checked_add(remainder_max).expect("概率");
    } else if !weight_hit.is_zero() && !weight_all.is_zero() {
        let share = weight_hit.checked_div(weight_all).expect("权重");
        zmax = zmax
            .checked_add(remainder_max.checked_mul(share).expect("概率"))
            .expect("概率");
    }
    zmax.is_zero()
}

fn lower_update(file: &str, issues: &mut Vec<Issue>, rule: &mut RuleBuild, update: &Update) {
    let Some(when) = lower_cond(file, issues, rule, &update.cond, false) else {
        return;
    };
    let mut ops = Vec::new();
    for op in &update.ops {
        match op {
            Op::SetBool { name, value } => match resolve(file, issues, rule, name) {
                Some(Kind::Bool) => ops.push(OpRt::SetBool(name.name.clone(), *value)),
                Some(other) => push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Bool 状态",
                    &kind_text(&other),
                ),
                None => {}
            },
            Op::Add {
                name,
                delta,
                delta_span,
            } => match resolve(file, issues, rule, name) {
                Some(Kind::Counter { upper, .. }) => {
                    if *delta == 0 {
                        push(
                            issues,
                            file,
                            Phase::Counter,
                            *delta_span,
                            "E_COUNTER_INIT",
                            "正增量",
                            "0c",
                        );
                    } else if upper.is_some_and(|limit| *delta > limit) {
                        push(
                            issues,
                            file,
                            Phase::Counter,
                            *delta_span,
                            "E_ALWAYS_SATURATED",
                            "可能改变计数的增量",
                            "必然饱和",
                        );
                    } else {
                        ops.push(OpRt::Add(name.name.clone(), *delta));
                    }
                }
                Some(other) => push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Counter",
                    &kind_text(&other),
                ),
                None => {}
            },
            Op::Reset { name } => match resolve(file, issues, rule, name) {
                Some(Kind::Counter { .. }) => ops.push(OpRt::Reset(name.name.clone())),
                Some(other) => push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Counter",
                    &kind_text(&other),
                ),
                None => {}
            },
            Op::Emit { name, args } => match resolve(file, issues, rule, name) {
                Some(Kind::Event(params)) => {
                    if params.len() != args.len() {
                        push(
                            issues,
                            file,
                            Phase::Type,
                            name.span,
                            "E_EVENT_SIGNATURE",
                            &format!("{} 个参数", params.len()),
                            &format!("{} 个参数", args.len()),
                        );
                    } else {
                        let mut ir = Vec::new();
                        let mut ok = true;
                        for (param, arg) in params.iter().zip(args) {
                            match lower_arg(file, issues, rule, *param, arg) {
                                Some(value) => ir.push(value),
                                None => ok = false,
                            }
                        }
                        if ok {
                            ops.push(OpRt::Emit(name.name.clone(), ir));
                        }
                    }
                }
                Some(other) => push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Event",
                    &kind_text(&other),
                ),
                None => {}
            },
        }
    }
    rule.updates.push(UpdateRt { when, ops });
}

fn lower_arg(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    param: ParamKind,
    arg: &Arg,
) -> Option<ArgRt> {
    match arg {
        Arg::Count(_, span) => {
            if param == ParamKind::Count {
                if let Arg::Count(value, _) = arg {
                    return Some(ArgRt::Count(*value));
                }
            }
            push(
                issues,
                file,
                Phase::Type,
                *span,
                "E_EVENT_SIGNATURE",
                param_text(param),
                "Count 字面量",
            );
            None
        }
        Arg::Rarity(_, span) => {
            if param == ParamKind::Rarity {
                if let Arg::Rarity(value, _) = arg {
                    return Some(ArgRt::Rarity(*value));
                }
            }
            push(
                issues,
                file,
                Phase::Type,
                *span,
                "E_EVENT_SIGNATURE",
                param_text(param),
                "Rarity 字面量",
            );
            None
        }
        Arg::Bool(_, span) => {
            if param == ParamKind::Bool {
                if let Arg::Bool(value, _) = arg {
                    return Some(ArgRt::Bool(*value));
                }
            }
            push(
                issues,
                file,
                Phase::Type,
                *span,
                "E_EVENT_SIGNATURE",
                param_text(param),
                "Bool",
            );
            None
        }
        Arg::Result(span) => {
            if param == ParamKind::Item {
                Some(ArgRt::Result)
            } else {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *span,
                    "E_EVENT_SIGNATURE",
                    param_text(param),
                    "Item",
                );
                None
            }
        }
        Arg::Grade(span) => {
            if param == ParamKind::Rarity {
                Some(ArgRt::Grade)
            } else {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *span,
                    "E_EVENT_SIGNATURE",
                    param_text(param),
                    "Rarity",
                );
                None
            }
        }
        Arg::Name(name) => match resolve(file, issues, rule, name) {
            Some(Kind::Counter { .. }) if param == ParamKind::Count => {
                Some(ArgRt::CountState(name.name.clone()))
            }
            Some(Kind::Bool) if param == ParamKind::Bool => {
                Some(ArgRt::BoolState(name.name.clone()))
            }
            Some(Kind::Item { .. }) if param == ParamKind::Item => {
                Some(ArgRt::ItemSlot(name.name.clone()))
            }
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_EVENT_SIGNATURE",
                    param_text(param),
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
    }
}

fn lower_cond(
    file: &str,
    issues: &mut Vec<Issue>,
    rule: &mut RuleBuild,
    cond: &Cond,
    pre: bool,
) -> Option<CondRt> {
    match cond {
        Cond::BoolLit(value, _) => Some(CondRt::Bool(*value)),
        Cond::Not(inner) => Some(CondRt::Not(Box::new(lower_cond(
            file, issues, rule, inner, pre,
        )?))),
        Cond::And(left, right) => Some(CondRt::And(
            Box::new(lower_cond(file, issues, rule, left, pre)?),
            Box::new(lower_cond(file, issues, rule, right, pre)?),
        )),
        Cond::Or(left, right) => Some(CondRt::Or(
            Box::new(lower_cond(file, issues, rule, left, pre)?),
            Box::new(lower_cond(file, issues, rule, right, pre)?),
        )),
        Cond::GradeEq { result, value } | Cond::GradeGe { result, value } => {
            if pre {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *result,
                    "E_RESULT_PHASE",
                    "抽前状态",
                    "result",
                );
                None
            } else if matches!(cond, Cond::GradeEq { .. }) {
                Some(CondRt::GradeEq(*value))
            } else {
                Some(CondRt::GradeGe(*value))
            }
        }
        Cond::InPool { result, pool } => {
            if pre {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *result,
                    "E_RESULT_PHASE",
                    "抽前状态",
                    "result",
                );
                return None;
            }
            match resolve(file, issues, rule, pool) {
                Some(Kind::Pool { .. }) => Some(CondRt::InPool(pool.name.clone())),
                Some(other) => {
                    push(
                        issues,
                        file,
                        Phase::Type,
                        pool.span,
                        "E_TYPE",
                        "Pool",
                        &kind_text(&other),
                    );
                    None
                }
                None => None,
            }
        }
        Cond::CountEq { name, value, .. } => match resolve(file, issues, rule, name) {
            Some(Kind::Counter { .. }) => Some(CondRt::CountEq(name.name.clone(), *value)),
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Counter",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        Cond::CountGe { name, value, .. } => match resolve(file, issues, rule, name) {
            Some(Kind::Counter { .. }) => Some(CondRt::CountGe(name.name.clone(), *value)),
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Counter",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
        Cond::CountMod {
            name,
            modulus,
            modulus_span,
            remainder,
            remainder_span,
        } => {
            let kind = resolve(file, issues, rule, name);
            if let Some(other) = &kind {
                if !matches!(other, Kind::Counter { .. }) {
                    push(
                        issues,
                        file,
                        Phase::Type,
                        name.span,
                        "E_TYPE",
                        "Counter",
                        &kind_text(other),
                    );
                    return None;
                }
            } else {
                return None;
            }
            if *modulus == 0 {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *modulus_span,
                    "E_MODULUS",
                    "正模数",
                    "0c",
                );
                None
            } else if *remainder >= *modulus {
                push(
                    issues,
                    file,
                    Phase::Type,
                    *remainder_span,
                    "E_MODULUS",
                    "小于模数的余数",
                    "余数越界",
                );
                None
            } else {
                Some(CondRt::CountMod(name.name.clone(), *modulus, *remainder))
            }
        }
        Cond::BoolEq { name, value, .. } => match resolve(file, issues, rule, name) {
            Some(Kind::Bool) => Some(CondRt::BoolEq(name.name.clone(), *value)),
            Some(other) => {
                push(
                    issues,
                    file,
                    Phase::Type,
                    name.span,
                    "E_TYPE",
                    "Bool",
                    &kind_text(&other),
                );
                None
            }
            None => None,
        },
    }
}

fn resolve(file: &str, issues: &mut Vec<Issue>, rule: &RuleBuild, name: &Ident) -> Option<Kind> {
    let Some(sym) = rule.syms.get(&name.name) else {
        push(
            issues,
            file,
            Phase::Name,
            name.span,
            "E_NAME_UNKNOWN",
            "已声明的名字",
            &name.name,
        );
        return None;
    };
    let use_at = at(name);
    let is_selector = matches!(sym.kind, Kind::Selector { .. });
    if sym.name_at < use_at && use_at <= sym.rhs_end {
        if is_selector {
            push(
                issues,
                file,
                Phase::Name,
                name.span,
                "E_SELECTOR_CYCLE",
                "非自指选择器",
                &name.name,
            );
        } else {
            push(
                issues,
                file,
                Phase::Name,
                name.span,
                "E_NAME_UNKNOWN",
                "已声明的名字",
                &name.name,
            );
        }
        return None;
    }
    if use_at < sym.name_at {
        push(
            issues,
            file,
            Phase::Name,
            name.span,
            "E_NAME_BEFORE_DECL",
            "先前已声明的名字",
            "稍后才声明的名字",
        );
        return None;
    }
    Some(sym.kind.clone())
}

struct Schema {
    count: Option<CountRt>,
    bool_initial: Option<bool>,
}

fn bind_banner(
    file: &str,
    banner: &Banner,
    rules: &BTreeMap<String, RuleBuild>,
    rosters: &BTreeMap<String, RosterBuild>,
    schemas: &mut BTreeMap<(u8, String, String), Schema>,
    issues: &mut Vec<Issue>,
) -> Option<BannerRt> {
    let Some(rule) = rules.get(&banner.source_rule.name) else {
        push(
            issues,
            file,
            Phase::Bind,
            banner.source_rule.span,
            "E_BIND_REF",
            "已声明的 rule",
            &banner.source_rule.name,
        );
        return None;
    };
    let Some(roster) = rosters.get(&banner.source_roster.name) else {
        push(
            issues,
            file,
            Phase::Bind,
            banner.source_roster.span,
            "E_BIND_REF",
            "已声明的 roster",
            &banner.source_roster.name,
        );
        return None;
    };
    if rule.has_pity && banner.pity_key.is_none() {
        push(
            issues,
            file,
            Phase::Bind,
            banner.keyword,
            "E_BIND_REF",
            "pity_key",
            "缺少 pity_key",
        );
    }
    if let Some(key) = banner.pity_key.as_ref().filter(|_| !rule.has_pity) {
        push(
            issues,
            file,
            Phase::Bind,
            key.keyword,
            "E_BIND_REF",
            "无 pity 状态",
            "pity_key",
        );
    }
    if rule.has_progress && banner.progress_key.is_none() {
        push(
            issues,
            file,
            Phase::Bind,
            banner.keyword,
            "E_BIND_REF",
            "progress_key",
            "缺少 progress_key",
        );
    }
    if let Some(key) = banner.progress_key.as_ref().filter(|_| !rule.has_progress) {
        push(
            issues,
            file,
            Phase::Bind,
            key.keyword,
            "E_BIND_REF",
            "无 progress 状态",
            "progress_key",
        );
    }

    let mut bound = BTreeSet::new();
    let mut unresolved = false;
    let mut pools = Vec::new();
    let mut items = Vec::new();
    let mut entry_rarity = BTreeMap::new();
    let mut entry_tags = BTreeMap::new();
    for (name, info) in &roster.entries {
        entry_rarity.insert(name.clone(), info.rarity);
        entry_tags.insert(name.clone(), info.tags.clone());
    }
    for bind in &banner.binds {
        if !bound.insert(bind.name.name.clone()) {
            push(
                issues,
                file,
                Phase::Bind,
                bind.name.span,
                "E_BIND_REF",
                "一次绑定",
                &bind.name.name,
            );
            continue;
        }
        let Some(sym) = rule.syms.get(&bind.name.name) else {
            unresolved = true;
            push(
                issues,
                file,
                Phase::Bind,
                bind.name.span,
                "E_BIND_REF",
                "规则中的 Pool 或 Item",
                &bind.name.name,
            );
            continue;
        };
        match &sym.kind {
            Kind::Pool { rarity } => {
                if bind.ty.kind != BindKind::Pool || bind.ty.rarity != *rarity {
                    push(
                        issues,
                        file,
                        Phase::Bind,
                        bind.ty.span,
                        "E_BIND_TYPE",
                        &format!("Pool[{rarity}r]"),
                        "绑定类型",
                    );
                }
                match &bind.rhs {
                    BindRhs::Members { names, bracket } => {
                        if names.is_empty() {
                            push(
                                issues,
                                file,
                                Phase::Bind,
                                *bracket,
                                "E_SELECTOR_EMPTY",
                                "非空池",
                                "[]",
                            );
                        }
                        let mut seen = BTreeSet::new();
                        let mut members = Vec::new();
                        for entry in names {
                            if !seen.insert(entry.name.clone()) {
                                push(
                                    issues,
                                    file,
                                    Phase::Bind,
                                    entry.span,
                                    "E_BIND_TYPE",
                                    "不重复的池成员",
                                    &entry.name,
                                );
                                continue;
                            }
                            match roster.entries.get(&entry.name) {
                                None => push(
                                    issues,
                                    file,
                                    Phase::Bind,
                                    entry.span,
                                    "E_ENTRY_UNKNOWN",
                                    "名册条目",
                                    &entry.name,
                                ),
                                Some(info) if !info.drawable => push(
                                    issues,
                                    file,
                                    Phase::Bind,
                                    entry.span,
                                    "E_ENTRY_DISABLED",
                                    "可抽取条目",
                                    &entry.name,
                                ),
                                Some(info) if info.rarity != *rarity => push(
                                    issues,
                                    file,
                                    Phase::Bind,
                                    entry.span,
                                    "E_RARITY_MISMATCH",
                                    &format!("{rarity}r"),
                                    &format!("{}r", info.rarity),
                                ),
                                Some(_) => members.push(entry.name.clone()),
                            }
                        }
                        pools.push((bind.name.name.clone(), members));
                    }
                    BindRhs::Entry(entry) => {
                        push(
                            issues,
                            file,
                            Phase::Bind,
                            entry.span,
                            "E_BIND_TYPE",
                            "池成员列表",
                            "条目",
                        );
                    }
                }
            }
            Kind::Item { rarity } => {
                if bind.ty.kind != BindKind::Item || bind.ty.rarity != *rarity {
                    push(
                        issues,
                        file,
                        Phase::Bind,
                        bind.ty.span,
                        "E_BIND_TYPE",
                        &format!("Item[{rarity}r]"),
                        "绑定类型",
                    );
                }
                match &bind.rhs {
                    BindRhs::Entry(entry) => match roster.entries.get(&entry.name) {
                        None => push(
                            issues,
                            file,
                            Phase::Bind,
                            entry.span,
                            "E_ENTRY_UNKNOWN",
                            "名册条目",
                            &entry.name,
                        ),
                        Some(info) if !info.drawable => push(
                            issues,
                            file,
                            Phase::Bind,
                            entry.span,
                            "E_ENTRY_DISABLED",
                            "可抽取条目",
                            &entry.name,
                        ),
                        Some(info) if info.rarity != *rarity => push(
                            issues,
                            file,
                            Phase::Bind,
                            entry.span,
                            "E_RARITY_MISMATCH",
                            &format!("{rarity}r"),
                            &format!("{}r", info.rarity),
                        ),
                        Some(_) => items.push((bind.name.name.clone(), entry.name.clone())),
                    },
                    BindRhs::Members { bracket, .. } => {
                        push(
                            issues,
                            file,
                            Phase::Bind,
                            *bracket,
                            "E_BIND_TYPE",
                            "单个条目",
                            "列表",
                        );
                    }
                }
            }
            other => push(
                issues,
                file,
                Phase::Bind,
                bind.name.span,
                "E_BIND_TYPE",
                "Pool 或 Item",
                &kind_text(other),
            ),
        }
    }
    if !unresolved {
        for name in rule.pools.iter().chain(rule.items.iter()) {
            if !bound.contains(name) {
                push(
                    issues,
                    file,
                    Phase::Bind,
                    banner.keyword,
                    "E_BIND_REF",
                    "完整绑定",
                    name,
                );
            }
        }
    }
    record_schema(file, issues, banner, rule, schemas);
    Some(BannerRt {
        name: banner.name.name.clone(),
        pity_key: banner.pity_key.as_ref().map(|k| k.value.clone()),
        progress_key: banner.progress_key.as_ref().map(|k| k.value.clone()),
        states: rule.states.clone(),
        pools,
        items,
        entry_rarity: entry_rarity.into_iter().collect(),
        entry_tags,
        selectors: rule.selectors.clone(),
        distributions: rule.distributions.clone(),
        actions: rule.actions.clone(),
        guarantees: rule.guarantees.clone(),
        updates: rule.updates.clone(),
    })
}

fn record_schema(
    file: &str,
    issues: &mut Vec<Issue>,
    banner: &Banner,
    rule: &RuleBuild,
    schemas: &mut BTreeMap<(u8, String, String), Schema>,
) {
    for state in &rule.states {
        let (tag, key_span, key) = match state.scope {
            Scope::Pity => {
                let Some(key) = &banner.pity_key else {
                    continue;
                };
                (1u8, key.keyword, key.value.clone())
            }
            Scope::Progress => {
                let Some(key) = &banner.progress_key else {
                    continue;
                };
                (2u8, key.keyword, key.value.clone())
            }
            _ => continue,
        };
        let schema = Schema {
            count: state.count.clone(),
            bool_initial: state.bool_initial,
        };
        let map_key = (tag, key, state.name.clone());
        if let Some(previous) = schemas.get(&map_key) {
            let same = previous.count.as_ref().map(|c| (c.initial, c.upper))
                == schema.count.as_ref().map(|c| (c.initial, c.upper))
                && previous.bool_initial == schema.bool_initial;
            if !same {
                push(
                    issues,
                    file,
                    Phase::Global,
                    key_span,
                    "E_SHARED_SLOT_SCHEMA",
                    "相同的状态声明",
                    "共享槽不一致",
                );
            }
        } else {
            schemas.insert(map_key, schema);
        }
    }
}

fn chance_bounds(node: &ChanceRt) -> (Rational, Rational) {
    match node {
        ChanceRt::Const(value) => (*value, *value),
        ChanceRt::Linear {
            start,
            base,
            step,
            cap,
            upper,
            ..
        } => {
            let low = linear_at(0, *start, *base, *step, *cap);
            let high = match upper {
                Some(limit) => linear_at(*limit, *start, *base, *step, *cap),
                None => *cap,
            };
            (low, high)
        }
    }
}

fn linear_at(count: u128, start: u128, base: Rational, step: Rational, cap: Rational) -> Rational {
    let steps = if count < start { 0 } else { count - start + 1 };
    let raw = base
        .checked_add(step.checked_mul(Rational::new(steps, 1)).expect("步长"))
        .expect("曲线");
    raw.min(cap)
}

fn chance_span(expr: &ChanceExpr) -> Span {
    match expr {
        ChanceExpr::Lit(_, span) => *span,
        ChanceExpr::Name(name) => name.span,
        ChanceExpr::Linear { counter, .. } => counter.span,
    }
}

fn chance_end(name: &Ident, expr: &ChanceExpr) -> (u32, u32) {
    match expr {
        ChanceExpr::Lit(_, span) => at_span(*span),
        ChanceExpr::Name(other) => at(other),
        ChanceExpr::Linear { cap_span, .. } => at_span(*cap_span),
    }
    .max(at(name))
}

fn selector_end(name: &Ident, rhs: &SelectorRhs) -> (u32, u32) {
    fn walk(rhs: &SelectorRhs, acc: (u32, u32)) -> (u32, u32) {
        match rhs {
            SelectorRhs::From(name) | SelectorRhs::Name(name) => acc.max(at(name)),
            SelectorRhs::Choose { cond, yes, no } => walk(no, walk(yes, cond_end(cond, acc))),
            SelectorRhs::Weighted { rows, colon } => {
                let mut acc = acc.max(at_span(*colon));
                for row in rows {
                    acc = walk(&row.child, acc.max(at_span(row.weight_span)));
                }
                acc
            }
        }
    }
    fn cond_end(cond: &Cond, acc: (u32, u32)) -> (u32, u32) {
        match cond {
            Cond::BoolLit(_, span) => acc.max(at_span(*span)),
            Cond::CountEq {
                name, value_span, ..
            }
            | Cond::CountGe {
                name, value_span, ..
            } => acc.max(at(name)).max(at_span(*value_span)),
            Cond::CountMod {
                name,
                modulus_span,
                remainder_span,
                ..
            } => acc
                .max(at(name))
                .max(at_span(*modulus_span))
                .max(at_span(*remainder_span)),
            Cond::BoolEq {
                name, value_span, ..
            } => acc.max(at(name)).max(at_span(*value_span)),
            Cond::GradeEq { result, .. } | Cond::GradeGe { result, .. } => {
                acc.max(at_span(*result))
            }
            Cond::InPool { result, pool } => acc.max(at_span(*result)).max(at(pool)),
            Cond::Not(inner) => cond_end(inner, acc),
            Cond::And(left, right) | Cond::Or(left, right) => cond_end(right, cond_end(left, acc)),
        }
    }
    walk(rhs, at(name))
}

fn at(name: &Ident) -> (u32, u32) {
    (name.span.line, name.span.column)
}

fn at_span(span: Span) -> (u32, u32) {
    (span.line, span.column)
}

fn kind_text(kind: &Kind) -> String {
    match kind {
        Kind::Counter { .. } => "Counter".to_string(),
        Kind::Bool => "Bool".to_string(),
        Kind::Pool { rarity } => format!("Pool[{rarity}r]"),
        Kind::Item { rarity } => format!("Item[{rarity}r]"),
        Kind::Event(_) => "Event".to_string(),
        Kind::Chance(_) => "Chance".to_string(),
        Kind::Selector { rarity, .. } => format!("Selector[{rarity}r]"),
        Kind::Distribution(_) => "Distribution".to_string(),
        Kind::Action => "Action".to_string(),
        Kind::Guarantee => "Guarantee".to_string(),
    }
}

fn param_text(kind: ParamKind) -> &'static str {
    match kind {
        ParamKind::Count => "Count",
        ParamKind::Bool => "Bool",
        ParamKind::Rarity => "Rarity",
        ParamKind::Item => "Item",
    }
}

fn push(
    issues: &mut Vec<Issue>,
    file: &str,
    phase: Phase,
    span: Span,
    code: &'static str,
    expected: &str,
    actual: &str,
) {
    issues.push(Issue {
        phase,
        file: span.file,
        line: span.line,
        column: span.column,
        diag: diag(code, file, span, expected, actual),
    });
}
