use crate::diagnostic::Span;
use crate::lex::Scope;
use crate::rational::Rational;

#[derive(Clone, Debug)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub keyword: Span,
    pub name: Ident,
    pub decls: Vec<Decl>,
    pub guarantees: Vec<Guarantee>,
    pub updates: Vec<Update>,
}

#[derive(Clone, Debug)]
pub struct Roster {
    #[allow(dead_code)]
    pub keyword: Span,
    pub name: Ident,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: Ident,
    pub rarity: u128,
    #[allow(dead_code)]
    pub rarity_span: Span,
    #[allow(dead_code)]
    pub display: String,
    pub tags: Vec<String>,
    pub drawable: bool,
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub keyword: Span,
    pub name: Ident,
    pub source_rule: Ident,
    pub source_roster: Ident,
    pub pity_key: Option<Keyed>,
    pub progress_key: Option<Keyed>,
    pub binds: Vec<Bind>,
}

#[derive(Clone, Debug)]
pub struct Keyed {
    pub keyword: Span,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Bind {
    pub name: Ident,
    pub ty: BindType,
    pub rhs: BindRhs,
}

#[derive(Clone, Debug)]
pub struct BindType {
    pub kind: BindKind,
    pub rarity: u128,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindKind {
    Pool,
    Item,
}

#[derive(Clone, Debug)]
pub enum BindRhs {
    Members { names: Vec<Ident>, bracket: Span },
    Entry(Ident),
}

#[derive(Clone, Debug)]
pub enum Decl {
    Counter {
        name: Ident,
        upper: Option<u128>,
        scope: Scope,
        initial: u128,
        initial_span: Span,
    },
    Bool {
        name: Ident,
        scope: Scope,
        initial: bool,
    },
    Pool {
        name: Ident,
        rarity: u128,
    },
    Item {
        name: Ident,
        rarity: u128,
    },
    Event {
        name: Ident,
        params: Vec<Param>,
    },
    Chance {
        name: Ident,
        expr: ChanceExpr,
    },
    Selector {
        name: Ident,
        rarity: u128,
        rhs: SelectorRhs,
    },
    Distribution {
        name: Ident,
        rows: Vec<DistRow>,
    },
    Action {
        name: Ident,
        lo: u128,
        lo_span: Span,
        hi: u128,
        hi_span: Span,
        target: Ident,
    },
}

#[derive(Clone, Debug)]
pub struct Param {
    pub kind: ParamKind,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Count,
    Bool,
    Rarity,
    Item,
}

#[derive(Clone, Debug)]
pub enum ChanceExpr {
    Lit(Rational, Span),
    Name(Ident),
    Linear {
        counter: Ident,
        start: u128,
        base: Rational,
        #[allow(dead_code)]
        base_span: Span,
        step: Rational,
        step_span: Span,
        cap: Rational,
        cap_span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum SelectorRhs {
    From(Ident),
    Choose {
        cond: Cond,
        yes: Box<SelectorRhs>,
        no: Box<SelectorRhs>,
    },
    Name(Ident),
    Weighted {
        colon: Span,
        rows: Vec<WeightRow>,
    },
}

#[derive(Clone, Debug)]
pub struct WeightRow {
    pub weight: Rational,
    pub weight_span: Span,
    pub child: SelectorRhs,
}

#[derive(Clone, Debug)]
pub struct DistRow {
    pub rarity: u128,
    pub rarity_span: Span,
    pub amount: Amount,
    pub selector: Ident,
}

#[derive(Clone, Debug)]
pub enum Amount {
    Chance(ChanceExpr),
    Weight(Rational, Span),
    Rest(Span),
}

#[derive(Clone, Debug)]
pub struct Guarantee {
    pub name: Ident,
    pub cond: Cond,
    pub effect: Effect,
}

#[derive(Clone, Debug)]
pub enum Effect {
    Item(Ident),
    Rarity {
        value: u128,
        span: Span,
    },
    Floor {
        keyword: Span,
        value: u128,
        mode: FloorMode,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloorMode {
    Rescale,
    Promote,
}

#[derive(Clone, Debug)]
pub struct Update {
    pub cond: Cond,
    pub ops: Vec<Op>,
}

#[derive(Clone, Debug)]
pub enum Op {
    SetBool {
        name: Ident,
        value: bool,
    },
    Add {
        name: Ident,
        delta: u128,
        delta_span: Span,
    },
    Reset {
        name: Ident,
    },
    Emit {
        name: Ident,
        args: Vec<Arg>,
    },
}

#[derive(Clone, Debug)]
pub enum Arg {
    Count(u128, Span),
    Rarity(u128, Span),
    Bool(bool, Span),
    Name(Ident),
    Result(Span),
    Grade(Span),
}

#[derive(Clone, Debug)]
pub enum Cond {
    BoolLit(bool, Span),
    CountEq {
        name: Ident,
        value: u128,
        value_span: Span,
    },
    CountGe {
        name: Ident,
        value: u128,
        value_span: Span,
    },
    CountMod {
        name: Ident,
        modulus: u128,
        modulus_span: Span,
        remainder: u128,
        remainder_span: Span,
    },
    BoolEq {
        name: Ident,
        value: bool,
        value_span: Span,
    },
    GradeEq {
        result: Span,
        value: u128,
    },
    GradeGe {
        result: Span,
        value: u128,
    },
    InPool {
        result: Span,
        pool: Ident,
    },
    Not(Box<Cond>),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
}

#[derive(Clone, Debug)]
pub enum SourceFile {
    Rule(Rule),
    Roster(Roster),
    Banners(Vec<Banner>),
}
