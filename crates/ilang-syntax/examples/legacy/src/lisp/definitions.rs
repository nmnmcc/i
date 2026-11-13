//! Definitions synthesized by the compiler for the language's built-in types.
use super::syntax::{Node, Scope, ScopeIdentity, Syntax, SyntaxError};
use super::{Datum, DatumKind, parse};
use crate::span::Span;

/// The one switch of step 2c for this file and for the typed-macro type
/// syntax (`ilang-check`, `types/typed_macro.rs`): `false` answers in the rows
/// before syntax three, `true` in the rows of syntax-3.md §6.8. Every function
/// here that depends on the rows has a `_rows` form that takes the choice.
pub const NEW_ROWS: bool = super::syntax::ROWS3;

/// Mark only the original parameterized head of a query answer. A parameter
/// taken out of it cannot bind in another head; copies of the whole head carry
/// this identity and freshening changes it together with all its parameters.
fn mark_definition_head(definition: Syntax, answer: u64, new_rows: bool) -> Syntax {
    let mut cursor = definition;
    let mut wrappers = Vec::new();
    while let Node::Round(parts) = cursor.node() {
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
            [_, body] if !new_rows && matches!(body.node(), Node::Curly(_)) => Some(0),
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

/// The names that [`builtin_definition_rows`] answers. In the rows of §6.8 a
/// member of a sum is a type of its own (question 1 of syn3-final-ast.md §5).
pub fn builtin_definition_names(new_rows: bool) -> &'static [&'static str] {
    if new_rows {
        &[
            "Bool",
            "True",
            "False",
            "Nil",
            "Result",
            "Ok",
            "Error",
            "AssertionFailure",
            "Assert",
        ]
    } else {
        &["Bool", "Nil", "Result", "AssertionFailure", "Assert"]
    }
}

/// The written definition of a queryable built-in type. `answer` is fresh for
/// each query so the type parameters in separate answers cannot capture one
/// another. Every node is located at the query that caused this answer.
pub fn builtin_definition(name: &str, scope: &Scope, at: Span, answer: u64) -> Option<Syntax> {
    builtin_definition_rows(name, scope, at, answer, NEW_ROWS)
}

/// [`builtin_definition`] in the chosen rows. In the rows of §6.8 a sum
/// answers `(= T (| C1 C2))` and each member answers its own item
/// `(= C (, …))`; a type with one constructor of its own name is one item.
pub fn builtin_definition_rows(
    name: &str,
    scope: &Scope,
    at: Span,
    answer: u64,
    new_rows: bool,
) -> Option<Syntax> {
    // The item and its type parameters in the order of the head.
    let (source, parameters): (&str, &[&str]) = if new_rows {
        match name {
            "Bool" => ("(= Bool (| True False))", &[]),
            "True" => ("(= True (,))", &[]),
            "False" => ("(= False (,))", &[]),
            "Nil" => ("(= Nil (,))", &[]),
            "Result" => ("(= (Result A E) (| (Ok A) (Error E)))", &["A", "E"]),
            "Ok" => ("(= (Ok A) (, A))", &["A"]),
            "Error" => ("(= (Error E) (, E))", &["E"]),
            "AssertionFailure" => ("(= AssertionFailure (, (= message String)))", &[]),
            "Assert" => (
                "(= Assert (, (= fail (-> (, AssertionFailure) Never))))",
                &[],
            ),
            _ => return None,
        }
    } else {
        match name {
            "Bool" => ("(Bool { True False })", &[]),
            "Nil" => ("(Nil { Nil })", &[]),
            "Result" => ("((Result A E) { (Ok A) (Error E) })", &["A", "E"]),
            "AssertionFailure" => (
                "(AssertionFailure { (AssertionFailure (: message String)) })",
                &[],
            ),
            "Assert" => (
                "(Assert { (Assert (: fail (-> ((: failure AssertionFailure)) Never))) })",
                &[],
            ),
            _ => return None,
        }
    };
    fn convert(d: &Datum, scope: &Scope, at: Span, answer: u64, parameters: &[&str]) -> Syntax {
        let children = |xs: &[Datum]| {
            xs.iter()
                .map(|d| convert(d, scope, at, answer, parameters))
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
            Node::Up(name) => match parameters.iter().position(|p| p == name) {
                Some(parameter) => syntax.with_scope_identity(ScopeIdentity::DefinitionParameter {
                    answer,
                    parameter: parameter as u32,
                    name: name.clone(),
                }),
                None => syntax.with_scope_identity(ScopeIdentity::Builtin { name: name.clone() }),
            },
            _ => syntax,
        }
    }
    let document = parse(source).expect("compiler builtin definition must parse");
    Some(mark_definition_head(
        convert(&document[0], scope, at, answer, parameters).as_answer(),
        answer,
        new_rows,
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
    if NEW_ROWS {
        let syntax = super::source_syntax::source_item_syntax(
            item,
            scope,
            source,
            source[item.span.start as usize..item.span.end as usize].starts_with('('),
        );
        return expanded_definition(&syntax, answer)
            .expect("a written type definition remains valid");
    }
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
    if NEW_ROWS {
        return expanded_definition(syntax, answer)
            .expect("a written type definition remains valid");
    }
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
                TypeKind::Record {
                    base,
                    fields,
                    removed,
                } => {
                    pending.extend(base.iter().map(Box::as_ref));
                    for (label, ty) in fields {
                        if label.parameter
                            && let Some(&id) = parameters.get(label.name.as_str())
                        {
                            sites.insert((label.span, label.name.clone()), id);
                        }
                        pending.push(ty);
                    }
                    for label in removed {
                        if label.parameter
                            && let Some(&id) = parameters.get(label.name.as_str())
                        {
                            sites.insert((label.span, label.name.clone()), id);
                        }
                    }
                }
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
    mark_definition_head(annotate(syntax, &sites, answer).as_answer(), answer, false)
}

/// A written type definition of the tree of syntax three as a query answer
/// in the rows of §6.8, with fresh identities for its type parameters. This is
/// [`source_definition`] for the new `Item`; a sum is the item
/// `(= T (| C1 C2))` and each member is its own item.
pub fn tree_definition(item: &crate::syn3::ast::Item, scope: &Scope, answer: u64) -> Syntax {
    tree_definition_from_datum(&crate::syn3::lisp::item_datum(item), scope, answer)
}

/// [`tree_definition`] for an item that is already a datum of the §6.8 rows:
/// the written datum of a Lisp file, or the builder's output.
pub fn tree_definition_from_datum(datum: &Datum, scope: &Scope, answer: u64) -> Syntax {
    expanded_definition_rows(&Syntax::from_datum(datum, scope), answer, true)
        .expect("a parsed type definition has valid syntax")
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
    expanded_definition_rows(item, answer, NEW_ROWS)
}

/// [`expanded_definition`] in the chosen rows.
pub fn expanded_definition_rows(
    item: &Syntax,
    answer: u64,
    new_rows: bool,
) -> Result<Syntax, SyntaxError> {
    if new_rows {
        return expanded_definition_new(item, answer);
    }
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
        while let Node::Round(parts) = body.node() {
            if let [marker, _, inner] = parts.as_slice()
                && matches!(marker.node(), Node::Symbol(s) if s == "@")
            {
                wrappers.push(body);
                body = inner;
            } else {
                break;
            }
        }
        let mut result = expanded_definition_rows(body, answer, false)?;
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
                        matches!(node.node(), Node::Curly(xs) if xs.len() == 1
                            && !matches!(xs[0].node(), Node::Round(parts)
                                if matches!(parts.first().map(|part| part.node()), Some(Node::Symbol(head)) if head == ":" || head == "-"))),
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
        false,
    ))
}

fn is_symbol(s: &Syntax, text: &str) -> bool {
    matches!(s.node(), Node::Symbol(x) if x == text)
}

/// Whether `root` is a type of the rows of §6.8, without recursion: the
/// accepted shapes are those of the type reader of `syn3::lisp`. The error is
/// at the first part that is not a type.
fn check_type_new(root: &Syntax) -> Result<(), SyntaxError> {
    let fail = |at: &Syntax, message: &str| {
        Err(SyntaxError {
            message: message.into(),
            span: at.span(),
        })
    };
    let mut pending = vec![root];
    // The items of a data type or of a need (`services`).
    fn items<'a>(
        parts: &'a [Syntax],
        services: bool,
        pending: &mut Vec<&'a Syntax>,
    ) -> Result<(), SyntaxError> {
        let mut i = 0;
        while i < parts.len() {
            let part = &parts[i];
            if is_symbol(part, "..") {
                let Some(next) = parts.get(i + 1) else {
                    return Err(SyntaxError {
                        message: "`..` needs a type after it".into(),
                        span: part.span(),
                    });
                };
                if !services {
                    return Err(SyntaxError {
                        message: "a data type has no spread; `.. N` is written only after `<-`"
                            .into(),
                        span: part.span(),
                    });
                }
                pending.push(next);
                i += 2;
                continue;
            }
            match part.node() {
                Node::Round(xs) if xs.first().is_some_and(|x| is_symbol(x, "=")) => {
                    match &xs[1..] {
                        [label, ty] if matches!(label.node(), Node::Name(_)) => pending.push(ty),
                        _ => {
                            return Err(SyntaxError {
                                message: "a labelled item `(= label T)` has two parts".into(),
                                span: part.span(),
                            });
                        }
                    }
                }
                _ => pending.push(part),
            }
            i += 1;
        }
        Ok(())
    }
    while let Some(node) = pending.pop() {
        match node.node() {
            Node::Int(_) | Node::Text(_) | Node::Up(_) | Node::Name(_) | Node::TypeView(_) => {}
            Node::Discard(text) if text == "_" => {}
            Node::Curly(parts) => match parts.as_slice() {
                [inner] => pending.push(inner),
                _ => return fail(node, "a type group `{ T }` has one part"),
            },
            Node::Round(parts) => {
                let Some(head) = parts.first() else {
                    return fail(node, "`()` is not a type; the empty data type is `(,)`");
                };
                let rest = &parts[1..];
                match head.node() {
                    Node::Symbol(symbol) => match (symbol.as_str(), rest) {
                        (",", _) => items(rest, false, &mut pending)?,
                        ("|" | "&", [_, _, ..]) => pending.extend(rest),
                        ("-", [k]) if matches!(k.node(), Node::Int(_)) => {}
                        ("-", [k]) => {
                            return fail(k, "`(- k)` in a type is a negative integer literal");
                        }
                        ("-", [_, _, ..]) => pending.extend(rest),
                        (".", [module, name])
                            if matches!(module.node(), Node::Name(_))
                                && matches!(name.node(), Node::Up(_) | Node::Name(_)) => {}
                        ("->", _) => {
                            let rest = match rest.first().map(|first| first.node()) {
                                Some(Node::Square(generics)) => {
                                    if let Some(other) =
                                        generics.iter().find(|g| !matches!(g.node(), Node::Up(_)))
                                    {
                                        return fail(other, "expected a type parameter");
                                    }
                                    &rest[1..]
                                }
                                _ => rest,
                            };
                            let [param, ret] = rest else {
                                return fail(
                                    node,
                                    "a function type `(-> P R)` has a parameter part and a result",
                                );
                            };
                            pending.push(param);
                            match ret.node() {
                                Node::Round(xs)
                                    if xs.first().is_some_and(|x| is_symbol(x, "<-")) =>
                                {
                                    let [result, needs] = &xs[1..] else {
                                        return fail(ret, "`(<- R S)` has two parts");
                                    };
                                    pending.push(result);
                                    match needs.node() {
                                        Node::Round(ns)
                                            if ns.first().is_some_and(|x| is_symbol(x, ",")) =>
                                        {
                                            items(&ns[1..], true, &mut pending)?;
                                        }
                                        _ => pending.push(needs),
                                    }
                                }
                                _ => pending.push(ret),
                            }
                        }
                        _ => return fail(node, "this is not a type"),
                    },
                    Node::Up(_) if !rest.is_empty() => pending.extend(rest),
                    Node::Name(_) => pending.extend(rest),
                    Node::Round(qualified) => match qualified.as_slice() {
                        [dot, module, name]
                            if is_symbol(dot, ".")
                                && matches!(module.node(), Node::Name(_))
                                && (matches!(name.node(), Node::Name(_))
                                    || (matches!(name.node(), Node::Up(_))
                                        && !rest.is_empty())) =>
                        {
                            pending.extend(rest);
                        }
                        _ => return fail(node, "this is not a type"),
                    },
                    _ => return fail(node, "this is not a type"),
                }
            }
            _ => return fail(node, "this is not a type"),
        }
    }
    Ok(())
}

