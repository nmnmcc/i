//! Definitions synthesized by the compiler for the language's built-in types.
use super::syntax::{Node, Scope, ScopeIdentity, Syntax, SyntaxError};
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

/// The name and optional module of a `definition` query; each atom keeps its
/// own scope, span, and provenance for resolution.
#[derive(Clone, Debug)]
pub struct TypeQuery {
    pub module: Option<Syntax>,
    pub name: Syntax,
}

/// Read the accepted shapes of a `definition` argument (§6).
pub fn type_query(of: &Syntax) -> Result<TypeQuery, super::syntax::SyntaxError> {
    let query = match of.node() {
        Node::Up(_) => Some(TypeQuery {
            module: None,
            name: of.clone(),
        }),
        Node::Round(parts) => match parts.as_slice() {
            [dot, module, name]
                if matches!(dot.node(), Node::Symbol(symbol) if symbol == ".")
                    && matches!(module.node(), Node::Name(_))
                    && matches!(name.node(), Node::Up(_)) =>
            {
                Some(TypeQuery {
                    module: Some(module.clone()),
                    name: name.clone(),
                })
            }
            _ => None,
        },
        _ => None,
    };
    query.ok_or_else(|| super::syntax::SyntaxError {
        message: "definition expects a type name or a qualified type name".into(),
        span: of.span(),
    })
}

/// Turn an expanded type item into a definition answer without losing the
/// distinct scopes of template and caller atoms. The item must already be
/// syntactically valid; malformed items are reported at their source span.
pub fn expanded_definition(item: &Syntax, answer: u64) -> Result<Syntax, SyntaxError> {
    use crate::ast::ItemKind;
    let parsed = super::parse_item(&item.to_datum()).map_err(|error| SyntaxError {
        message: error.message,
        span: error.span,
    })?;
    let ItemKind::TypeDef(def) = parsed.kind else {
        return Err(SyntaxError {
            message: "definition expects a type definition".into(),
            span: item.span(),
        });
    };
    let Node::Round(parts) = item.node() else {
        unreachable!("type definition item is round")
    };
    let (head_index, body_index, custom) = match parts.as_slice() {
        [head, body] if matches!(body.node(), Node::Curly(_)) => {
            let _ = head;
            (0, 1, true)
        }
        [eq, _, _] if matches!(eq.node(), Node::Symbol(s) if s == "=") => (1, 2, false),
        _ => unreachable!("parsed type definition has an item shape"),
    };
    let mut binders = Vec::new();
    if !def.params.is_empty() {
        let Node::Round(head) = parts[head_index].node() else {
            unreachable!("parameterized type has a round head")
        };
        for (index, binder) in head.iter().skip(1).enumerate() {
            let Node::Up(name) = binder.node() else {
                unreachable!("parsed type parameter is an upper name")
            };
            binders.push((
                name.clone(),
                binder.scope().clone(),
                binder.scope_identity().cloned(),
                index as u32,
            ));
        }
    }
    fn named(
        s: &Syntax,
        binders: &[(String, Scope, Option<ScopeIdentity>, u32)],
        answer: u64,
    ) -> Syntax {
        if let Node::Up(name) = s.node()
            && let Some((_, _, _, parameter)) = binders.iter().find(|(text, scope, identity, _)| {
                text == name && scope == s.scope() && identity.as_ref() == s.scope_identity()
            })
        {
            return s.with_scope_identity(ScopeIdentity::DefinitionParameter {
                answer,
                parameter: *parameter,
                name: name.clone(),
            });
        }
        s.clone()
    }
    fn ty(
        s: &Syntax,
        binders: &[(String, Scope, Option<ScopeIdentity>, u32)],
        answer: u64,
    ) -> Syntax {
        match s.node() {
            Node::Up(_) => named(s, binders, answer),
            Node::Round(xs) if matches!(xs.as_slice(), [dot, _, _] if matches!(dot.node(), Node::Symbol(x) if x == ".")) => {
                s.clone()
            }
            Node::Round(xs) => s.with_node(Node::Round(
                xs.iter().map(|x| ty(x, binders, answer)).collect(),
            )),
            Node::Square(xs) => s.with_node(Node::Square(
                xs.iter().map(|x| ty(x, binders, answer)).collect(),
            )),
            Node::Curly(xs) => s.with_node(Node::Curly(
                xs.iter().map(|x| ty(x, binders, answer)).collect(),
            )),
            Node::Bits(xs) => s.with_node(Node::Bits(
                xs.iter().map(|x| ty(x, binders, answer)).collect(),
            )),
            _ => s.clone(),
        }
    }
    let mut result = parts.clone();
    if !binders.is_empty() {
        let Node::Round(head) = parts[head_index].node() else {
            unreachable!()
        };
        let mut transformed = head.clone();
        for (index, node) in transformed.iter_mut().enumerate().skip(1) {
            *node = node.with_scope_identity(ScopeIdentity::DefinitionParameter {
                answer,
                parameter: (index - 1) as u32,
                name: binders[index - 1].0.clone(),
            });
        }
        result[head_index] = parts[head_index].with_node(Node::Round(transformed));
    }
    if custom {
        let Node::Curly(constructors) = parts[body_index].node() else {
            unreachable!()
        };
        let mut converted = Vec::new();
        for constructor in constructors {
            let Node::Round(fields) = constructor.node() else {
                converted.push(constructor.clone());
                continue;
            };
            let mut fields = fields.clone();
            for field in fields.iter_mut().skip(1) {
                *field = ty(field, &binders, answer);
            }
            converted.push(constructor.with_node(Node::Round(fields)));
        }
        result[body_index] = parts[body_index].with_node(Node::Curly(converted));
    } else {
        result[body_index] = ty(&parts[body_index], &binders, answer);
    }
    Ok(item.with_node(Node::Round(result)).as_answer())
}
