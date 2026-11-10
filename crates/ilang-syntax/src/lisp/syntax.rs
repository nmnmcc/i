//! Source-located, hygienic syntax values for compile-time code.
use std::collections::BTreeSet;
use std::sync::Arc;

use super::{Datum, DatumKind};
use crate::span::Span;
use crate::token::TokenKind;
use num_bigint::BigUint;

/// A module and the marks that determine name resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub module: u32,
    pub marks: BTreeSet<u64>,
}

/// An opaque syntax value. Equality of values is identity, not code equality.
#[derive(Clone, Debug)]
pub struct Syntax(Arc<SyntaxData>);

#[derive(Clone, Debug)]
struct SyntaxData {
    node: Node,
    scope: Scope,
    span: Span,
}

/// The contents of a syntax value; children keep their own scopes and spans.
#[derive(Clone, Debug)]
pub enum Node {
    Name(String),
    Discard(String),
    Up(String),
    Symbol(String),
    Int(BigUint),
    Float(f64),
    Text(String),
    Round(Vec<Syntax>),
    Square(Vec<Syntax>),
    Curly(Vec<Syntax>),
    Bits(Vec<Syntax>),
    Quote(Syntax),
    Unquote(Syntax),
    Splice(Syntax),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct SyntaxError {
    pub message: String,
    pub span: Span,
}

/// The value supplied by an unquote or splice during template evaluation.
#[derive(Clone, Debug)]
pub enum Substitution {
    One(Syntax),
    Many(Vec<Syntax>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    ImportItem,
    ExportItem,
    Item,
    Attribute,
    Constructor,
    Field,
    Type,
    FunctionResult,
    Parameter,
    Expression,
    Statement,
    CallArgument,
    Clause,
    ClauseHead,
    Pattern,
    Segment,
    Format,
}

/// One quote-pattern capture, paired with its written pattern syntax.
#[derive(Clone, Debug)]
pub struct Capture {
    pub pattern: Syntax,
    pub value: Substitution,
}

impl Syntax {
    pub fn from_datum(d: &Datum, scope: &Scope) -> Self {
        let children = |xs: &[Datum]| xs.iter().map(|x| Self::from_datum(x, scope)).collect();
        let node = match &d.kind {
            DatumKind::Name(s) => Node::Name(s.clone()),
            DatumKind::Discard(s) => Node::Discard(s.clone()),
            DatumKind::UpName(s) => Node::Up(s.clone()),
            DatumKind::Symbol(s) => Node::Symbol(s.clone()),
            DatumKind::Int(n) => Node::Int(n.clone()),
            DatumKind::Float(n) => Node::Float(*n),
            DatumKind::String(s) => Node::Text(s.clone()),
            DatumKind::Round(xs) => Node::Round(children(xs)),
            DatumKind::Square(xs) => Node::Square(children(xs)),
            DatumKind::Curly(xs) => Node::Curly(children(xs)),
            DatumKind::Bits(xs) => Node::Bits(children(xs)),
            DatumKind::Quote(x) => Node::Quote(Self::from_datum(x, scope)),
            DatumKind::Unquote(x) => Node::Unquote(Self::from_datum(x, scope)),
            DatumKind::Splice(x) => Node::Splice(Self::from_datum(x, scope)),
        };
        Self(Arc::new(SyntaxData {
            node,
            scope: scope.clone(),
            span: d.span,
        }))
    }
    pub fn node(&self) -> &Node {
        &self.0.node
    }
    pub fn span(&self) -> Span {
        self.0.span
    }
    pub fn scope(&self) -> &Scope {
        &self.0.scope
    }
    pub fn identical(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub fn to_datum(&self) -> Datum {
        let children = |xs: &[Syntax]| xs.iter().map(Syntax::to_datum).collect();
        let kind = match self.node() {
            Node::Name(s) => DatumKind::Name(s.clone()),
            Node::Discard(s) => DatumKind::Discard(s.clone()),
            Node::Up(s) => DatumKind::UpName(s.clone()),
            Node::Symbol(s) => DatumKind::Symbol(s.clone()),
            Node::Int(n) => DatumKind::Int(n.clone()),
            Node::Float(n) => DatumKind::Float(*n),
            Node::Text(s) => DatumKind::String(s.clone()),
            Node::Round(xs) => DatumKind::Round(children(xs)),
            Node::Square(xs) => DatumKind::Square(children(xs)),
            Node::Curly(xs) => DatumKind::Curly(children(xs)),
            Node::Bits(xs) => DatumKind::Bits(children(xs)),
            Node::Quote(x) => DatumKind::Quote(Box::new(x.to_datum())),
            Node::Unquote(x) => DatumKind::Unquote(Box::new(x.to_datum())),
            Node::Splice(x) => DatumKind::Splice(Box::new(x.to_datum())),
        };
        Datum {
            kind,
            span: self.span(),
            docs: Vec::new(),
        }
    }
    pub fn show(&self) -> String {
        super::show(&self.to_datum())
    }
    pub fn show_as(&self, position: Position) -> Result<String, SyntaxError> {
        super::to_source::show_as(&self.to_datum(), position).map_err(|e| SyntaxError {
            message: e.message,
            span: e.span,
        })
    }

    /// Construct a node with the origin and scope of `like`.
    pub fn make(node: Node, like: &Syntax) -> Result<Self, SyntaxError> {
        let invalid = || SyntaxError {
            message: "the node is not a valid syntax atom".into(),
            span: like.span(),
        };
        let check = |s: &str, accepts: fn(&TokenKind) -> bool| {
            let lexed = crate::lexer::lex_lisp(s);
            lexed.diagnostics.is_empty()
                && lexed.comments.is_empty()
                && lexed.tokens.len() == 2
                && lexed.tokens[0].span == Span::new(0, s.len() as u32)
                && accepts(&lexed.tokens[0].kind)
        };
        let valid = match &node {
            Node::Name(s) => check(s, |k| matches!(k, TokenKind::Name(_))),
            Node::Discard(s) => check(s, |k| matches!(k, TokenKind::Discard(_))),
            Node::Up(s) => check(s, |k| matches!(k, TokenKind::UpName(_))),
            Node::Symbol(s) => check(s, |k| {
                !matches!(
                    k,
                    TokenKind::Name(_)
                        | TokenKind::Discard(_)
                        | TokenKind::UpName(_)
                        | TokenKind::Int(_)
                        | TokenKind::Float(_)
                        | TokenKind::Str(_)
                        | TokenKind::LParen
                        | TokenKind::RParen
                        | TokenKind::LBrace
                        | TokenKind::RBrace
                        | TokenKind::LBracket
                        | TokenKind::RBracket
                        | TokenKind::LtLt
                        | TokenKind::GtGt
                        | TokenKind::Quote
                        | TokenKind::Tilde
                        | TokenKind::Eof
                )
            }),
            Node::Float(n) => n.is_finite() && !n.is_sign_negative(),
            _ => true,
        };
        if !valid {
            return Err(invalid());
        }
        Ok(Self(Arc::new(SyntaxData {
            node,
            scope: like.scope().clone(),
            span: like.span(),
        })))
    }

    /// Flip one expansion mark on this node and every child.
    pub fn flip_mark(&self, mark: u64) -> Self {
        let map = |xs: &[Syntax]| xs.iter().map(|x| x.flip_mark(mark)).collect();
        let node = match self.node() {
            Node::Round(xs) => Node::Round(map(xs)),
            Node::Square(xs) => Node::Square(map(xs)),
            Node::Curly(xs) => Node::Curly(map(xs)),
            Node::Bits(xs) => Node::Bits(map(xs)),
            Node::Quote(x) => Node::Quote(x.flip_mark(mark)),
            Node::Unquote(x) => Node::Unquote(x.flip_mark(mark)),
            Node::Splice(x) => Node::Splice(x.flip_mark(mark)),
            leaf => leaf.clone(),
        };
        let mut scope = self.scope().clone();
        if !scope.marks.insert(mark) {
            scope.marks.remove(&mark);
        }
        Self(Arc::new(SyntaxData {
            node,
            scope,
            span: self.span(),
        }))
    }

    /// Compare datums, ignoring provenance, scopes, and value identity.
    pub fn same_code(&self, other: &Self) -> bool {
        use Node::*;
        fn all(a: &[Syntax], b: &[Syntax]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same_code(b))
        }
        match (self.node(), other.node()) {
            (Name(a), Name(b))
            | (Discard(a), Discard(b))
            | (Up(a), Up(b))
            | (Symbol(a), Symbol(b))
            | (Text(a), Text(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (Round(a), Round(b))
            | (Square(a), Square(b))
            | (Curly(a), Curly(b))
            | (Bits(a), Bits(b)) => all(a, b),
            (Quote(a), Quote(b)) | (Unquote(a), Unquote(b)) | (Splice(a), Splice(b)) => {
                a.same_code(b)
            }
            _ => false,
        }
    }

    /// Evaluate a quoted template. The caller evaluates each embedded i
    /// expression and returns either one syntax value or a sequence.
    pub fn instantiate(
        &self,
        evaluate: &mut impl FnMut(&Syntax) -> Result<Substitution, SyntaxError>,
    ) -> Result<Syntax, SyntaxError> {
        let Node::Quote(body) = self.node() else {
            return Err(self.error("expected a quote"));
        };
        fn one(
            s: &Syntax,
            eval: &mut impl FnMut(&Syntax) -> Result<Substitution, SyntaxError>,
        ) -> Result<Syntax, SyntaxError> {
            fn sequence(
                xs: &[Syntax],
                eval: &mut impl FnMut(&Syntax) -> Result<Substitution, SyntaxError>,
            ) -> Result<Vec<Syntax>, SyntaxError> {
                let mut out = Vec::new();
                for x in xs {
                    if let Node::Splice(e) = x.node() {
                        match eval(e)? {
                            Substitution::Many(items) => out.extend(items),
                            Substitution::One(_) => {
                                return Err(x.error("a splice needs a list of syntax values"));
                            }
                        }
                    } else {
                        out.push(one(x, eval)?);
                    }
                }
                Ok(out)
            }
            match s.node() {
                Node::Unquote(e) => match eval(e)? {
                    Substitution::One(value) => Ok(value),
                    Substitution::Many(_) => Err(s.error("an unquote needs one syntax value")),
                },
                Node::Splice(_) => Err(s.error("a splice must be inside brackets")),
                Node::Round(xs) => Syntax::make(Node::Round(sequence(xs, eval)?), s),
                Node::Square(xs) => Syntax::make(Node::Square(sequence(xs, eval)?), s),
                Node::Curly(xs) => Syntax::make(Node::Curly(sequence(xs, eval)?), s),
                Node::Bits(xs) => Syntax::make(Node::Bits(sequence(xs, eval)?), s),
                Node::Quote(_) => Err(s.error("a quote cannot be nested directly in a quote")),
                _ => Ok(s.clone()),
            }
        }
        one(body, evaluate)
    }

    /// Match a quote pattern without publishing partial captures on failure.
    pub fn match_quote(&self, value: &Syntax) -> Result<Option<Vec<Capture>>, SyntaxError> {
        let Node::Quote(pattern) = self.node() else {
            return Err(self.error("expected a quote pattern"));
        };
        fn validate(pattern: &Syntax) -> Result<(), SyntaxError> {
            match pattern.node() {
                Node::Splice(_) => Err(pattern.error("a splice must be the last part of brackets")),
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    for (i, child) in xs.iter().enumerate() {
                        if matches!(child.node(), Node::Splice(_)) {
                            if i + 1 != xs.len() {
                                return Err(
                                    child.error("a splice must be the last part of brackets")
                                );
                            }
                        } else {
                            validate(child)?;
                        }
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        validate(pattern)?;
        fn pair(
            pattern: &Syntax,
            value: &Syntax,
            captures: &mut Vec<Capture>,
        ) -> Result<bool, SyntaxError> {
            if let Node::Unquote(p) = pattern.node() {
                captures.push(Capture {
                    pattern: p.clone(),
                    value: Substitution::One(value.clone()),
                });
                return Ok(true);
            }
            if matches!(pattern.node(), Node::Splice(_)) {
                return Err(pattern.error("a splice must be the last part of brackets"));
            }
            fn parts(
                ps: &[Syntax],
                vs: &[Syntax],
                captures: &mut Vec<Capture>,
            ) -> Result<bool, SyntaxError> {
                let splice = ps.last().and_then(|p| {
                    if let Node::Splice(x) = p.node() {
                        Some((p, x))
                    } else {
                        None
                    }
                });
                let fixed = ps.len() - usize::from(splice.is_some());
                if vs.len() < fixed || (splice.is_none() && vs.len() != fixed) {
                    return Ok(false);
                }
                for (p, v) in ps[..fixed].iter().zip(vs) {
                    if !pair(p, v, captures)? {
                        return Ok(false);
                    }
                }
                if let Some((_, p)) = splice {
                    captures.push(Capture {
                        pattern: p.clone(),
                        value: Substitution::Many(vs[fixed..].to_vec()),
                    });
                }
                Ok(true)
            }
            match (pattern.node(), value.node()) {
                (Node::Round(p), Node::Round(v))
                | (Node::Square(p), Node::Square(v))
                | (Node::Curly(p), Node::Curly(v))
                | (Node::Bits(p), Node::Bits(v)) => parts(p, v, captures),
                _ => Ok(pattern.same_code(value)),
            }
        }
        let mut captures = Vec::new();
        Ok(pair(pattern, value, &mut captures)?.then_some(captures))
    }

    fn error(&self, message: &str) -> SyntaxError {
        SyntaxError {
            message: message.into(),
            span: self.span(),
        }
    }
}
