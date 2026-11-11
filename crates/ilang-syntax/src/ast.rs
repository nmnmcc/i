//! The syntax tree. Every node has a span. Groups `{ e }` of one expression
//! are kept as blocks; type groups `{ T }` leave no node.

use crate::span::Span;
use num_bigint::{BigInt, BigUint};

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A record type label is either a literal string or a declared label parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordLabel {
    pub ident: Ident,
    pub parameter: bool,
}

impl std::ops::Deref for RecordLabel {
    type Target = Ident;
    fn deref(&self) -> &Ident {
        &self.ident
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Module {
    /// The `////` comments of the file (§1.2), one entry per line.
    pub doc: Vec<String>,
    pub imports: Vec<ImportItem>,
    pub exports: Vec<ExportItem>,
    pub items: Vec<Item>,
    /// The imports group.
    pub imports_span: Option<Span>,
    pub exports_span: Option<Span>,
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

/// `@[module.]name[(args)]` (§2.2). `args` holds the arguments when every
/// one is a string literal with no label and there is at least one; `call`
/// holds them, with the span of the `( )`, in every other case with `( )`.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub module: Option<Ident>,
    pub name: Ident,
    pub args: Vec<(String, Span)>,
    pub call: Option<(Vec<Arg>, Span)>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    TypeDef(TypeDef),
    Binding(Binding),
    /// `name[(A, …)]: Type` (§2.5, §10.1).
    Declaration {
        name: Ident,
        type_params: Vec<Ident>,
        ty: TypeExpr,
    },
    /// `[module.]name(args)` at the top level, kept for macros (§2.2).
    MacroItem {
        module: Option<Ident>,
        name: Ident,
        args: Vec<Arg>,
    },
}

/// `T(A, B) { C1 C2 }` or `T(A) = Type` (§4.3). `params` holds a head of
/// plain `UpName`s with no module and no arguments. `head` holds any other head,
/// and `params` is then empty.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    /// Frozen from the written RHS before expansion (§4.3).
    pub alias_pattern: bool,
    pub name: Ident,
    pub params: Vec<Ident>,
    pub head: Vec<TypeExpr>,
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

/// `pattern = value`, or `name(A, …)[: T] = value` (§2.5, §6.10).
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub pattern: Pattern,
    pub value: Expr,
    pub span: Span,
    /// The type variables declared after the name (§4.7).
    pub type_params: Vec<Ident>,
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
            Pipe => "|",
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
    /// A Lisp template; expanded by the compile-time macro phase.
    Quote(Box<crate::lisp::Datum>),
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
    /// A structural record, with an optional spread evaluated before its fields.
    Record {
        base: Option<Box<Expr>>,
        fields: Vec<(Ident, Expr)>,
    },
    List {
        items: Vec<Expr>,
        rest: Option<Box<Expr>>,
    },
    BitArray(Vec<Segment<Expr>>),
    Block(Block),
    Fn(Box<Function>),
    /// The match `e |> { … }` (§6.12).
    Case {
        subject: Box<Expr>,
        clauses: Vec<Clause>,
    },
    /// A clause block that is not the right side of `|>` (§6.12).
    Clauses(Vec<Clause>),
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
    /// A Lisp quote pattern used by a compile-time macro.
    Quote(Box<crate::lisp::Datum>),
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
    Lit(LitValue),
    Named {
        module: Option<Ident>,
        name: Ident,
        args: Vec<TypeExpr>,
    },
    /// `name` or `a.x` with a lowercase `x`, kept for the type of a value (§3).
    ValueOf {
        module: Option<Ident>,
        name: Ident,
    },
    /// `[a.]m(args)` with a lowercase `m`, kept for macros (§3).
    Call {
        module: Option<Ident>,
        name: Ident,
        args: Vec<TypeArg>,
    },
    /// `_`.
    Hole,
    Tuple(Vec<TypeExpr>),
    Record {
        base: Option<Box<TypeExpr>>,
        fields: Vec<(RecordLabel, TypeExpr)>,
        removed: Vec<RecordLabel>,
    },
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

/// A type-call argument is a type, including a literal type (§3).
pub type TypeArg = TypeExpr;

/// Canonical integer or decoded string value, shared by written and tested types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LitValue {
    Int(BigInt),
    Str(String),
}

impl LitValue {
    pub fn text(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Str(s) => crate::print::string_literal(s),
        }
    }
}

