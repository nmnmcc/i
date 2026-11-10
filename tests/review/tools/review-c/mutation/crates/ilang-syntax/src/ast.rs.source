//! The syntax tree. Every node has a span. Groups `{ e }` of one expression
//! are kept as blocks; type groups `{ T }` leave no node.

use crate::span::Span;
use num_bigint::BigUint;

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Module {
    /// The `////` comments of the file (§1.2), one entry per line.
    pub doc: Vec<String>,
    pub imports: Vec<ImportItem>,
    pub exports: Vec<ExportItem>,
    pub items: Vec<Item>,
}

/// `[alias =] [(A, b) =] a/b/c` (§2.3).
#[derive(Clone, Debug, PartialEq)]
pub struct ImportItem {
    pub alias: Option<Ident>,
    pub names: Vec<Ident>,
    pub path: Vec<Ident>,
    pub span: Span,
}

/// `name`, `T`, or `T(..)` (§2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportItem {
    pub name: Ident,
    pub constructors: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The `///` comments before the item, one entry per line.
    pub doc: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub name: Ident,
    pub args: Vec<(String, Span)>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    TypeDef(TypeDef),
    Binding(Binding),
    /// `name: Type`, only with `@external` (§2.5, §10.1).
    Declaration {
        name: Ident,
        ty: TypeExpr,
    },
}

/// `T(a, b) { C1 C2 }` or `T(a) = Type` (§4.3).
#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub body: TypeBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeBody {
    Custom(Vec<Constructor>),
    Alias(TypeExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constructor {
    pub name: Ident,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub label: Option<Ident>,
    pub ty: TypeExpr,
    pub span: Span,
}

/// `pattern = value` (§2.5, §6.10).
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub pattern: Pattern,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Bind(Binding),
    /// `! T = e` (§6.10, §9.2).
    Env {
        ty: TypeExpr,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    NegFloat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LtFloat,
    LtEqFloat,
    GtFloat,
    GtEqFloat,
    Concat,
    Pipe,
    Add,
    Sub,
    AddFloat,
    SubFloat,
    Mul,
    Div,
    Rem,
    MulFloat,
    DivFloat,
}

impl BinOp {
    /// The level of §6.9, from 1 (weakest) to 8.
    pub fn level(self) -> u8 {
        use BinOp::*;
        match self {
            Or => 1,
            And => 2,
            Eq | NotEq => 3,
            Lt | LtEq | Gt | GtEq | LtFloat | LtEqFloat | GtFloat | GtEqFloat => 4,
            Concat => 5,
            Pipe => 6,
            Add | Sub | AddFloat | SubFloat => 7,
            Mul | Div | Rem | MulFloat | DivFloat => 8,
        }
    }

    pub fn text(self) -> &'static str {
        use BinOp::*;
        match self {
            Or => "||",
            And => "&&",
            Eq => "==",
            NotEq => "!=",
            Lt => "<",
            LtEq => "<=",
            Gt => ">",
            GtEq => ">=",
            LtFloat => "<.",
            LtEqFloat => "<=.",
            GtFloat => ">.",
            GtEqFloat => ">=.",
            Concat => "<>",
            Pipe => "|>",
            Add => "+",
            Sub => "-",
            AddFloat => "+.",
            SubFloat => "-.",
            Mul => "*",
            Div => "/",
            Rem => "%",
            MulFloat => "*.",
            DivFloat => "/.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(BigUint),
    Float(f64),
    String(String),
    /// `x`, `C`, or `m.C`. `m.x` is a `Field`, because the parser does not
    /// know whether `m` is a module (§2.3).
    Name {
        module: Option<Ident>,
        name: Ident,
    },
    Tuple(Vec<Expr>),
    List {
        items: Vec<Expr>,
        rest: Option<Box<Expr>>,
    },
    BitArray(Vec<Segment<Expr>>),
    Block(Block),
    Fn(Box<Function>),
    Case {
        subject: Box<Expr>,
        clauses: Vec<Clause>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Field {
        target: Box<Expr>,
        name: Ident,
    },
    TupleIndex {
        target: Box<Expr>,
        index: BigUint,
        index_span: Span,
    },
    /// `C(..base, f: e)` (§6.7).
    RecordUpdate {
        constructor: Box<Expr>,
        base: Box<Expr>,
        fields: Vec<(Ident, Expr)>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub label: Option<Ident>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clause {
    pub patterns: Vec<Pattern>,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// `(params) -> A ? E ! N { body }` (§6.8).
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub params: Vec<Param>,
    pub ret: Option<RetType>,
    pub body: Block,
}

/// The part of a function after `->`.
#[derive(Clone, Debug, PartialEq)]
pub struct RetType {
    pub value: TypeExpr,
    pub err: Option<TypeExpr>,
    pub env: Option<TypeExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Pattern(Pattern),
    /// `name ! T` (§6.11).
    Env {
        name: Ident,
        ty: TypeExpr,
    },
}

/// A part of a bit array (§8.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Segment<T> {
    pub value: T,
    pub format: Option<Format>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Format {
    pub name: Ident,
    pub arg: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

/// A name or a discard in a place where only those may be.
#[derive(Clone, Debug, PartialEq)]
pub struct Binder {
    pub name: String,
    pub discard: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    Var(String),
    Discard(String),
    /// `x: T` (§7.1).
    Typed {
        binder: Binder,
        ty: TypeExpr,
    },
    /// `x = p`.
    Named {
        name: Ident,
        pattern: Box<Pattern>,
    },
    Int {
        negative: bool,
        value: BigUint,
    },
    Float {
        negative: bool,
        value: f64,
    },
    String(String),
    /// `"lit" <> rest`.
    Prefix {
        prefix: String,
        rest: Binder,
    },
    /// `C`, `m.C(p, …)`, `C(p, ..)`.
    Constructor {
        module: Option<Ident>,
        name: Ident,
        args: Vec<Pattern>,
        spread: bool,
    },
    Tuple(Vec<Pattern>),
    /// `[p, …]`, `[p, …, ..]`, `[p, …, ..r]`.
    List {
        items: Vec<Pattern>,
        rest: Option<Option<Binder>>,
    },
    BitArray(Vec<Segment<Pattern>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnParamType {
    pub label: Option<Ident>,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Named {
        module: Option<Ident>,
        name: Ident,
        args: Vec<TypeExpr>,
    },
    Var(String),
    /// `_`.
    Hole,
    Tuple(Vec<TypeExpr>),
    Fn {
        params: Vec<FnParamType>,
        ret: Box<TypeExpr>,
        err: Option<Box<TypeExpr>>,
        env: Option<Box<TypeExpr>>,
    },
    /// `A | B | …`, two or more members.
    Union(Vec<TypeExpr>),
    /// `A & B & …`, two or more members.
    Inter(Vec<TypeExpr>),
    /// `A - B - …`, from left to right, two or more members.
    Diff(Vec<TypeExpr>),
}
