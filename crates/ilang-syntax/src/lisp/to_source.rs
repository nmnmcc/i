//! Position-directed conversion of datums to the normal surface.
use super::{Datum, DatumKind as K, Document, show};
use crate::span::Span;
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
    if !crate::shape::equivalent(&wanted, &parsed) {
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
    normal_module(document).map(|(text, _)| text)
}

fn normal_module(document: &Document) -> Result<(String, Vec<(Span, usize)>)> {
    let mut output = String::new();
    let mut anchors = Vec::new();
    let mut headers = 0;
    let mut items_started = false;
    for datum in &document.datums {
        for doc in &datum.docs {
            output.push_str(if doc.module { "////" } else { "///" });
            output.push_str(&doc.text);
            output.push('\n');
        }
        let base = output.len();
        anchors.push((datum.span, base));
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
            output.push('(');
            output.push_str(&joined(rest, if imports { import } else { export }, ", ")?);
            output.push(')');
        } else {
            items_started = true;
            let (text, nested) = super::render::render_anchored(datum, super::render::P::Item)?;
            output.push_str(&text);
            anchors.extend(nested.into_iter().map(|(span, at)| (span, base + at)));
        }
        output.push('\n');
    }
    for doc in &document.trailing_docs {
        output.push_str(if doc.module { "////" } else { "///" });
        output.push_str(&doc.text);
        output.push('\n');
    }
    Ok((output, anchors))
}

/// Convert to normal source and transfer standalone comments at item and
/// statement boundaries. Return the spans of comments that cannot be kept.
pub fn normal_with_comments(source: &str, document: &Document) -> Result<(String, Vec<Span>)> {
    let (output, anchors) = normal_module(document)?;
    // Validate the same item and statement forms as the core conversion.
    let _ = super::to_ast::parse_module(document)?;
    Ok(super::comments::transfer_anchored(source, output, anchors))
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
        }
    }
    #[test]
    fn render_deep_types_without_host_stack_growth() {
        for depth in [1000, 10000] {
            let d = nest(depth, "T");
            let s = show_as(&d, super::super::syntax::Position::Type).unwrap();
            assert_eq!(s, format!("{}x{}", "T(".repeat(depth), ")".repeat(depth)));
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
            format!("{}x{}", "f({ v = ".repeat(10000), " })".repeat(10000))
        );
    }
    #[test]
    fn normal_module_renders_deep_expression_without_host_stack_growth() {
        let binding = datum(K::Round(vec![
            datum(K::Symbol("=".into())),
            datum(K::Name("value".into())),
            nest(10000, "f"),
        ]));
        let module = Document {
            datums: vec![binding],
            trailing_docs: vec![],
        };
        assert_eq!(
            normal_source(&module).unwrap(),
            format!("value = {}x{}\n", "f(".repeat(10000), ")".repeat(10000))
        );
    }
    #[test]
    fn deep_comment_conversion_is_linear_and_keeps_the_anchor() {
        struct Frame {
            call: usize,
            curly: usize,
            stmt: usize,
        }
        fn at(kind: K, start: usize, end: usize) -> Datum {
            Datum {
                kind,
                span: Span::new(start as u32, end as u32),
                docs: vec![],
            }
        }
        let mut source = String::from("(= value ");
        let mut frames = Vec::new();
        for depth in 0..10000 {
            let call = source.len();
            source.push_str("(f { ");
            let curly = call + 3;
            if depth == 5000 {
                source.push_str("\n// marker\n");
            }
            let stmt = source.len();
            source.push_str("(= v ");
            frames.push(Frame { call, curly, stmt });
        }
        let x = source.len();
        source.push('x');
        let mut value = at(K::Name("x".into()), x, x + 1);
        for frame in frames.into_iter().rev() {
            source.push(')');
            let statement = at(
                K::Round(vec![
                    at(K::Symbol("=".into()), frame.stmt + 1, frame.stmt + 2),
                    at(K::Name("v".into()), frame.stmt + 3, frame.stmt + 4),
                    value,
                ]),
                frame.stmt,
                source.len(),
            );
            source.push_str(" v");
            let tail = at(K::Name("v".into()), source.len() - 1, source.len());
            source.push_str(" })");
            let curly = at(
                K::Curly(vec![statement, tail]),
                frame.curly,
                source.len() - 1,
            );
            value = at(
                K::Round(vec![
                    at(K::Name("f".into()), frame.call + 1, frame.call + 2),
                    curly,
                ]),
                frame.call,
                source.len(),
            );
        }
        source.push(')');
        let root = at(
            K::Round(vec![
                at(K::Symbol("=".into()), 1, 2),
                at(K::Name("value".into()), 3, 8),
                value,
            ]),
            0,
            source.len(),
        );
        let document = Document {
            datums: vec![root],
            trailing_docs: vec![],
        };
        let (output, lost) = normal_with_comments(&source, &document).unwrap();
        assert!(lost.is_empty(), "{lost:?}");
        assert_eq!(output.matches("// marker").count(), 1);
        assert!(output.contains("// marker\nv = "));
    }

    #[test]
    fn many_comment_insertions_keep_source_order() {
        let mut source = String::from("(= f (-> () _ {\n");
        for n in 0..10000 {
            source.push_str(&format!("// c{n}\nx\n"));
        }
        source.push_str("}))\n");
        let document = super::super::parse(&source).unwrap();
        let (output, lost) = normal_with_comments(&source, &document).unwrap();
        assert!(lost.is_empty(), "{lost:?}");
        let mut at = 0;
        for n in 0..10000 {
            let wanted = format!("// c{n}\n");
            let found = output[at..]
                .find(&wanted)
                .expect("comment remains in order");
            at += found + wanted.len();
        }
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
