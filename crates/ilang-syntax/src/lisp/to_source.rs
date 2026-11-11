//! Position-directed conversion of datums to the normal surface.
use super::{Datum, DatumKind as K, Document, show};
use crate::ast::{Block, Expr, ExprKind, Stmt};
use crate::span::Span;
use crate::walk::{Visitor, Walk};
use crate::{Code, Diagnostic};

type Result<T> = std::result::Result<T, Diagnostic>;

pub(super) fn show_as(d: &Datum, position: super::syntax::Position) -> Result<String> {
    use super::render::{P, render};
    use super::syntax::Position::*;
    match position {
        ImportItem => render(d, P::Import),
        ExportItem => render(d, P::Export),
        Item => {
            if let K::Curly(xs) = &d.kind {
                joined(xs, item, "\n")
            } else {
                item(d)
            }
        }
        Attribute => render(d, P::Attribute).map(|s| format!("@{s}")),
        Constructor => render(d, P::Constructor),
        Field => render(d, P::Field),
        Type => render(d, P::Type),
        FunctionResult => render(d, P::Return),
        Parameter => render(d, P::Parameter),
        Expression => checked_expression(d),
        Statement => render(d, P::Statement),
        CallArgument => render(d, P::Arg),
        Clause => render(d, P::Clause),
        ClauseHead => render(d, P::ClauseHead),
        Pattern => render(d, P::Pattern),
        Segment => render(d, P::Segment),
        Format => render(d, P::Format),
    }
}
fn checked_expression(d: &Datum) -> Result<String> {
    let expected = super::to_ast::parse_expression(d)?;
    let text = expr(d)?;
    let (parsed, errors) = crate::parse_module(&format!("value = {text}\n"));
    if !errors.is_empty() {
        return Err(bad(d, "a normal-surface expression"));
    }
    let mut wanted = parsed.clone();
    let Some(crate::ast::Item {
        kind: crate::ast::ItemKind::Binding(binding),
        ..
    }) = wanted.items.first_mut()
    else {
        return Err(bad(d, "a normal-surface expression"));
    };
    binding.value = expected;
    if crate::shape::canon(&wanted) != crate::shape::canon(&parsed) {
        return Err(bad(d, "a normal-surface expression"));
    }
    Ok(text)
}
pub(super) fn bad(d: &Datum, position: &str) -> Diagnostic {
    Diagnostic::new(
        Code::Unexpected,
        d.span,
        format!("datum is not valid as {position}"),
    )
}
pub(super) fn symbol(d: &Datum, s: &str) -> bool {
    matches!(&d.kind, K::Symbol(x) if x == s)
}
pub(super) fn round(d: &Datum) -> Option<&[Datum]> {
    match &d.kind {
        K::Round(xs) => Some(xs),
        _ => None,
    }
}
pub(super) fn atom(d: &Datum) -> Option<String> {
    match &d.kind {
        K::Name(_) | K::Discard(_) | K::UpName(_) | K::Int(_) | K::Float(_) | K::String(_) => {
            Some(show(d))
        }
        _ => None,
    }
}
pub(super) fn label(d: &Datum) -> Result<String> {
    match &d.kind {
        K::Name(s) => Ok(s.clone()),
        _ => Err(bad(d, "a label")),
    }
}
fn joined(xs: &[Datum], f: fn(&Datum) -> Result<String>, sep: &str) -> Result<String> {
    Ok(xs.iter().map(f).collect::<Result<Vec<_>>>()?.join(sep))
}

