//! Definitions synthesized by the compiler for the language's built-in types.
use super::syntax::{Node, Scope, ScopeIdentity, Syntax, SyntaxError};
use super::{Datum, DatumKind, parse};
use crate::span::Span;

/// Mark only the original parameterized head of a query answer. A parameter
/// taken out of it cannot bind in another head; copies of the whole head carry
/// this identity and freshening changes it together with all its parameters.
fn mark_definition_head(definition: Syntax, answer: u64) -> Syntax {
    let mut cursor = definition;
    let mut wrappers = Vec::new();
    loop {
        let Node::Round(parts) = cursor.node() else {
            break;
        };
        if let [marker, _, inner] = parts.as_slice()
            && matches!(marker.node(), Node::Symbol(s) if s == "@")
        {
            wrappers.push(cursor.clone());
            cursor = inner.clone();
        } else {
            break;
        }
    }
    if let Node::Round(parts) = cursor.node() {
        let head = match parts.as_slice() {
            [eq, _, _] if matches!(eq.node(), Node::Symbol(s) if s == "=") => Some(1),
            [_, body] if matches!(body.node(), Node::Curly(_)) => Some(0),
            _ => None,
        };
        if let Some(index) = head && let Node::Round(head_parts) = parts[index].node()
            && head_parts.iter().skip(1).any(|part| matches!(part.scope_identity(), Some(ScopeIdentity::DefinitionParameter { answer: owner, .. }) if *owner == answer))
        {
            let mut updated = parts.clone();
            updated[index] = updated[index].with_definition_head(answer);
            cursor = cursor.replace_children(updated);
        }
    }
    for wrapper in wrappers.into_iter().rev() {
        let Node::Round(parts) = wrapper.node() else {
            unreachable!()
        };
        cursor = wrapper.replace_children(vec![parts[0].clone(), parts[1].clone(), cursor]);
    }
    cursor
}

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
    Some(mark_definition_head(
        convert(&document[0], scope, at, answer, name == "Result").as_answer(),
        answer,
    ))
}

/// Give a written type definition as a query answer with fresh identities for
/// its formal type parameters. Alias-pattern items retain their written head.
pub fn source_definition(
    item: &crate::ast::Item,
    source: &str,
    scope: &Scope,
    answer: u64,
) -> Syntax {
    use crate::ast::ItemKind;
    let ItemKind::TypeDef(_) = &item.kind else {
        panic!("definition query requires a type definition")
    };
    if source[item.span.start as usize..item.span.end as usize].starts_with('(') {
        let document = super::parse(source).expect("a parsed Lisp source remains valid");
        let datum = document
            .iter()
            .find(|datum| datum.span == item.span)
            .expect("a Lisp source item has its original datum");
        return expanded_definition(&Syntax::from_datum(datum, scope), answer)
            .expect("a parsed type definition has valid syntax");
    }

    let syntax = super::item_syntax(item, scope, source);
    source_definition_from_syntax(item, &syntax, answer)
}

/// Complete an already encoded written item with fresh parameter identities.
pub fn source_definition_from_syntax(
    item: &crate::ast::Item,
    syntax: &Syntax,
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
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            match &ty.kind {
                TypeKind::Named { module, name, args } => {
                    if module.is_none()
                        && let Some(&id) = parameters.get(name.name.as_str())
                    {
                        sites.insert((name.span, name.name.clone()), id);
                    }
                    pending.extend(args.iter().rev());
                }
                TypeKind::Call { args, .. }
                | TypeKind::Tuple(args)
                | TypeKind::Union(args)
                | TypeKind::Inter(args)
                | TypeKind::Diff(args) => pending.extend(args.iter().rev()),
                TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } => {
                    pending.extend(env.iter().rev().map(|t| t.as_ref()));
                    pending.extend(err.iter().rev().map(|t| t.as_ref()));
                    pending.push(ret);
                    pending.extend(params.iter().rev().map(|p| &p.ty));
                }
                TypeKind::Lit(_) | TypeKind::ValueOf { .. } | TypeKind::Hole => {}
            }
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
        enum Task<'a> {
            Enter(&'a Syntax),
            Finish(&'a Syntax, usize),
        }
        let mut pending = vec![Task::Enter(syntax)];
        let mut values = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Enter(node) => {
                    if let Node::Up(name) = node.node()
                        && let Some(&parameter) = sites.get(&(node.span(), name.clone()))
                    {
                        values.push(
                            node.with_scope_identity(ScopeIdentity::DefinitionParameter {
                                answer,
                                parameter,
                                name: name.clone(),
                            }),
                        );
                        continue;
                    }
                    let children: &[Syntax] = match node.node() {
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
                        Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => {
                            std::slice::from_ref(x)
                        }
                        _ => &[],
                    };
                    pending.push(Task::Finish(node, children.len()));
                    pending.extend(children.iter().rev().map(Task::Enter));
                }
                Task::Finish(node, n) => {
                    let children = values.split_off(values.len() - n);
                    let result = match node.node() {
                        Node::Round(_) | Node::Square(_) | Node::Curly(_) | Node::Bits(_) => {
                            node.replace_children(children)
                        }
                        Node::Quote(_) => node.with_node(Node::Quote(
                            children.into_iter().next().expect("quote child"),
                        )),
                        Node::Unquote(_) => node.with_node(Node::Unquote(
                            children.into_iter().next().expect("unquote child"),
                        )),
                        Node::Splice(_) => node.with_node(Node::Splice(
                            children.into_iter().next().expect("splice child"),
                        )),
                        _ => node.clone(),
                    };
                    values.push(result);
                }
            }
        }
        values.pop().expect("annotation result")
    }
    mark_definition_head(annotate(syntax, &sites, answer).as_answer(), answer)
}