/// [`expanded_definition`] in the rows of §6.8. A type definition is
/// `(= Head Right)` under any number of `(@ attribute Item)`: `Head` is an
/// upper name, or `(Name P…)`; the parts `P` are the type parameters when all
/// are upper names, and an alias pattern (kept as written) when not. `Right`
/// is `Opaque`, a data type `(, …)`, or a type. Type groups `{ T }` are
/// removed, except the one group of the item form that marks a name whose
/// right side is a data type or `Opaque` (§6.8).
fn expanded_definition_new(item: &Syntax, answer: u64) -> Result<Syntax, SyntaxError> {
    let not_a_definition = || SyntaxError {
        message: "definition expects a type definition".into(),
        span: item.span(),
    };
    let mut wrappers = Vec::new();
    let mut body = item;
    while let Node::Round(parts) = body.node() {
        if let [marker, _, inner] = parts.as_slice()
            && is_symbol(marker, "@")
        {
            wrappers.push(body);
            body = inner;
        } else {
            break;
        }
    }
    let Node::Round(parts) = body.node() else {
        return Err(not_a_definition());
    };
    let [eq, head, right] = parts.as_slice() else {
        return Err(not_a_definition());
    };
    if !is_symbol(eq, "=") {
        return Err(not_a_definition());
    }
    let head_parts: &[Syntax] = match head.node() {
        Node::Up(_) => &[],
        Node::Round(xs) if matches!(xs.first().map(|x| x.node()), Some(Node::Up(_))) => &xs[1..],
        _ => return Err(not_a_definition()),
    };
    for part in head_parts {
        check_type_new(part)?;
    }
    check_type_new(right)?;

    type Binder = (String, Scope, Option<ScopeIdentity>, u32);
    let mut binders: Vec<Binder> = Vec::new();
    // A value-dependent alias binds its head names; those names are not
    // formal type parameters (spec §4.7).
    let mut rhs = right;
    while let Node::Curly(parts) = rhs.node() {
        let [inner] = parts.as_slice() else {
            break;
        };
        rhs = inner;
    }
    let alias_pattern = matches!(rhs.node(), Node::Name(_))
        || matches!(rhs.node(), Node::Round(parts)
            if matches!(parts.as_slice(), [dot, _, last]
                if is_symbol(dot, ".") && matches!(last.node(), Node::Name(_))));
    if !alias_pattern
        && head_parts
            .iter()
            .all(|part| matches!(part.node(), Node::Up(_)))
    {
        for (index, binder) in head_parts.iter().enumerate() {
            let Node::Up(name) = binder.node() else {
                unreachable!()
            };
            binders.push((
                name.clone(),
                binder.scope().clone(),
                binder.scope_identity().cloned(),
                index as u32,
            ));
        }
    }
    /// Give the uses of the type parameters their identity and remove the
    /// type groups. The type parameters of a polymorphic function type
    /// `(-> [A] …)` shadow the definition's inside that type.
    fn ty(s: &Syntax, binders: &[Binder], answer: u64) -> Syntax {
        enum Task<'a> {
            Enter(&'a Syntax),
            Finish(&'a Syntax, usize, bool, bool),
        }
        let mut pending = vec![Task::Enter(s)];
        let mut values = Vec::new();
        let mut shadows: Vec<&[Syntax]> = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Enter(node) => {
                    if let Node::Up(name) = node.node() {
                        let shadowed = shadows.iter().any(|generics| {
                            generics
                                .iter()
                                .any(|g| matches!(g.node(), Node::Up(n) if n == name))
                        });
                        let found = binders.iter().find(|(text, scope, identity, _)| {
                            text == name
                                && scope == node.scope()
                                && identity.as_ref() == node.scope_identity()
                        });
                        values.push(match found {
                            Some((_, _, _, parameter)) if !shadowed => {
                                node.with_scope_identity(ScopeIdentity::DefinitionParameter {
                                    answer,
                                    parameter: *parameter,
                                    name: name.clone(),
                                })
                            }
                            _ => node.clone(),
                        });
                        continue;
                    }
                    let children: &[Syntax] = match node.node() {
                        Node::Round(xs) if matches!(xs.as_slice(), [dot, _, _] if is_symbol(dot, ".")) =>
                        {
                            values.push(node.clone());
                            continue;
                        }
                        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
                        _ => {
                            values.push(node.clone());
                            continue;
                        }
                    };
                    let generics = match (node.node(), children) {
                        (Node::Round(_), [arrow, generics, ..]) if is_symbol(arrow, "->") => {
                            match generics.node() {
                                Node::Square(gs) => Some(gs.as_slice()),
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(generics) = generics {
                        shadows.push(generics);
                    }
                    pending.push(Task::Finish(
                        node,
                        children.len(),
                        matches!(node.node(), Node::Curly(xs) if xs.len() == 1),
                        generics.is_some(),
                    ));
                    pending.extend(children.iter().rev().map(Task::Enter));
                }
                Task::Finish(node, n, unwrap, shadow) => {
                    if shadow {
                        shadows.pop();
                    }
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
        let Node::Round(head_nodes) = head.node() else {
            unreachable!()
        };
        let mut transformed = head_nodes.clone();
        for (index, node) in transformed.iter_mut().enumerate().skip(1) {
            *node = node.with_scope_identity(ScopeIdentity::DefinitionParameter {
                answer,
                parameter: (index - 1) as u32,
                name: binders[index - 1].0.clone(),
            });
        }
        result[1] = head.with_node(Node::Round(transformed));
    }
    result[2] = match right.node() {
        // The group of the item form: `(= U { (, T1 T2) })` is a name, and
        // `(= U (, T1 T2))` is a labelled type.
        Node::Curly(group) if group.len() == 1 => {
            let inner = ty(&group[0], &binders, answer);
            let kept = match inner.node() {
                Node::Round(xs) => xs.first().is_some_and(|x| is_symbol(x, ",")),
                Node::Up(name) => name == "Opaque",
                _ => false,
            };
            if kept {
                right.replace_children(vec![inner])
            } else {
                inner
            }
        }
        _ => ty(right, &binders, answer),
    };
    let mut answered = mark_definition_head(
        body.with_node(Node::Round(result)).as_answer(),
        answer,
        true,
    );
    if wrappers.is_empty() {
        return Ok(answered);
    }
    for wrapper in wrappers.into_iter().rev() {
        let Node::Round(parts) = wrapper.node() else {
            unreachable!()
        };
        answered = wrapper.with_node(Node::Round(vec![
            parts[0].clone(),
            parts[1].clone(),
            answered,
        ]));
    }
    Ok(answered.as_answer())
}

/// Freshen repeated copies of a query-provided definition head within one
/// item result. The caller supplies globally fresh answer identities.
pub fn freshen_duplicate_heads(result: &Syntax, fresh: impl FnMut() -> u64) -> Syntax {
    freshen_duplicate_heads_rows(result, NEW_ROWS, fresh)
}

/// [`freshen_duplicate_heads`] in the chosen rows: in the rows of §6.8 the
/// head of a type definition is always the second part of `(= head right)`.
pub fn freshen_duplicate_heads_rows(
    result: &Syntax,
    new_rows: bool,
    mut fresh: impl FnMut() -> u64,
) -> Syntax {
    use std::collections::HashSet;
    fn owner(item: &Syntax, new_rows: bool) -> Option<u64> {
        let Node::Round(parts) = item.node() else {
            return None;
        };
        match parts.as_slice() {
            [marker, _, inner] if matches!(marker.node(), Node::Symbol(s) if s == "@") => {
                owner(inner, new_rows)
            }
            [head, body] if !new_rows && matches!(body.node(), Node::Curly(_)) => {
                head.definition_head()
            }
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
    let owner = |item: &Syntax| owner(item, new_rows);
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
