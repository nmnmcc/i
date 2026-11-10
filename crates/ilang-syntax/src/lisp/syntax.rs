//! Source-located, hygienic syntax values for compile-time code.
use std::collections::BTreeSet;
use std::sync::Arc;

use super::{Datum, DatumKind};
use crate::span::Span;
use crate::token::TokenKind;
use num_bigint::BigUint;

/// A module and the marks that determine name resolution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    pub module: u32,
    pub marks: BTreeSet<u64>,
}

/// A name supplied by a compiler query may identify a binder or builtin
/// directly, independently of ordinary module scope and expansion marks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScopeIdentity {
    DefinitionParameter {
        answer: u64,
        parameter: u32,
        name: String,
    },
    Builtin {
        name: String,
    },
    TypeVariable {
        variable: u64,
        name: String,
    },
}

impl ScopeIdentity {
    fn name(&self) -> &str {
        match self {
            Self::DefinitionParameter { name, .. }
            | Self::Builtin { name }
            | Self::TypeVariable { name, .. } => name,
        }
    }
}

/// An opaque syntax value. Equality of values is identity, not code equality.
#[derive(Clone, Debug)]
pub struct Syntax(Arc<SyntaxData>);

#[derive(Clone, Debug)]
struct SyntaxData {
    node: Node,
    scope: Scope,
    span: Span,
    provenance: Provenance,
    identity: Option<ScopeIdentity>,
    definition_head: Option<u64>,
    origin: Arc<()>,
}

