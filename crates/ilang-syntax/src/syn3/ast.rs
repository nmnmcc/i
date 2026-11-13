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
    Quote(Quoted),
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

// Trees produced by macros can be much deeper than the host call stack. Move
// children onto one explicit work list before dropping their owning node.
mod iterative_drop {
    use super::*;

    enum Part {
        Expr(ExprKind),
        Pattern(PatternKind),
        Type(TypeKind),
    }

    fn expr(e: &mut Expr, work: &mut Vec<Part>) {
        work.push(Part::Expr(std::mem::replace(
            &mut e.kind,
            ExprKind::Data(Vec::new()),
        )));
    }
    fn pattern(p: &mut Pattern, work: &mut Vec<Part>) {
        work.push(Part::Pattern(std::mem::replace(
            &mut p.kind,
            PatternKind::Discard(String::new()),
        )));
    }
    fn ty(t: &mut Type, work: &mut Vec<Part>) {
        work.push(Part::Type(std::mem::replace(&mut t.kind, TypeKind::Hole)));
    }
    fn format(f: &mut Option<Format>, work: &mut Vec<Part>) {
        if let Some(arg) = f.as_mut().and_then(|f| f.arg.as_mut()) {
            expr(arg, work);
        }
    }
    fn block(b: &mut Block, work: &mut Vec<Part>) {
        for stmt in &mut b.stmts {
            match stmt {
                Stmt::Bind(b) => {
                    pattern(&mut b.pattern, work);
                    expr(&mut b.value, work);
                }
                Stmt::Expr(e) => expr(e, work),
            }
        }
    }
    fn ditems(items: &mut [DItem], work: &mut Vec<Part>) {
        for i in items {
            match i {
                DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => expr(e, work),
            }
        }
    }
    fn dpitems(items: &mut [DPItem], work: &mut Vec<Part>) {
        for i in items {
            match i {
                DPItem::Pos(p) | DPItem::Key(_, p) => pattern(p, work),
                DPItem::Rest(_) => {}
            }
        }
    }
    fn titems(items: &mut [TItem], work: &mut Vec<Part>) {
        for i in items {
            match i {
                TItem::Pos(t) | TItem::Label(_, t) | TItem::Spread(t) => ty(t, work),
            }
        }
    }

    fn drain(mut work: Vec<Part>) {
        while let Some(part) = work.pop() {
            match part {
                Part::Expr(mut kind) => match &mut kind {
                    ExprKind::Quote(_)
                    | ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::String(_)
                    | ExprKind::Name { .. } => {}
                    ExprKind::Interp(parts) => {
                        for p in parts {
                            if let InterpPart::Expr(e) = p {
                                expr(e, &mut work);
                            }
                        }
                    }
                    ExprKind::Data(items) => ditems(items, &mut work),
                    ExprKind::List { items, rest } => {
                        for x in items {
                            expr(x, &mut work);
                        }
                        if let Some(x) = rest {
                            expr(x, &mut work);
                        }
                    }
                    ExprKind::BitArray(xs) => {
                        for x in xs {
                            expr(&mut x.value, &mut work);
                            format(&mut x.format, &mut work);
                        }
                    }
                    ExprKind::Block(b) => block(b, &mut work),
                    ExprKind::Chain(fs) => {
                        for f in fs {
                            for p in &mut f.params {
                                for c in &mut p.choices {
                                    pattern(c, &mut work);
                                }
                            }
                            if let Some(g) = &mut f.guard {
                                expr(g, &mut work);
                            }
                            for s in f.services.iter_mut().flatten() {
                                ty(&mut s.ty, &mut work);
                            }
                            if let Some(r) = &mut f.ret {
                                ty(r, &mut work);
                            }
                            block(&mut f.body, &mut work);
                        }
                    }
                    ExprKind::Call { callee, args } => {
                        expr(callee, &mut work);
                        ditems(args, &mut work);
                    }
                    ExprKind::Provide { target, services } => {
                        expr(target, &mut work);
                        expr(services, &mut work);
                    }
                    ExprKind::Field { target, .. }
                    | ExprKind::Index { target, .. }
                    | ExprKind::Unary { expr: target, .. } => expr(target, &mut work),
                    ExprKind::Binary { lhs, rhs, .. } | ExprKind::Pipe { lhs, rhs } => {
                        expr(lhs, &mut work);
                        expr(rhs, &mut work);
                    }
                },
                Part::Pattern(mut kind) => match &mut kind {
                    PatternKind::Quote(_)
                    | PatternKind::Var(_)
                    | PatternKind::Discard(_)
                    | PatternKind::Int { .. }
                    | PatternKind::Float { .. }
                    | PatternKind::String(_)
                    | PatternKind::Prefix { .. } => {}
                    PatternKind::Typed { ty: t, .. } => ty(t, &mut work),
                    PatternKind::Ctor { args, .. } => {
                        if let Some(args) = args {
                            dpitems(args, &mut work);
                        }
                    }
                    PatternKind::Data(items) => dpitems(items, &mut work),
                    PatternKind::List { items, .. } | PatternKind::And(items) => {
                        for p in items {
                            pattern(p, &mut work);
                        }
                    }
                    PatternKind::BitArray(xs) => {
                        for x in xs {
                            pattern(&mut x.value, &mut work);
                            format(&mut x.format, &mut work);
                        }
                    }
                },
                Part::Type(mut kind) => match &mut kind {
                    TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
                    TypeKind::Named { args, .. }
                    | TypeKind::Call { args, .. }
                    | TypeKind::Union(args)
                    | TypeKind::Inter(args)
                    | TypeKind::Diff(args) => {
                        for t in args {
                            ty(t, &mut work);
                        }
                    }
                    TypeKind::Data(items) => titems(items, &mut work),
                    TypeKind::Fn(f) => {
                        ty(&mut f.param, &mut work);
                        if let Some(s) = &mut f.services {
                            ty(s, &mut work);
                        }
                        ty(&mut f.ret, &mut work);
                    }
                },
            }
        }
    }

    impl Drop for Expr {
        fn drop(&mut self) {
            if matches!(
                &self.kind,
                ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::String(_)
                    | ExprKind::Name { .. }
                    | ExprKind::Quote(_)
            ) || matches!(&self.kind, ExprKind::Data(xs) if xs.is_empty())
            {
                return;
            }
            let mut work = Vec::new();
            expr(self, &mut work);
            drain(work);
        }
    }
    impl Drop for Pattern {
        fn drop(&mut self) {
            if matches!(
                &self.kind,
                PatternKind::Quote(_)
                    | PatternKind::Var(_)
                    | PatternKind::Discard(_)
                    | PatternKind::Int { .. }
                    | PatternKind::Float { .. }
                    | PatternKind::String(_)
                    | PatternKind::Prefix { .. }
            ) {
                return;
            }
            let mut work = Vec::new();
            pattern(self, &mut work);
            drain(work);
        }
    }
    impl Drop for Type {
        fn drop(&mut self) {
            if matches!(
                &self.kind,
                TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole
            ) {
                return;
            }
            let mut work = Vec::new();
            ty(self, &mut work);
            drain(work);
        }
    }
}