// Trees produced by macros can be much deeper than the host call stack. Move
// children onto one explicit worklist before dropping their owning node.
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
            ExprKind::Tuple(Vec::new()),
        )));
    }
    fn pattern(p: &mut Pattern, work: &mut Vec<Part>) {
        work.push(Part::Pattern(std::mem::replace(
            &mut p.kind,
            PatternKind::Discard(String::new()),
        )));
    }
    fn ty(t: &mut TypeExpr, work: &mut Vec<Part>) {
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
                Stmt::Env { ty: t, value, .. } => {
                    ty(t, work);
                    expr(value, work);
                }
                Stmt::Expr(e) => expr(e, work),
            }
        }
    }
    fn clauses(cs: &mut [Clause], work: &mut Vec<Part>) {
        for c in cs {
            for p in &mut c.patterns {
                pattern(p, work);
            }
            if let Some(g) = &mut c.guard {
                expr(g, work);
            }
            expr(&mut c.body, work);
        }
    }
    fn drain(mut work: Vec<Part>) {
        while let Some(part) = work.pop() {
            match part {
                Part::Expr(mut kind) => match &mut kind {
                    ExprKind::Tuple(xs) => {
                        for x in xs {
                            expr(x, &mut work);
                        }
                    }
                    ExprKind::Record { base, fields } => {
                        if let Some(base) = base {
                            expr(base, &mut work);
                        }
                        for (_, value) in fields {
                            expr(value, &mut work);
                        }
                    }
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
                    ExprKind::Fn(f) => {
                        for p in &mut f.params {
                            match p {
                                Param::Pattern(p) => pattern(p, &mut work),
                                Param::Env { ty: t, .. } => ty(t, &mut work),
                            }
                        }
                        if let Some(r) = &mut f.ret {
                            ty(&mut r.value, &mut work);
                            if let Some(t) = &mut r.err {
                                ty(t, &mut work);
                            }
                            if let Some(t) = &mut r.env {
                                ty(t, &mut work);
                            }
                        }
                        block(&mut f.body, &mut work);
                    }
                    ExprKind::Case {
                        subject,
                        clauses: cs,
                    } => {
                        expr(subject, &mut work);
                        clauses(cs, &mut work);
                    }
                    ExprKind::Clauses(cs) => clauses(cs, &mut work),
                    ExprKind::Call { callee, args } => {
                        expr(callee, &mut work);
                        for a in args {
                            expr(&mut a.value, &mut work);
                        }
                    }
                    ExprKind::Field { target, .. }
                    | ExprKind::TupleIndex { target, .. }
                    | ExprKind::Unary { expr: target, .. } => expr(target, &mut work),
                    ExprKind::RecordUpdate {
                        constructor,
                        base,
                        fields,
                    } => {
                        expr(constructor, &mut work);
                        expr(base, &mut work);
                        for (_, e) in fields {
                            expr(e, &mut work);
                        }
                    }
                    ExprKind::Binary { lhs, rhs, .. } => {
                        expr(lhs, &mut work);
                        expr(rhs, &mut work);
                    }
                    ExprKind::Quote(_)
                    | ExprKind::Int(_)
                    | ExprKind::Float(_)
                    | ExprKind::String(_)
                    | ExprKind::Name { .. } => {}
                },
                Part::Pattern(mut kind) => match &mut kind {
                    PatternKind::Typed { ty: t, .. } => ty(t, &mut work),
                    PatternKind::Named { pattern: p, .. } => pattern(p, &mut work),
                    PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                        for p in args {
                            pattern(p, &mut work);
                        }
                    }
                    PatternKind::List { items, .. } => {
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
                    PatternKind::Quote(_)
                    | PatternKind::Var(_)
                    | PatternKind::Discard(_)
                    | PatternKind::Int { .. }
                    | PatternKind::Float { .. }
                    | PatternKind::String(_)
                    | PatternKind::Prefix { .. } => {}
                },
                Part::Type(mut kind) => match &mut kind {
                    TypeKind::Named { args, .. }
                    | TypeKind::Call { args, .. }
                    | TypeKind::Tuple(args)
                    | TypeKind::Union(args)
                    | TypeKind::Inter(args)
                    | TypeKind::Diff(args) => {
                        for t in args {
                            ty(t, &mut work);
                        }
                    }
                    TypeKind::Record { base, fields, .. } => {
                        if let Some(base) = base {
                            ty(base, &mut work);
                        }
                        for (_, value) in fields {
                            ty(value, &mut work);
                        }
                    }
                    TypeKind::Fn {
                        params,
                        ret,
                        err,
                        env,
                    } => {
                        for p in params {
                            ty(&mut p.ty, &mut work);
                        }
                        ty(ret, &mut work);
                        if let Some(t) = err {
                            ty(t, &mut work);
                        }
                        if let Some(t) = env {
                            ty(t, &mut work);
                        }
                    }
                    TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
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
            ) || matches!(&self.kind, ExprKind::Tuple(xs) if xs.is_empty())
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
    impl Drop for TypeExpr {
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