/// Freshen one copied definition's bound parameter identities as a unit.
/// Names from other answers and builtin identities retain their identity.
pub fn freshen_parameters(definition: &Syntax, old_answer: u64, new_answer: u64) -> Syntax {
    assert_ne!(
        old_answer, new_answer,
        "a copied definition needs a fresh answer identity"
    );
    use std::collections::HashMap;
    fn key(s: &Syntax) -> usize {
        std::ptr::from_ref(s.node()) as usize
    }
    let mut seen = HashMap::<usize, Syntax>::new();
    let mut pending = vec![(definition, false)];
    while let Some((syntax, finish)) = pending.pop() {
        let id = key(syntax);
        if seen.contains_key(&id) {
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
            seen.get(&key(child))
                .expect("definition child was freshened first")
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
        let mut rewritten = syntax.with_node(node);
        if let Some(ScopeIdentity::DefinitionParameter {
            answer,
            parameter,
            name,
        }) = syntax.scope_identity()
            && *answer == old_answer
        {
            rewritten = rewritten.replace_scope_identity(ScopeIdentity::DefinitionParameter {
                answer: new_answer,
                parameter: *parameter,
                name: name.clone(),
            });
        }
        if rewritten.definition_head() == Some(old_answer) {
            rewritten = rewritten.with_definition_head(new_answer);
        }
        seen.insert(id, rewritten);
    }
    seen.remove(&key(definition))
        .expect("definition was freshened")
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
    if let [marker, _, _] = parts.as_slice()
        && matches!(marker.node(), Node::Symbol(s) if s == "@")
    {
        let mut wrappers = Vec::new();
        let mut body = item;
        loop {
            let Node::Round(parts) = body.node() else {
                break;
            };
            if let [marker, _, inner] = parts.as_slice()
                && matches!(marker.node(), Node::Symbol(s) if s == "@")
            {
                wrappers.push(body);
                body = inner;
            } else {
                break;
            }
        }
        let mut result = expanded_definition(body, answer)?;
        for wrapper in wrappers.into_iter().rev() {
            let Node::Round(parts) = wrapper.node() else {
                unreachable!()
            };
            result = wrapper.with_node(Node::Round(vec![
                parts[0].clone(),
                parts[1].clone(),
                result,
            ]));
        }
        return Ok(result.as_answer());
    }
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
        enum Task<'a> {
            Enter(&'a Syntax),
            Finish(&'a Syntax, usize, bool),
        }
        let mut pending = vec![Task::Enter(s)];
        let mut values = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Enter(node) => {
                    if matches!(node.node(), Node::Up(_)) {
                        values.push(named(node, binders, answer));
                        continue;
                    }
                    if matches!(node.node(), Node::Round(xs) if matches!(xs.as_slice(), [dot, _, _] if matches!(dot.node(), Node::Symbol(x) if x == ".")))
                    {
                        values.push(node.clone());
                        continue;
                    }
                    let children: &[Syntax] = match node.node() {
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
                        _ => {
                            values.push(node.clone());
                            continue;
                        }
                    };
                    pending.push(Task::Finish(
                        node,
                        children.len(),
                        matches!(node.node(), Node::Curly(xs) if xs.len() == 1),
                    ));
                    pending.extend(children.iter().rev().map(Task::Enter));
                }
                Task::Finish(node, n, unwrap) => {
                    let children = values.split_off(values.len() - n);
                    values.push(if unwrap {
                        children.into_iter().next().expect("group child")
                    } else {
                        node.replace_children(children)
                    });
                }
            }
        }
        values.pop().expect("type transformation result")
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
    Ok(mark_definition_head(
        item.with_node(Node::Round(result)).as_answer(),
        answer,
    ))
}

/// Freshen repeated copies of a query-provided definition head within one
/// item result. The caller supplies globally fresh answer identities.
pub fn freshen_duplicate_heads(result: &Syntax, mut fresh: impl FnMut() -> u64) -> Syntax {
    use std::collections::HashSet;
    fn owner(item: &Syntax) -> Option<u64> {
        let Node::Round(parts) = item.node() else {
            return None;
        };
        match parts.as_slice() {
            [marker, _, inner] if matches!(marker.node(), Node::Symbol(s) if s == "@") => {
                owner(inner)
            }
            [head, body] if matches!(body.node(), Node::Curly(_)) => head.definition_head(),
            [marker, head, _] if matches!(marker.node(), Node::Symbol(s) if s == "=") => {
                head.definition_head()
            }
            _ => None,
        }
    }
    let items: Vec<_> = match result.node() {
        Node::Curly(items) => items.clone(),
        _ => vec![result.clone()],
    };
    let mut reserved: HashSet<_> = items.iter().filter_map(owner).collect();
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(id) = owner(&item) else {
            out.push(item);
            continue;
        };
        if seen.insert(id) {
            out.push(item);
            continue;
        }
        let next = fresh();
        assert!(
            reserved.insert(next),
            "definition answer identity must be fresh"
        );
        out.push(freshen_parameters(&item, id, next));
    }
    match result.node() {
        Node::Curly(_) => result.replace_children(out),
        _ => out.pop().expect("a single item result is present"),
    }
}