/// Name provenance determines which foreign declarations `make` may reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    Written,
    Answer,
    Made { from_answer: bool },
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
    pub(super) fn written(node: Node, scope: &Scope, span: Span) -> Self {
        Self(Arc::new(SyntaxData {
            node,
            scope: scope.clone(),
            span,
            provenance: Provenance::Written,
            identity: None,
            definition_head: None,
            origin: Arc::new(()),
        }))
    }
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
            provenance: Provenance::Written,
            identity: None,
            definition_head: None,
            origin: Arc::new(()),
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
    pub fn provenance(&self) -> Provenance {
        self.0.provenance
    }
    pub fn scope_identity(&self) -> Option<&ScopeIdentity> {
        self.0.identity.as_ref()
    }
    /// Identity of the query answer whose original definition head this is.
    /// Only compiler-produced answers can assign it; `make` never copies it.
    pub fn definition_head(&self) -> Option<u64> {
        self.0.definition_head
    }
    pub(super) fn with_definition_head(&self, answer: u64) -> Self {
        assert!(
            matches!(self.node(), Node::Round(_)),
            "a definition head is a round node"
        );
        Self(Arc::new(SyntaxData {
            node: self.node().clone(),
            scope: self.scope().clone(),
            span: self.span(),
            provenance: self.provenance(),
            identity: self.scope_identity().cloned(),
            definition_head: Some(answer),
            origin: self.0.origin.clone(),
        }))
    }
    /// Whether a made name remains compatible with a compiler-owned identity.
    /// A false result is a compile error, not a catchable SyntaxError.
    pub fn valid_scope_identity(&self) -> bool {
        match (self.provenance(), self.scope_identity(), self.node()) {
            (Provenance::Made { .. }, Some(identity), Node::Name(name) | Node::Up(name)) => {
                name == identity.name()
            }
            _ => true,
        }
    }
    /// Assign the query identity of a name in a compiler-provided answer.
    pub fn with_scope_identity(&self, identity: ScopeIdentity) -> Self {
        let name = match self.node() {
            Node::Name(name) | Node::Up(name) => name,
            _ => panic!("a special scope identity requires a name"),
        };
        assert_eq!(
            name,
            identity.name(),
            "special scope identity has a different name"
        );
        Self(Arc::new(SyntaxData {
            node: self.node().clone(),
            scope: self.scope().clone(),
            span: self.span(),
            provenance: Provenance::Answer,
            identity: Some(identity),
            definition_head: self.definition_head(),
            origin: Arc::new(()),
        }))
    }
    pub(super) fn with_node(&self, node: Node) -> Self {
        Self(Arc::new(SyntaxData {
            node,
            scope: self.scope().clone(),
            span: self.span(),
            provenance: self.provenance(),
            identity: self.scope_identity().cloned(),
            definition_head: self.definition_head(),
            origin: Arc::new(()),
        }))
    }

    /// Rebuild a quoted template node with its evaluated children. The node's
    /// written origin and scope stay intact; each child keeps its own identity.
    pub fn replace_children(&self, children: Vec<Syntax>) -> Self {
        let node = match self.node() {
            Node::Round(_) => Node::Round(children),
            Node::Square(_) => Node::Square(children),
            Node::Curly(_) => Node::Curly(children),
            Node::Bits(_) => Node::Bits(children),
            Node::Quote(_) | Node::Unquote(_) | Node::Splice(_) => {
                let [child] = <[Syntax; 1]>::try_from(children)
                    .expect("quote template node needs exactly one child");
                match self.node() {
                    Node::Quote(_) => Node::Quote(child),
                    Node::Unquote(_) => Node::Unquote(child),
                    Node::Splice(_) => Node::Splice(child),
                    _ => unreachable!(),
                }
            }
            _ => panic!("cannot replace children of a syntax atom"),
        };
        Self(Arc::new(SyntaxData {
            node,
            scope: self.scope().clone(),
            span: self.span(),
            provenance: self.provenance(),
            identity: self.scope_identity().cloned(),
            definition_head: self.definition_head(),
            origin: self.0.origin.clone(),
        }))
    }

    /// Mark a definition-query answer and all of its descendants as such.
    pub fn as_answer(&self) -> Self {
        let map = |xs: &[Syntax]| xs.iter().map(Syntax::as_answer).collect();
        let node = match self.node() {
            Node::Round(xs) => Node::Round(map(xs)),
            Node::Square(xs) => Node::Square(map(xs)),
            Node::Curly(xs) => Node::Curly(map(xs)),
            Node::Bits(xs) => Node::Bits(map(xs)),
            Node::Quote(x) => Node::Quote(x.as_answer()),
            Node::Unquote(x) => Node::Unquote(x.as_answer()),
            Node::Splice(x) => Node::Splice(x.as_answer()),
            leaf => leaf.clone(),
        };
        Self(Arc::new(SyntaxData {
            node,
            scope: self.scope().clone(),
            span: self.span(),
            provenance: Provenance::Answer,
            identity: self.scope_identity().cloned(),
            definition_head: self.definition_head(),
            origin: Arc::new(()),
        }))
    }
    pub fn identical(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// Stable identity while this value is alive, for host reference interning.
    pub fn identity_key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
    pub(super) fn replace_scope_identity(&self, identity: ScopeIdentity) -> Self {
        Self(Arc::new(SyntaxData {
            node: self.node().clone(),
            scope: self.scope().clone(),
            span: self.span(),
            provenance: self.provenance(),
            identity: Some(identity),
            definition_head: self.definition_head(),
            origin: Arc::new(()),
        }))
    }
    /// Find a descendant by its original source span.
    pub fn at_span(&self, span: Span) -> Option<&Syntax> {
        if self.span() == span {
            return Some(self);
        }
        match self.node() {
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                xs.iter().find_map(|x| x.at_span(span))
            }
            Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => x.at_span(span),
            _ => None,
        }
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
            provenance: Provenance::Made {
                from_answer: matches!(
                    like.provenance(),
                    Provenance::Answer | Provenance::Made { from_answer: true }
                ),
            },
            identity: like.scope_identity().cloned(),
            definition_head: None,
            origin: Arc::new(()),
        })))
    }

    /// Flip one expansion mark on this node and every child.
    pub fn flip_mark(&self, mark: u64) -> Self {
        flip_mark_all(std::slice::from_ref(self), mark).remove(0)
    }

    /// Whether two nodes are the same written or constructed node across mark changes.
    pub fn same_origin(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.origin, &other.0.origin)
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
                Node::Round(xs) => Ok(s.with_node(Node::Round(sequence(xs, eval)?))),
                Node::Square(xs) => Ok(s.with_node(Node::Square(sequence(xs, eval)?))),
                Node::Curly(xs) => Ok(s.with_node(Node::Curly(sequence(xs, eval)?))),
                Node::Bits(xs) => Ok(s.with_node(Node::Bits(sequence(xs, eval)?))),
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