/// Convert a Lisp-surface module to normal source. Errors refer to the datum's
/// original span. A future macro phase consumes quotes without printing them.
pub fn normal_source(document: &Document) -> Result<String> {
    let mut lines = Vec::new();
    let mut headers = 0;
    let mut items_started = false;
    for datum in &document.datums {
        for doc in &datum.docs {
            lines.push(format!(
                "{}{}",
                if doc.module { "////" } else { "///" },
                doc.text
            ));
        }
        if let Some([first, rest @ ..]) = round(datum)
            && symbol(first, ",")
        {
            if items_started || rest.is_empty() || headers == 2 {
                return Err(bad(datum, "a module header"));
            }
            let imports = rest.iter().any(is_import);
            if imports && headers != 0 {
                return Err(bad(datum, "an imports header"));
            }
            headers = if imports { 1 } else { 2 };
            lines.push(format!(
                "({})",
                joined(rest, if imports { import } else { export }, ", ")?
            ));
            continue;
        }
        items_started = true;
        lines.push(item(datum)?);
    }
    for doc in &document.trailing_docs {
        lines.push(format!(
            "{}{}",
            if doc.module { "////" } else { "///" },
            doc.text
        ));
    }
    Ok(if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    })
}

/// Convert to normal source and transfer standalone comments at item and
/// statement boundaries. Return the spans of comments that cannot be kept.
pub fn normal_with_comments(source: &str, document: &Document) -> Result<(String, Vec<Span>)> {
    let output = normal_source(document)?;
    let mut anchors = Vec::new();
    for datum in &document.datums {
        let text = if let Some([first, rest @ ..]) = round(datum)
            && symbol(first, ",")
        {
            if rest.is_empty() {
                return Err(bad(datum, "a module header"));
            }
            let f = if rest.iter().any(is_import) {
                import
            } else {
                export
            };
            format!("({})", joined(rest, f, ", ")?)
        } else {
            item(datum)?
        };
        anchors.push((datum.span, text));
    }
    struct Statements(Vec<Span>);
    impl Visitor for Statements {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter {
                return;
            }
            let body: Option<&Block> = match &e.kind {
                ExprKind::Block(b) => Some(b),
                ExprKind::Fn(f) => Some(&f.body),
                _ => None,
            };
            if let Some(body) = body {
                for s in &body.stmts {
                    self.0.push(match s {
                        Stmt::Bind(b) => b.span,
                        Stmt::Env { span, .. } => *span,
                        Stmt::Expr(e) => e.span,
                    });
                }
            }
        }
    }
    let mut module = super::to_ast::parse_module(document)?;
    let mut stmts = Statements(Vec::new());
    module.walk(&mut stmts);
    let mut by_span = std::collections::HashMap::new();
    let mut pending: Vec<_> = document.datums.iter().rev().collect();
    while let Some(d) = pending.pop() {
        by_span.entry(d.span).or_insert(d);
        match &d.kind {
            K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => {
                pending.extend(xs.iter().rev())
            }
            K::Quote(x) | K::Unquote(x) | K::Splice(x) => pending.push(x),
            _ => {}
        }
    }
    for span in stmts.0 {
        if let Some(d) = by_span.get(&span) {
            anchors.push((span, statement(d)?));
        }
    }
    Ok(super::comments::transfer(
        source, output, anchors, true, false, false,
    ))
}

