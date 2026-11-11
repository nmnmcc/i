//! Source-located, hygienic syntax values for compile-time code.
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, Weak};

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

#[derive(Debug)]
struct SyntaxData {
    node: Node,
    scope: Scope,
    span: Span,
    provenance: Provenance,
    identity: Option<ScopeIdentity>,
    definition_head: Option<u64>,
    origin: Arc<()>,
    host_identity: Mutex<Option<Weak<dyn std::any::Any + Send + Sync>>>,
}

// Deep syntax trees are disposed iteratively: each otherwise-unique child is
// emptied before its Arc is released, so dropping a macro result uses constant
// host stack even when the syntax is very deeply nested.
impl Drop for SyntaxData {
    fn drop(&mut self) {
        fn collect(node: Node, pending: &mut Vec<Syntax>) {
            match node {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    pending.extend(xs)
                }
                Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => pending.push(x),
                _ => {}
            }
        }
        let mut pending = Vec::new();
        collect(
            std::mem::replace(&mut self.node, Node::Round(Vec::new())),
            &mut pending,
        );
        while let Some(syntax) = pending.pop() {
            if let Ok(mut data) = Arc::try_unwrap(syntax.0) {
                collect(
                    std::mem::replace(&mut data.node, Node::Round(Vec::new())),
                    &mut pending,
                );
            }
        }
    }
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
            host_identity: Mutex::new(None),
        }))
    }
    pub fn from_datum(d: &Datum, scope: &Scope) -> Self {
        let mut tasks = vec![(d, false)];
        let mut built = Vec::new();
        while let Some((datum, finish)) = tasks.pop() {
            let children: &[Datum] = match &datum.kind {
                DatumKind::Round(xs)
                | DatumKind::Square(xs)
                | DatumKind::Curly(xs)
                | DatumKind::Bits(xs) => xs,
                DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => {
                    std::slice::from_ref(x.as_ref())
                }
                _ => &[],
            };
            if !finish && !children.is_empty() {
                tasks.push((datum, true));
                tasks.extend(children.iter().rev().map(|child| (child, false)));
                continue;
            }
            let values = built.split_off(built.len() - children.len());
            let node = match &datum.kind {
                DatumKind::Name(s) => Node::Name(s.clone()),
                DatumKind::Discard(s) => Node::Discard(s.clone()),
                DatumKind::UpName(s) => Node::Up(s.clone()),
                DatumKind::Symbol(s) => Node::Symbol(s.clone()),
                DatumKind::Int(n) => Node::Int(n.clone()),
                DatumKind::Float(n) => Node::Float(*n),
                DatumKind::String(s) => Node::Text(s.clone()),
                DatumKind::Round(_) => Node::Round(values),
                DatumKind::Square(_) => Node::Square(values),
                DatumKind::Curly(_) => Node::Curly(values),
                DatumKind::Bits(_) => Node::Bits(values),
                DatumKind::Quote(_) => Node::Quote(values.into_iter().next().unwrap()),
                DatumKind::Unquote(_) => Node::Unquote(values.into_iter().next().unwrap()),
                DatumKind::Splice(_) => Node::Splice(values.into_iter().next().unwrap()),
            };
            built.push(Self::written(node, scope, datum.span));
        }
        built.pop().expect("datum conversion produces one syntax")
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
            host_identity: Mutex::new(None),
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
            host_identity: Mutex::new(None),
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
            host_identity: Mutex::new(None),
        }))
    }

    /// Rebuild a quoted template node with its evaluated children. The node's
    /// written origin and scope stay intact; each child keeps its own identity.
    pub fn replace_children(&self, children: Vec<Syntax>) -> Self {
        // The queried type name may change, but its original parameter
        // binders must remain the same syntax nodes in the same positions.
        let definition_head = self.definition_head().filter(|_| match self.node() {
            Node::Round(original) if !original.is_empty() && original.len() == children.len() => {
                original
                    .iter()
                    .skip(1)
                    .zip(children.iter().skip(1))
                    .all(|(a, b)| a.same_origin(b))
            }
            _ => false,
        });
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
            definition_head,
            origin: self.0.origin.clone(),
            host_identity: Mutex::new(None),
        }))
    }

    /// Mark a definition-query answer and all of its descendants as such.
    pub fn as_answer(&self) -> Self {
        use std::collections::HashMap;
        let mut seen = HashMap::<usize, Syntax>::new();
        let mut pending = vec![(self, false)];
        while let Some((syntax, finish)) = pending.pop() {
            let key = Arc::as_ptr(&syntax.0) as usize;
            if seen.contains_key(&key) {
                continue;
            }
            if !finish {
                pending.push((syntax, true));
                match syntax.node() {
                    Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                        for child in xs.iter().rev() {
                            pending.push((child, false));
                        }
                    }
                    Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => pending.push((x, false)),
                    _ => {}
                }
                continue;
            }
            let mapped = |child: &Syntax| {
                seen.get(&(Arc::as_ptr(&child.0) as usize))
                    .expect("answer child was built first")
                    .clone()
            };
            let children = |xs: &[Syntax]| xs.iter().map(mapped).collect();
            let node = match syntax.node() {
                Node::Round(xs) => Node::Round(children(xs)),
                Node::Square(xs) => Node::Square(children(xs)),
                Node::Curly(xs) => Node::Curly(children(xs)),
                Node::Bits(xs) => Node::Bits(children(xs)),
                Node::Quote(x) => Node::Quote(mapped(x)),
                Node::Unquote(x) => Node::Unquote(mapped(x)),
                Node::Splice(x) => Node::Splice(mapped(x)),
                leaf => leaf.clone(),
            };
            seen.insert(
                key,
                Syntax(Arc::new(SyntaxData {
                    node,
                    scope: syntax.scope().clone(),
                    span: syntax.span(),
                    provenance: Provenance::Answer,
                    identity: syntax.scope_identity().cloned(),
                    definition_head: syntax.definition_head(),
                    origin: Arc::new(()),
                    host_identity: Mutex::new(None),
                })),
            );
        }
        seen.remove(&(Arc::as_ptr(&self.0) as usize))
            .expect("answer root was built")
    }
    pub fn identical(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    /// A weakly held host identity for this value. The host reference owns the
    /// Syntax, while the Syntax holds only a weak reference, so no cycle or
    /// process-wide cache survives a run.
    pub fn host_identity(&self) -> Arc<dyn std::any::Any + Send + Sync> {
        let mut cached = self
            .0
            .host_identity
            .lock()
            .expect("syntax host identity lock poisoned");
        if let Some(identity) = cached.as_ref().and_then(Weak::upgrade) {
            return identity;
        }
        let identity: Arc<dyn std::any::Any + Send + Sync> = Arc::new(self.clone());
        *cached = Some(Arc::downgrade(&identity));
        identity
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
            host_identity: Mutex::new(None),
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
        enum Task<'a> {
            Visit(&'a Syntax),
            Finish(&'a Syntax, usize),
        }
        let mut tasks = vec![Task::Visit(self)];
        let mut values = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(value) => {
                    let children = match value.node() {
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                            Some(xs.as_slice())
                        }
                        Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => {
                            Some(std::slice::from_ref(x))
                        }
                        _ => None,
                    };
                    if let Some(children) = children {
                        tasks.push(Task::Finish(value, children.len()));
                        tasks.extend(children.iter().rev().map(Task::Visit));
                    } else {
                        let kind = match value.node() {
                            Node::Name(s) => DatumKind::Name(s.clone()),
                            Node::Discard(s) => DatumKind::Discard(s.clone()),
                            Node::Up(s) => DatumKind::UpName(s.clone()),
                            Node::Symbol(s) => DatumKind::Symbol(s.clone()),
                            Node::Int(n) => DatumKind::Int(n.clone()),
                            Node::Float(n) => DatumKind::Float(*n),
                            Node::Text(s) => DatumKind::String(s.clone()),
                            _ => unreachable!("composite syntax has children"),
                        };
                        values.push(Datum {
                            kind,
                            span: value.span(),
                            docs: Vec::new(),
                        });
                    }
                }
                Task::Finish(value, count) => {
                    let at = values
                        .len()
                        .checked_sub(count)
                        .expect("syntax child stack underflow");
                    let mut children = values.split_off(at);
                    let kind = match value.node() {
                        Node::Round(_) => DatumKind::Round(children),
                        Node::Square(_) => DatumKind::Square(children),
                        Node::Curly(_) => DatumKind::Curly(children),
                        Node::Bits(_) => DatumKind::Bits(children),
                        Node::Quote(_) => {
                            DatumKind::Quote(Box::new(children.pop().expect("quote child")))
                        }
                        Node::Unquote(_) => {
                            DatumKind::Unquote(Box::new(children.pop().expect("unquote child")))
                        }
                        Node::Splice(_) => {
                            DatumKind::Splice(Box::new(children.pop().expect("splice child")))
                        }
                        _ => unreachable!("leaf syntax has no finish task"),
                    };
                    values.push(Datum {
                        kind,
                        span: value.span(),
                        docs: Vec::new(),
                    });
                }
            }
        }
        assert_eq!(values.len(), 1, "syntax conversion has one result");
        values.pop().expect("syntax conversion result")
    }
    pub fn show(&self) -> String {
        super::show_tree(self)
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
            host_identity: Mutex::new(None),
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
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left.node(), right.node()) {
                (Name(a), Name(b))
                | (Discard(a), Discard(b))
                | (Up(a), Up(b))
                | (Symbol(a), Symbol(b))
                | (Text(a), Text(b))
                    if a == b => {}
                (Int(a), Int(b)) if a == b => {}
                (Float(a), Float(b)) if a == b => {}
                (Round(a), Round(b))
                | (Square(a), Square(b))
                | (Curly(a), Curly(b))
                | (Bits(a), Bits(b))
                    if a.len() == b.len() =>
                {
                    pending.extend(a.iter().zip(b));
                }
                (Quote(a), Quote(b)) | (Unquote(a), Unquote(b)) | (Splice(a), Splice(b)) => {
                    pending.push((a, b));
                }
                _ => return false,
            }
        }
        true
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
        let mut validate = vec![(pattern, false)];
        while let Some((current, allowed_splice)) = validate.pop() {
            match current.node() {
                Node::Splice(_) if !allowed_splice => {
                    return Err(current.error("a splice must be the last part of brackets"));
                }
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    validate.extend(
                        xs.iter()
                            .enumerate()
                            .rev()
                            .map(|(i, child)| (child, i + 1 == xs.len())),
                    );
                }
                _ => {}
            }
        }
        enum Task<'a> {
            Pair(&'a Syntax, &'a Syntax),
            Splice(&'a Syntax, &'a [Syntax]),
        }
        let mut tasks = vec![Task::Pair(pattern, value)];
        let mut captures = Vec::new();
        while let Some(task) = tasks.pop() {
            let (pattern, value) = match task {
                Task::Splice(p, values) => {
                    captures.push(Capture {
                        pattern: p.clone(),
                        value: Substitution::Many(values.to_vec()),
                    });
                    continue;
                }
                Task::Pair(pattern, value) => (pattern, value),
            };
            if let Node::Unquote(p) = pattern.node() {
                captures.push(Capture {
                    pattern: p.clone(),
                    value: Substitution::One(value.clone()),
                });
                continue;
            }
            let (ps, vs) = match (pattern.node(), value.node()) {
                (Node::Round(p), Node::Round(v))
                | (Node::Square(p), Node::Square(v))
                | (Node::Curly(p), Node::Curly(v))
                | (Node::Bits(p), Node::Bits(v)) => (p, v),
                _ if pattern.same_code(value) => continue,
                _ => return Ok(None),
            };
            let splice = ps.last().and_then(|p| match p.node() {
                Node::Splice(x) => Some(x),
                _ => None,
            });
            let fixed = ps.len() - usize::from(splice.is_some());
            if vs.len() < fixed || (splice.is_none() && vs.len() != fixed) {
                return Ok(None);
            }
            if let Some(p) = splice {
                tasks.push(Task::Splice(p, &vs[fixed..]));
            }
            tasks.extend(
                ps[..fixed]
                    .iter()
                    .zip(vs)
                    .rev()
                    .map(|(p, v)| Task::Pair(p, v)),
            );
        }
        Ok(Some(captures))
    }

    fn error(&self, message: &str) -> SyntaxError {
        SyntaxError {
            message: message.into(),
            span: self.span(),
        }
    }
}

