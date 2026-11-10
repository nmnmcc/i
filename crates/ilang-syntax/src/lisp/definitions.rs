//! Definitions synthesized by the compiler for the language's built-in types.
use super::syntax::{Node, Scope, ScopeIdentity, Syntax};
use super::{Datum, DatumKind, parse};
use crate::span::Span;

/// The written definition of a queryable built-in type. `answer` is fresh for
/// each query so the type parameters in separate answers cannot capture one
/// another. Every node is located at the query that caused this answer.
pub fn builtin_definition(name: &str, scope: &Scope, at: Span, answer: u64) -> Option<Syntax> {
    let source = match name {
        "Bool" => "(Bool { True False })",
        "Nil" => "(Nil { Nil })",
        "Result" => "((Result A E) { (Ok A) (Error E) })",
        "AssertionFailure" => "(AssertionFailure { (AssertionFailure (: message String)) })",
        "Assert" => "(Assert { (Assert (: fail (-> ((: failure AssertionFailure)) Never))) })",
        _ => return None,
    };
    fn convert(d: &Datum, scope: &Scope, at: Span, answer: u64, result: bool) -> Syntax {
        let children = |xs: &[Datum]| {
            xs.iter()
                .map(|d| convert(d, scope, at, answer, result))
                .collect()
        };
        let node = match &d.kind {
            DatumKind::Name(name) => Node::Name(name.clone()),
            DatumKind::UpName(name) => Node::Up(name.clone()),
            DatumKind::Symbol(symbol) => Node::Symbol(symbol.clone()),
            DatumKind::Round(xs) => Node::Round(children(xs)),
            DatumKind::Curly(xs) => Node::Curly(children(xs)),
            _ => panic!("invalid builtin definition datum"),
        };
        let syntax = Syntax::written(node, scope, at);
        match syntax.node() {
            Node::Up(name) if result && matches!(name.as_str(), "A" | "E") => syntax
                .with_scope_identity(ScopeIdentity::DefinitionParameter {
                    answer,
                    parameter: u32::from(name == "E"),
                    name: name.clone(),
                }),
            Node::Up(name) => {
                syntax.with_scope_identity(ScopeIdentity::Builtin { name: name.clone() })
            }
            _ => syntax,
        }
    }
    let document = parse(source).expect("compiler builtin definition must parse");
    Some(convert(&document[0], scope, at, answer, name == "Result").as_answer())
}
