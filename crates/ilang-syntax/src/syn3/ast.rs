//! The syntax tree of syntax three (docs/design/syntax-3.md §3, with the
//! services amendment: `<-` in types, heads and calls, `<A>(…)` generics).
//!
//! Every node keeps its span in an [`Sp`], which never takes part in
//! equality, so `parse(print(t)) == t` compares trees and not positions.

use crate::lisp::Datum;
use crate::span::Span;
use num_bigint::BigUint;

pub use crate::ast::{BinOp, LitValue, UnOp};

/// A span that is equal to every other span.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sp(pub Span);

impl PartialEq for Sp {
    fn eq(&self, _: &Sp) -> bool {
        true
    }
}

impl From<Span> for Sp {
    fn from(span: Span) -> Sp {
        Sp(span)
    }
}

/// A quoted datum. Two are equal when they are written the same.
#[derive(Clone, Debug)]
pub struct Quoted(pub Box<Datum>);

impl PartialEq for Quoted {
    fn eq(&self, other: &Quoted) -> bool {
        crate::lisp::show(&self.0) == crate::lisp::show(&other.0)
    }
}

/// A quote pattern of a compile-time macro: the quoted datum, the slot of
/// its template in the table of the compiler program, and the patterns of
/// its unquotes.
#[derive(Clone, Debug, PartialEq)]
pub struct QuotePattern {
    pub datum: Quoted,
    /// Template table slot, filled when a compiler program is prepared
    /// (`lisp::quote_lower3`).
    pub template: Option<u32>,
    /// Capture patterns in matcher order; the flag marks a splice.
    pub captures: Vec<(bool, Pattern)>,
}