/// Validate the authority and datum restrictions on one macro result (§4.3).
/// The inputs must be the syntax arguments of that run. Mark changes preserve
/// node origins; rebuilding even an identical attribute does not.
pub fn validate_expansion(result: &Syntax, arguments: &[Syntax]) -> Result<(), SyntaxError> {
    use std::collections::{HashMap, HashSet};
    fn same_origin_tree(a: &Syntax, b: &Syntax) -> bool {
        if !a.same_origin(b) {
            return false;
        }
        match (a.node(), b.node()) {
            (Node::Round(x), Node::Round(y))
            | (Node::Square(x), Node::Square(y))
            | (Node::Curly(x), Node::Curly(y))
            | (Node::Bits(x), Node::Bits(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same_origin_tree(x, y))
            }
            (Node::Quote(x), Node::Quote(y))
            | (Node::Unquote(x), Node::Unquote(y))
            | (Node::Splice(x), Node::Splice(y)) => same_origin_tree(x, y),
            (
                Node::Round(_)
                | Node::Square(_)
                | Node::Curly(_)
                | Node::Bits(_)
                | Node::Quote(_)
                | Node::Unquote(_)
                | Node::Splice(_),
                _,
            ) => false,
            _ => true,
        }
    }
    fn restricted(s: &Syntax) -> Option<&str> {
        let Node::Round(xs) = s.node() else {
            return None;
        };
        let [head, attr, _] = xs.as_slice() else {
            return None;
        };
        if !matches!(head.node(), Node::Symbol(x) if x == "@") {
            return None;
        }
        let name = match attr.node() {
            Node::Name(name) => name.as_str(),
            Node::Round(parts) => match parts.first().map(Syntax::node) {
                Some(Node::Name(name)) => name.as_str(),
                _ => return None,
            },
            _ => return None,
        };
        matches!(name, "external" | "host" | "js" | "macro").then_some(name)
    }
    fn descend<'a>(
        root: &'a Syntax,
        f: &mut impl FnMut(&'a Syntax) -> Result<(), SyntaxError>,
    ) -> Result<(), SyntaxError> {
        f(root)?;
        match root.node() {
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                for child in xs {
                    descend(child, f)?;
                }
            }
            Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => descend(x, f)?,
            _ => {}
        }
        Ok(())
    }
    let mut allowed = HashMap::new();
    for arg in arguments {
        descend(arg, &mut |s| {
            if restricted(s).is_some_and(|name| name != "macro") {
                allowed.insert(Arc::as_ptr(&s.0.origin) as usize, s);
            }
            Ok(())
        })?;
    }
    let mut used = HashSet::new();
    descend(result, &mut |s| {
        if !s.valid_scope_identity() {
            return Err(s.error("a made name cannot change a compiler-provided name identity"));
        }
        if matches!(
            s.node(),
            Node::Quote(_) | Node::Unquote(_) | Node::Splice(_)
        ) {
            return Err(s.error("a macro result cannot contain a quote, unquote, or splice"));
        }
        if let Some(name) = restricted(s) {
            if name == "macro" {
                return Err(s.error("a macro result cannot define a macro"));
            }
            let origin = Arc::as_ptr(&s.0.origin) as usize;
            if !allowed
                .get(&origin)
                .is_some_and(|original| same_origin_tree(original, s))
                || !used.insert(origin)
            {
                return Err(s.error("a foreign attribute must come unchanged from an argument and occur at most once"));
            }
        }
        Ok(())
    })
}

/// Flip one expansion mark on a set of values, preserving shared value identity.
pub fn flip_mark_all(values: &[Syntax], mark: u64) -> Vec<Syntax> {
    use std::collections::HashMap;
    fn flip(value: &Syntax, mark: u64, seen: &mut HashMap<usize, Syntax>) -> Syntax {
        let key = Arc::as_ptr(&value.0) as usize;
        if let Some(result) = seen.get(&key) {
            return result.clone();
        }
        let map = |items: &[Syntax], seen: &mut HashMap<usize, Syntax>| {
            items.iter().map(|item| flip(item, mark, seen)).collect()
        };
        let node = match value.node() {
            Node::Round(xs) => Node::Round(map(xs, seen)),
            Node::Square(xs) => Node::Square(map(xs, seen)),
            Node::Curly(xs) => Node::Curly(map(xs, seen)),
            Node::Bits(xs) => Node::Bits(map(xs, seen)),
            Node::Quote(x) => Node::Quote(flip(x, mark, seen)),
            Node::Unquote(x) => Node::Unquote(flip(x, mark, seen)),
            Node::Splice(x) => Node::Splice(flip(x, mark, seen)),
            leaf => leaf.clone(),
        };
        let mut scope = value.scope().clone();
        if !scope.marks.insert(mark) {
            scope.marks.remove(&mark);
        }
        let result = Syntax(Arc::new(SyntaxData {
            node,
            scope,
            span: value.span(),
            provenance: value.provenance(),
            identity: value.scope_identity().cloned(),
            definition_head: value.definition_head(),
            origin: value.0.origin.clone(),
        }));
        seen.insert(key, result.clone());
        result
    }
    let mut seen = HashMap::new();
    values
        .iter()
        .map(|value| flip(value, mark, &mut seen))
        .collect()
}