/// Whether every node still comes from exactly the same source subtree.
fn same_origin_tree(a: &Syntax, b: &Syntax) -> bool {
    let mut pending = vec![(a, b)];
    while let Some((a, b)) = pending.pop() {
        if !a.same_origin(b) {
            return false;
        }
        match (a.node(), b.node()) {
            (Node::Round(x), Node::Round(y))
            | (Node::Square(x), Node::Square(y))
            | (Node::Curly(x), Node::Curly(y))
            | (Node::Bits(x), Node::Bits(y)) => {
                if x.len() != y.len() {
                    return false;
                }
                pending.extend(x.iter().zip(y));
            }
            (Node::Quote(x), Node::Quote(y))
            | (Node::Unquote(x), Node::Unquote(y))
            | (Node::Splice(x), Node::Splice(y)) => pending.push((x, y)),
            (
                Node::Round(_)
                | Node::Square(_)
                | Node::Curly(_)
                | Node::Bits(_)
                | Node::Quote(_)
                | Node::Unquote(_)
                | Node::Splice(_),
                _,
            ) => return false,
            _ => {}
        }
    }
    true
}

/// Validate the authority and datum restrictions on one macro result (§4.3).
/// The inputs must be the syntax arguments of that run. Mark changes preserve
/// node origins; rebuilding even an identical attribute does not.
pub fn validate_expansion(result: &Syntax, arguments: &[Syntax]) -> Result<(), SyntaxError> {
    use std::collections::{HashMap, HashSet};
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
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            f(current)?;
            match current.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    pending.extend(xs.iter().rev())
                }
                Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => pending.push(x),
                _ => {}
            }
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
    let mut seen = HashMap::<usize, Syntax>::new();
    let mut pending = Vec::new();
    for value in values.iter().rev() {
        pending.push((value, false));
    }
    while let Some((value, finish)) = pending.pop() {
        let key = Arc::as_ptr(&value.0) as usize;
        if seen.contains_key(&key) {
            continue;
        }
        if !finish {
            pending.push((value, true));
            match value.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    for child in xs.iter().rev() {
                        pending.push((child, false));
                    }
                }
                Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => pending.push((x, false)),
                _ => {}
            }
            continue;
        }
        let mapped = |child: &Syntax| {
            seen.get(&(Arc::as_ptr(&child.0) as usize))
                .expect("child was flipped first")
                .clone()
        };
        let map = |items: &[Syntax]| items.iter().map(mapped).collect();
        let node = match value.node() {
            Node::Round(xs) => Node::Round(map(xs)),
            Node::Square(xs) => Node::Square(map(xs)),
            Node::Curly(xs) => Node::Curly(map(xs)),
            Node::Bits(xs) => Node::Bits(map(xs)),
            Node::Quote(x) => Node::Quote(mapped(x)),
            Node::Unquote(x) => Node::Unquote(mapped(x)),
            Node::Splice(x) => Node::Splice(mapped(x)),
            leaf => leaf.clone(),
        };
        let mut scope = value.scope().clone();
        if !scope.marks.insert(mark) {
            scope.marks.remove(&mark);
        }
        seen.insert(
            key,
            Syntax(Arc::new(SyntaxData {
                node,
                scope,
                span: value.span(),
                provenance: value.provenance(),
                identity: value.scope_identity().cloned(),
                definition_head: value.definition_head(),
                origin: value.0.origin.clone(),
                host_identity: Mutex::new(None),
            })),
        );
    }
    values
        .iter()
        .map(|value| seen[&(Arc::as_ptr(&value.0) as usize)].clone())
        .collect()
}