impl QuotePattern {
    /// A quote pattern whose captures were not read.
    pub fn unread(datum: Datum) -> QuotePattern {
        QuotePattern {
            datum: Quoted(Box::new(datum)),
            template: None,
            captures: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Sp,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Ident {
        Ident {
            name: name.into(),
            span: Sp(span),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Module {
    /// The `////` comments of the file, one entry per line.
    pub doc: Vec<String>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub items: Vec<Item>,
}

/// `[alias =] [(A, b) =] a/b/c`.
#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub alias: Option<Ident>,
    pub names: Vec<Ident>,
    pub path: Vec<Ident>,
    pub span: Sp,
}

/// `name`, `T`, or `T(..)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub name: Ident,
    pub constructors: bool,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The `///` comments before the item, one entry per line.
    pub doc: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub kind: ItemKind,
    pub span: Sp,
}

/// `@[module.]name[(data)]`. `args` is `None` when there are no brackets.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub module: Option<Ident>,
    pub name: Ident,
    pub args: Option<Vec<DItem>>,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    TypeDef(TypeDef),
    Bind(Binding),
    /// `name: Type` with no value (`@external`, `@js`).
    Decl {
        name: Ident,
        ty: Type,
    },
    /// `[module.]name(data)` at the top level, kept for macros.
    Macro {
        module: Option<Ident>,
        name: Ident,
        args: Vec<DItem>,
    },
}

/// `Name [ (head) ] = right side` (§2.8). The head is read as type patterns;
/// the kind follows from the written right side.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub name: Ident,
    pub head: Vec<Type>,
    pub rhs: TypeRhs,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeRhs {
    /// `T = Opaque`.
    Opaque,
    /// `T = (items)`: a labelled type with data, its own constructor.
    Data(Vec<TItem>),
    /// Anything else: a name (alias), or an alias pattern when the type is a
    /// value type. A data type here is printed in a group, `{ (A, B) }`.
    Type(Type),
}

/// `Pattern = Expr`.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub pattern: Pattern,
    pub value: Expr,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Bind(Binding),
    Expr(Expr),
}

/// `{ s1 … sn e }`.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    /// A Lisp template (Lisp surface only).
    Quote(Quoted),
    Int(BigUint),
    Float(f64),
    String(String),
    /// A string with `\( … )` parts.
    Interp(Vec<InterpPart>),
    /// `x`, `C`, or `m.C`. `m.x` is a `Field`.
    Name {
        module: Option<Ident>,
        name: Ident,
    },
    /// `( items )`: data.
    Data(Vec<DItem>),
    List {
        items: Vec<Expr>,
        rest: Option<Box<Expr>>,
    },
    BitArray(Vec<Segment<Expr>>),
    Block(Block),
    /// `F1 | F2 | …`, one or more function literals (§2.5).
    Chain(Vec<Function>),
    /// `callee(items)`: the argument is data.
    Call {
        callee: Box<Expr>,
        args: Vec<DItem>,
    },
    /// `f(x) <- (s)` and `{ … } <- (s)`: the target runs with the services
    /// of the right side added to its environment.
    Provide {
        target: Box<Expr>,
        services: Box<Expr>,
    },
    Field {
        target: Box<Expr>,
        name: Ident,
    },
    /// `d.0`.
    Index {
        target: Box<Expr>,
        index: BigUint,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    /// Never `BinOp::Pipe`.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `a |> f`, `a |> f(b)`, and the match `a |> F1 | F2` (§2.11).
    Pipe {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum InterpPart {
    Text(String),
    Expr(Expr),
}

/// An item of a data value or of a call's data.
#[derive(Clone, Debug, PartialEq)]
pub enum DItem {
    Pos(Expr),
    Label(Ident, Expr),
    Spread(Expr),
}

/// `[<A, …>] (params [? guard]) [<- (services)] [-> R] { body }`.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub generics: Vec<Ident>,
    pub params: Vec<PItem>,
    pub guard: Option<Expr>,
    /// `None` when there is no `<-` part.
    pub services: Option<Vec<SItem>>,
    pub ret: Option<Type>,
    pub body: Block,
    pub span: Sp,
}

/// An item of a parameter group: `[key =] p1 | p2 | …`.
#[derive(Clone, Debug, PartialEq)]
pub struct PItem {
    pub key: Option<Ident>,
    pub choices: Vec<Pattern>,
}

/// An item of the service pattern of a function head: `name: T` binds the
/// service whose key is the named type `T`; `..name: N`, the last item,
/// binds the whole environment as one value of the row type `N`.
#[derive(Clone, Debug, PartialEq)]
pub struct SItem {
    pub rest: bool,
    pub name: Ident,
    pub ty: Type,
}

/// A part of a bit array.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment<T> {
    pub value: T,
    pub format: Option<Format>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Format {
    pub name: Ident,
    pub arg: Option<Box<Expr>>,
}

/// A name or a discard.
#[derive(Clone, Debug, PartialEq)]
pub struct Binder {
    pub name: String,
    pub discard: bool,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    /// A Lisp quote pattern (Lisp surface only).
    Quote(QuotePattern),
    Var(String),
    Discard(String),
    /// `x: T`.
    Typed {
        binder: Binder,
        ty: Type,
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
    /// A string with a fixed prefix and suffix around one binder.
    Prefix {
        prefix: String,
        suffix: String,
        rest: Binder,
    },
    /// `C`, `m.C(items)`. `args` is `None` when there are no brackets.
    Ctor {
        module: Option<Ident>,
        name: Ident,
        args: Option<Vec<DPItem>>,
    },
    /// `( items )`.
    Data(Vec<DPItem>),
    /// `[p, …]`, `[p, …, ..]`, `[p, …, ..r]`.
    List {
        items: Vec<Pattern>,
        rest: Option<Option<Binder>>,
    },
    BitArray(Vec<Segment<Pattern>>),
    /// `p1 = p2 = …`, two or more parts that match the same value.
    And(Vec<Pattern>),
}

/// An item of a data pattern or of a constructor pattern.
#[derive(Clone, Debug, PartialEq)]
pub enum DPItem {
    Pos(Pattern),
    Key(Ident, Pattern),
    /// `..`, `..name`, `.._`; the last item.
    Rest(Option<Binder>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Sp,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Lit(LitValue),
    Named {
        module: Option<Ident>,
        name: Ident,
        args: Vec<Type>,
    },
    /// `name` or `a.x`: the type of a value.
    ValueOf {
        module: Option<Ident>,
        name: Ident,
    },
    /// `[a.]m(args)` with a lowercase `m`, kept for macros.
    Call {
        module: Option<Ident>,
        name: Ident,
        args: Vec<Type>,
    },
    /// `_`.
    Hole,
    /// `( items )`.
    Data(Vec<TItem>),
    Fn(Box<FnType>),
    Union(Vec<Type>),
    Inter(Vec<Type>),
    Diff(Vec<Type>),
}

/// `[<A, …>] P [<- Needs] -> R`. `param` is a data type, or any other type,
/// which is then written in a group: `{ (Float) | (Float, Float) } -> Float`.
/// `services` (the needs) is a data type whose items are named types and at
/// most one spread `..N`, or one named type `N`, a row variable.
#[derive(Clone, Debug, PartialEq)]
pub struct FnType {
    pub generics: Vec<Ident>,
    pub param: Type,
    pub services: Option<Type>,
    pub ret: Type,
}

/// An item of a data type. `Spread` is only legal after `<-`.
#[derive(Clone, Debug, PartialEq)]
pub enum TItem {
    Pos(Type),
    Label(Ident, Type),
    Spread(Type),
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Expr {
        Expr {
            kind,
            span: Sp(span),
        }
    }
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Pattern {
        Pattern {
            kind,
            span: Sp(span),
        }
    }
}

impl Type {
    pub fn new(kind: TypeKind, span: Span) -> Type {
        Type {
            kind,
            span: Sp(span),
        }
    }
}
