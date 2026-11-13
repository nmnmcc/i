//! Position-directed conversion of datums to the normal surface.
use super::{Datum, DatumKind as K, Document, show};
use crate::span::Span;
use crate::{Code, Diagnostic};

type Result<T> = std::result::Result<T, Diagnostic>;

/// A datum of the rows before syntax three as text of the surface before
/// syntax three.
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
        TypeItem => render(d, P::Field),
        Type => render(d, P::Type),
        FunctionResult => render(d, P::Return),
        PatternItem => render(d, P::Parameter),
        Expression => checked_expression(d),
        Statement => render(d, P::Statement),
        ValueItem => render(d, P::Arg),
        Pattern => render(d, P::Pattern),
        Segment => render(d, P::Segment),
        Format => render(d, P::Format),
    }
}

// ---- The rows of syntax three ----

/// The depth up to which a datum is read and printed on the caller's stack.
const DIRECT_DEPTH: usize = 64;

/// Whether the datum is nested more than `limit` deep.
fn deeper_than(d: &Datum, limit: usize) -> bool {
    let mut work = vec![(d, 0usize)];
    while let Some((d, depth)) = work.pop() {
        if depth > limit {
            return true;
        }
        match &d.kind {
            K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => {
                work.extend(xs.iter().map(|x| (x, depth + 1)))
            }
            K::Quote(x) | K::Unquote(x) | K::Splice(x) => work.push((x, depth + 1)),
            _ => {}
        }
    }
    false
}

/// A datum of the rows of syntax three (docs/design/syntax-3.md §6.8) as
/// text of the normal surface, read for one position of §6.7. The reader and
/// the printer recurse, so a deep datum is handled on a deep stack.
pub(super) fn show_as3(d: &Datum, position: super::syntax::Position) -> Result<String> {
    if deeper_than(d, DIRECT_DEPTH) {
        crate::syn3::deep::deep(|| show3(d, position))
    } else {
        show3(d, position)
    }
}