pub(super) fn import_name(d: &Datum) -> Result<String> {
    match &d.kind {
        K::Name(s) | K::UpName(s) => Ok(s.clone()),
        _ => Err(bad(d, "an imported name")),
    }
}
pub(super) fn is_upper_head(mut d: &Datum) -> bool {
    loop {
        if matches!(d.kind, K::UpName(_)) {
            return true;
        }
        match round(d).and_then(|x| x.first()) {
            Some(next) => d = next,
            None => return false,
        }
    }
}
pub(super) fn ends_in_type_test(mut d: &Datum) -> bool {
    loop {
        match round(d) {
            Some([h, _, _]) if symbol(h, ":") => return true,
            Some([h, _, p]) if symbol(h, "=") => d = p,
            _ => return false,
        }
    }
}
pub(super) fn is_match(d: &Datum) -> bool {
    matches!(round(d), Some([h,_,b]) if symbol(h,".") && matches!(b.kind,K::Curly(_)))
}
pub(super) fn is_binop(d: &Datum) -> bool {
    matches!(&d.kind, K::Symbol(s) if matches!(s.as_str(), "|" | "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "<." | "<=." | ">." | ">=." | "<>" | "+" | "-" | "+." | "-." | "*" | "/" | "%" | "*." | "/."))
}
pub(super) fn is_clause(d: &Datum) -> bool {
    matches!(round(d),Some([h,_,_]) if symbol(h,"->"))
}
fn is_import(d: &Datum) -> bool {
    let mut stack = vec![d];
    while let Some(x) = stack.pop() {
        if let Some(xs) = round(x) {
            for child in xs {
                if symbol(child, "/") {
                    return true;
                }
                stack.push(child);
            }
        }
    }
    false
}
fn item(d: &Datum) -> Result<String> {
    super::render::render(d, super::render::P::Item)
}
fn import(d: &Datum) -> Result<String> {
    super::render::render(d, super::render::P::Import)
}
fn export(d: &Datum) -> Result<String> {
    super::render::render(d, super::render::P::Export)
}
fn statement(d: &Datum) -> Result<String> {
    super::render::render(d, super::render::P::Statement)
}
fn expr(d: &Datum) -> Result<String> {
    super::render::render(d, super::render::P::Expr)
}

#[cfg(test)]
mod deep_tests {
    use super::*;
    fn datum(kind: K) -> Datum {
        Datum {
            kind,
            span: Span::default(),
            docs: vec![],
        }
    }
    fn nest(n: usize, name: &str) -> Datum {
        let mut value = datum(K::Name("x".into()));
        for _ in 0..n {
            value = datum(K::Round(vec![
                datum(if name == "T" {
                    K::UpName(name.into())
                } else {
                    K::Name(name.into())
                }),
                value,
            ]));
        }
        value
    }
    #[test]
    fn render_deep_calls_without_host_stack_growth() {
        for depth in [1000, 10000] {
            let d = nest(depth, "f");
            let s = super::super::render::render(&d, super::super::render::P::Expr).unwrap();
            assert_eq!(s, format!("{}x{}", "f(".repeat(depth), ")".repeat(depth)));
            std::mem::forget(d);
        }
    }
    #[test]
    fn render_deep_types_without_host_stack_growth() {
        for depth in [1000, 10000] {
            let d = nest(depth, "T");
            let s = show_as(&d, super::super::syntax::Position::Type).unwrap();
            assert_eq!(s, format!("{}x{}", "T(".repeat(depth), ")".repeat(depth)));
            std::mem::forget(d);
        }
    }
    #[test]
    fn render_deep_constructor_patterns_without_host_stack_growth() {
        let mut d = datum(K::Name("x".into()));
        for _ in 0..10000 {
            d = datum(K::Round(vec![datum(K::UpName("Some".into())), d]));
        }
        let output = show_as(&d, super::super::syntax::Position::Pattern).unwrap();
        assert_eq!(
            output,
            format!("{}x{}", "Some(".repeat(10000), ")".repeat(10000))
        );
        std::mem::forget(d);
    }
    #[test]
    fn render_deep_mixed_block_and_call_without_host_stack_growth() {
        let mut d = datum(K::Name("x".into()));
        for _ in 0..10000 {
            let bind = datum(K::Round(vec![
                datum(K::Symbol("=".into())),
                datum(K::Name("v".into())),
                d,
            ]));
            d = datum(K::Round(vec![
                datum(K::Name("f".into())),
                datum(K::Curly(vec![bind])),
            ]));
        }
        let output = super::super::render::render(&d, super::super::render::P::Expr).unwrap();
        assert_eq!(
            output,
            format!("{}x{}", "f({\nv = ".repeat(10000), "\n})".repeat(10000))
        );
        std::mem::forget(d);
    }
    #[test]
    fn render_long_list_preserves_each_element() {
        let d = datum(K::Square(
            (0..10000).map(|_| datum(K::Name("x".into()))).collect(),
        ));
        let s = super::super::render::render(&d, super::super::render::P::Expr).unwrap();
        assert_eq!(s, format!("[{}]", vec!["x"; 10000].join(", ")));
    }
}
