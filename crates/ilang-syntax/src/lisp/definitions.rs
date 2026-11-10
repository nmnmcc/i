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

/// Give a written type definition as a query answer with fresh identities for
/// its formal type parameters. Alias-pattern items retain their written head.
pub fn source_definition(
    item: &crate::ast::Item,
    source: &str,
    scope: &Scope,
    answer: u64,
) -> Syntax {
    use crate::ast::{ItemKind, TypeBody, TypeExpr, TypeKind};
    use std::collections::HashMap;
    let ItemKind::TypeDef(def) = &item.kind else {
        panic!("definition query requires a type definition")
    };
    let parameters: HashMap<_, _> = def
        .params
        .iter()
        .enumerate()
        .map(|(index, name)| (name.name.as_str(), index as u32))
        .collect();
    let mut sites = HashMap::new();
    for name in &def.params {
        sites.insert(
            (name.span, name.name.clone()),
            parameters[name.name.as_str()],
        );
    }
    fn type_sites(
        ty: &TypeExpr,
        parameters: &HashMap<&str, u32>,
        sites: &mut HashMap<(Span, String), u32>,
    ) {
        match &ty.kind {
            TypeKind::Named { module, name, args } => {
                if module.is_none()
                    && let Some(&id) = parameters.get(name.name.as_str())
                {
                    sites.insert((name.span, name.name.clone()), id);
                }
                for arg in args {
                    type_sites(arg, parameters, sites);
                }
            }
            TypeKind::Call { args, .. } => {
                for arg in args {
                    type_sites(arg, parameters, sites);
                }
            }
            TypeKind::Tuple(types)
            | TypeKind::Union(types)
            | TypeKind::Inter(types)
            | TypeKind::Diff(types) => {
                for ty in types {
                    type_sites(ty, parameters, sites);
                }
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                for param in params {
                    type_sites(&param.ty, parameters, sites);
                }
                type_sites(ret, parameters, sites);
                for ty in err.iter().chain(env.iter()) {
                    type_sites(ty, parameters, sites);
                }
            }
            TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
        }
    }
    for ty in &def.head {
        type_sites(ty, &parameters, &mut sites);
    }
    match &def.body {
        TypeBody::Alias(ty) => type_sites(ty, &parameters, &mut sites),
        TypeBody::Custom(constructors) => {
            for constructor in constructors {
                for field in &constructor.fields {
                    type_sites(&field.ty, &parameters, &mut sites);
                }
            }
        }
    }
    fn annotate(syntax: &Syntax, sites: &HashMap<(Span, String), u32>, answer: u64) -> Syntax {
        if let Node::Up(name) = syntax.node()
            && let Some(&parameter) = sites.get(&(syntax.span(), name.clone()))
        {
            return syntax.with_scope_identity(ScopeIdentity::DefinitionParameter {
                answer,
                parameter,
                name: name.clone(),
            });
        }
        let children = |xs: &[Syntax]| xs.iter().map(|s| annotate(s, sites, answer)).collect();
        let node = match syntax.node() {
            Node::Round(xs) => Node::Round(children(xs)),
            Node::Square(xs) => Node::Square(children(xs)),
            Node::Curly(xs) => Node::Curly(children(xs)),
            Node::Bits(xs) => Node::Bits(children(xs)),
            Node::Quote(x) => Node::Quote(annotate(x, sites, answer)),
            Node::Unquote(x) => Node::Unquote(annotate(x, sites, answer)),
            Node::Splice(x) => Node::Splice(annotate(x, sites, answer)),
            leaf => leaf.clone(),
        };
        syntax.with_node(node)
    }
    let syntax = super::item_syntax(item, scope, source);
    annotate(&syntax, &sites, answer).as_answer()
}

/// Freshen one copied definition's bound parameter identities as a unit.
/// Names from other answers and builtin identities retain their identity.
pub fn freshen_parameters(definition: &Syntax, old_answer: u64, new_answer: u64) -> Syntax {
    assert_ne!(
        old_answer, new_answer,
        "a copied definition needs a fresh answer identity"
    );
    fn walk(syntax: &Syntax, old_answer: u64, new_answer: u64) -> Syntax {
        if let Some(ScopeIdentity::DefinitionParameter {
            answer,
            parameter,
            name,
        }) = syntax.scope_identity()
            && *answer == old_answer
        {
            return syntax.replace_scope_identity(ScopeIdentity::DefinitionParameter {
                answer: new_answer,
                parameter: *parameter,
                name: name.clone(),
            });
        }
        let children = |xs: &[Syntax]| xs.iter().map(|s| walk(s, old_answer, new_answer)).collect();
        let node = match syntax.node() {
            Node::Round(xs) => Node::Round(children(xs)),
            Node::Square(xs) => Node::Square(children(xs)),
            Node::Curly(xs) => Node::Curly(children(xs)),
            Node::Bits(xs) => Node::Bits(children(xs)),
            Node::Quote(x) => Node::Quote(walk(x, old_answer, new_answer)),
            Node::Unquote(x) => Node::Unquote(walk(x, old_answer, new_answer)),
            Node::Splice(x) => Node::Splice(walk(x, old_answer, new_answer)),
            leaf => leaf.clone(),
        };
        syntax.with_node(node)
    }
    walk(definition, old_answer, new_answer)
}