fn show3(d: &Datum, position: super::syntax::Position) -> Result<String> {
    use super::syntax::Position::*;
    use crate::syn3::ast::{self, ItemKind, Module};
    use crate::syn3::group;
    use crate::syn3::lisp::position::{self as read, PatternPart, SegmentPart};
    use crate::syn3::print::position as print;

    // The normal surface has no quotes: the printer would write them as
    // Lisp text, which no position of the normal surface reads.
    let mut work = vec![d];
    while let Some(x) = work.pop() {
        match &x.kind {
            K::Quote(_) | K::Unquote(_) | K::Splice(_) => {
                return Err(Diagnostic::new(
                    Code::Unexpected,
                    x.span,
                    "a quote, an unquote or a splice has no text in the normal surface",
                ));
            }
            K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => work.extend(xs),
            _ => {}
        }
    }
    // The text is accepted when the normal surface reads it as the tree
    // that the datum is; groups do not count.
    let unreadable = |what: &str| {
        Diagnostic::new(
            Code::Unexpected,
            d.span,
            format!("this syntax has no text in the normal surface as {what}"),
        )
    };
    let module = |items: Vec<ast::Item>| Module {
        items,
        ..Module::default()
    };
    let wrap = |pattern: ast::Pattern, value: ast::Expr| {
        module(vec![ast::Item {
            doc: Vec::new(),
            attributes: Vec::new(),
            kind: ItemKind::Bind(ast::Binding {
                pattern,
                value,
                span: ast::Sp(Span::default()),
            }),
            span: ast::Sp(Span::default()),
        }])
    };
    let same = |a: &Module, b: &Module| {
        crate::syn3::surface::ungrouped(a) == crate::syn3::surface::ungrouped(b)
    };
    let hole = || ast::Pattern::new(ast::PatternKind::Discard("_".into()), Span::default());
    let unit = || ast::Expr::new(ast::ExprKind::Data(Vec::new()), Span::default());
    match position {
        ImportItem => Ok(print::import(&read::import_item(d)?)),
        ExportItem => Ok(print::export(&read::export_item(d)?)),
        Item => {
            let items = match &d.kind {
                K::Curly(xs) => xs.iter().map(read::item).collect::<Result<Vec<_>>>()?,
                _ => vec![read::item(d)?],
            };
            let mut grouped = module(items);
            group::regroup(&mut grouped);
            let text = grouped
                .items
                .iter()
                .map(print::item)
                .collect::<Vec<_>>()
                .join("\n");
            let (back, errors) = crate::syn3::parse_module(&format!("{text}\n"));
            if !errors.is_empty() || !same(&grouped, &back) {
                return Err(unreadable("an item"));
            }
            Ok(text)
        }
        Attribute => {
            let mut attribute = read::attribute(d)?;
            if let Some(args) = &mut attribute.args {
                group::group_items(args);
            }
            Ok(print::attribute(&attribute))
        }
        Type => {
            let ty = read::type_from_datum(d)?;
            let text = print::ty(&ty);
            match crate::syn3::parse_type(&text) {
                Ok(back) if back == ty => Ok(text),
                _ => Err(unreadable("a type")),
            }
        }
        TypeItem => Ok(print::type_item(&read::type_item(d)?)),
        FunctionResult => Ok(print::function_result(&read::function_result(d)?)),
        Expression => {
            let mut expr = read::expr_from_datum(d)?;
            group::group_expr(&mut expr);
            let text = print::expr(&expr);
            let back = crate::syn3::parse_expr(&text).map_err(|_| unreadable("an expression"))?;
            if same(&wrap(hole(), expr), &wrap(hole(), back)) {
                Ok(text)
            } else {
                Err(unreadable("an expression"))
            }
        }
        ValueItem => {
            let mut item = read::value_item(d)?;
            group::group_items(std::slice::from_mut(&mut item));
            Ok(print::value_item(&item))
        }
        Statement => {
            let mut block = ast::Block {
                stmts: vec![read::statement(d)?],
                span: ast::Sp(Span::default()),
            };
            group::group_block(&mut block);
            Ok(print::statement(&block.stmts[0]))
        }
        Pattern => {
            let mut part = read::pattern_part(d)?;
            match &mut part {
                PatternPart::Pattern(pattern) => {
                    group::group_pattern(pattern);
                    let text = print::pattern(&part);
                    let PatternPart::Pattern(pattern) = part else {
                        unreachable!("the part is a pattern")
                    };
                    let back =
                        crate::syn3::parse_pattern(&text).map_err(|_| unreadable("a pattern"))?;
                    if same(&wrap(pattern, unit()), &wrap(back, unit())) {
                        Ok(text)
                    } else {
                        Err(unreadable("a pattern"))
                    }
                }
                PatternPart::Params(items, guard) => {
                    for item in items {
                        item.choices.iter_mut().for_each(group::group_pattern);
                    }
                    if let Some(guard) = guard {
                        group::group_expr(guard);
                    }
                    Ok(print::pattern(&part))
                }
            }
        }
        PatternItem => {
            let mut item = read::pattern_item(d)?;
            if let read::PatternItem::Item(item) = &mut item {
                item.choices.iter_mut().for_each(group::group_pattern);
            }
            Ok(print::pattern_item(&item))
        }
        Segment => {
            let mut segment = read::segment(d)?;
            match &mut segment {
                SegmentPart::Expr(s) => {
                    group::group_expr(&mut s.value);
                    group::group_format(&mut s.format);
                }
                SegmentPart::Pattern(s) => {
                    group::group_pattern(&mut s.value);
                    group::group_format(&mut s.format);
                }
            }
            Ok(print::segment(&segment))
        }
        Format => {
            let mut format = Some(read::format(d)?);
            group::group_format(&mut format);
            Ok(print::format(format.as_ref().expect("the format is set")))
        }
    }
}
fn checked_expression(d: &Datum) -> Result<String> {
    let expected = super::to_ast::parse_expression(d)?;
    let text = expr(d)?;
    let (parsed, errors) = crate::parse_module(&format!("value = {text}\n"));
    if !errors.is_empty() {
        return Err(bad(d, "a normal-surface expression"));
    }
    let Some(crate::ast::Item {
        kind: crate::ast::ItemKind::Binding(binding),
        ..
    }) = parsed.items.first()
    else {
        return Err(bad(d, "a normal-surface expression"));
    };
    if !crate::shape::equivalent_expression(&expected, &binding.value) {
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
    fn render_long_list_preserves_each_element() {
        let d = datum(K::Square(
            (0..10000).map(|_| datum(K::Name("x".into()))).collect(),
        ));
        let s = super::super::render::render(&d, super::super::render::P::Expr).unwrap();
        assert_eq!(s, format!("[{}]", vec!["x"; 10000].join(", ")));
    }
}
