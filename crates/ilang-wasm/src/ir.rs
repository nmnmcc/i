//! Compact, platform-independent input to the Wasm instruction emitter.
//!
//! Bindings are alpha-renamed during lowering. Every name denotes one lexical
//! binding, including across alternative patterns and captured environments.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use ilang_core::{ast::BinOp, type_::Type};
use num_bigint::BigInt;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub functions: Vec<Function>,
    pub constants: BTreeMap<String, Expr>,
    pub constructors: Vec<Constructor>,
    /// Constructorless custom types cross the host boundary as opaque references.
    pub external_types: BTreeSet<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Function {
    pub symbol: String,
    pub arguments: Vec<String>,
    pub argument_types: Vec<Arc<Type>>,
    pub return_type: Arc<Type>,
    pub body: Expr,
    pub external: Option<(String, String)>,
    pub location: Location,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Constructor {
    pub symbol: String,
    pub name: String,
    /// Fully qualified name of the defining custom type.
    pub type_name: String,
    pub tag: u32,
    pub type_id: u32,
    pub arity: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub module: String,
    pub start: u32,
    pub end: u32,
    /// One-based source line. Zero denotes unavailable source coordinates.
    pub line: u32,
    /// One-based UTF-16 column, matching the frontend and language server.
    /// Zero denotes unavailable source coordinates.
    pub column: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Nil,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    Var(String),
    Function(String),
    Constant(String),
    Constructor {
        tag: u32,
        arity: usize,
    },
    Call {
        fun: Box<Expr>,
        args: Vec<Expr>,
    },
    Lambda {
        args: Vec<String>,
        captures: Vec<String>,
        body: Box<Expr>,
    },
    Block(Vec<Statement>),
    Tuple(Vec<Expr>),
    List {
        elements: Vec<Expr>,
        tail: Option<Box<Expr>>,
    },
    Record {
        tag: u32,
        args: Vec<Expr>,
    },
    Field {
        record: Box<Expr>,
        index: usize,
    },
    RecordUpdate {
        record: Box<Expr>,
        updates: Vec<(usize, Expr)>,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    NegateBool(Box<Expr>),
    NegateInt(Box<Expr>),
    Case {
        subjects: Vec<Expr>,
        clauses: Vec<Clause>,
    },
    Panic {
        message: Box<Expr>,
        location: Location,
    },
    Echo {
        value: Box<Expr>,
        message: Option<Box<Expr>>,
        location: Location,
    },
    BitArray(Vec<BitSegment<Expr>>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    Expr(Expr),
    Let {
        pattern: Pattern,
        value: Expr,
        assert: bool,
        message: Option<Expr>,
        location: Location,
    },
    Assert {
        value: Expr,
        message: Option<Expr>,
        location: Location,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Clause {
    /// Each inner vector matches all subjects, and the outer vector is an OR.
    pub alternatives: Vec<Vec<Pattern>>,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Pattern {
    Discard,
    Variable(String),
    Alias {
        name: String,
        pattern: Box<Pattern>,
    },
    Nil,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    Tuple(Vec<Pattern>),
    List {
        elements: Vec<Pattern>,
        tail: Option<Box<Pattern>>,
    },
    Record {
        tag: u32,
        arguments: Vec<Pattern>,
    },
    StringPrefix {
        prefix: String,
        left: Option<String>,
        rest: Option<String>,
    },
    BitArray(Vec<BitSegment<Pattern>>),
    /// A computed value in a bit-array segment size pattern.
    Value(Box<Expr>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BitSegment<T> {
    pub value: T,
    pub options: BitOptions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BitOptions {
    pub kind: BitKind,
    pub size: Option<Box<Expr>>,
    pub unit: u8,
    pub little_endian: bool,
    pub signed: bool,
}

impl Default for BitOptions {
    fn default() -> Self {
        Self {
            kind: BitKind::Int,
            size: None,
            unit: 1,
            little_endian: false,
            signed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BitKind {
    Int,
    Float,
    Bits,
    Bytes,
    Utf8,
    Utf16,
    Utf32,
    Utf8Codepoint,
    Utf16Codepoint,
    Utf32Codepoint,
}
