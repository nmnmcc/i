//! Position-directed printing of the final syntax-three tree.
use super::{Datum, DatumKind as K, Document};
use crate::span::Span;
use crate::{Code, Diagnostic};
type Result<T> = std::result::Result<T, Diagnostic>;
pub fn normal_source(document: &Document) -> Result<String> { normal_source3(document) }
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

pub fn normal_source3(document: &Document) -> Result<String> {
    crate::syn3::deep::deep(|| {
        let (module, errors) = crate::syn3::lisp::from_document(document);
        if let Some(error) = errors.into_iter().next() { return Err(error); }
        Ok(crate::syn3::print_tree(&module, false))
    })
}
#[cfg(test)]
mod deep_tests {
    use migration_syntax::span::Span;
    use migration_syntax::lisp::{Datum, DatumKind as K, Document};
    use migration_syntax::lisp::to_source::{show_as, normal_source, normal_with_comments};
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
            let s = migration_syntax::lisp::render::render(&d, migration_syntax::lisp::render::P::Expr).unwrap();
            assert_eq!(s, format!("{}x{}", "f(".repeat(depth), ")".repeat(depth)));
        }
    }
    #[test]
    fn render_deep_types_without_host_stack_growth() {
        for depth in [1000, 10000] {
            let d = nest(depth, "T");
            let s = show_as(&d, migration_syntax::lisp::syntax::Position::Type).unwrap();
            assert_eq!(s, format!("{}x{}", "T(".repeat(depth), ")".repeat(depth)));
        }
    }
    #[test]
    fn render_deep_constructor_patterns_without_host_stack_growth() {
        let mut d = datum(K::Name("x".into()));
        for _ in 0..10000 {
            d = datum(K::Round(vec![datum(K::UpName("Some".into())), d]));
        }
        let output = show_as(&d, migration_syntax::lisp::syntax::Position::Pattern).unwrap();
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
        let output = migration_syntax::lisp::render::render(&d, migration_syntax::lisp::render::P::Expr).unwrap();
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
        let mut source = String::from("(= f (-> (,) _ {\n");
        for n in 0..10000 {
            source.push_str(&format!("// c{n}\nx\n"));
        }
        source.push_str("}))\n");
        let document = crate::lisp::parse(&source).unwrap();
        let (output, lost) = super::normal_with_comments(&source, &document).unwrap();
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
        let s = migration_syntax::lisp::render::render(&d, migration_syntax::lisp::render::P::Expr).unwrap();
        assert_eq!(s, format!("[{}]", vec!["x"; 10000].join(", ")));
    }
}

#[cfg(test)]
mod record_tests {
    type Result<T> = std::result::Result<T, migration_syntax::Diagnostic>;
    use migration_syntax::lisp::{Datum, DatumKind as K};
    use migration_syntax::lisp::render::{P, render};
    use migration_syntax::lisp::parse;

    fn render_one(input: &str, p: P) -> Result<String> {
        let document = parse(input).unwrap();
        render(&document[0], p)
    }

    #[test]
    fn records_keep_spreads_fields_labels_and_grouping() {
        assert_eq!(render_one("()", P::Expr).unwrap(), "()");
        assert!(render_one("()", P::Pattern).is_err());
        assert_eq!(
            render_one("((: x 1) (: \"y-z\" v))", P::Expr).unwrap(),
            "(x: 1, \"y-z\": v)"
        );
        assert_eq!(
            render_one("(.. base (: x v))", P::Expr).unwrap(),
            "(..base, x: v)"
        );
        assert_eq!(
            render_one("(.. R (- X) (: y T))", P::Type).unwrap(),
            "(..R, -X, y: T)"
        );
        assert_eq!(render_one("{ T }", P::Type).unwrap(), "{ T }");
        assert_eq!(render_one("{ x }", P::Expr).unwrap(), "{ x }");
        assert!(render_one("(.. x (- y))", P::Expr).is_err());
        assert!(render_one("(..)", P::Type).is_err());
    }

    #[test]
    fn deeply_nested_record_fields_render_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let datum = |kind| Datum {
                    kind,
                    span: migration_syntax::span::Span::default(),
                    docs: vec![],
                };
                let mut value = datum(K::Name("v".into()));
                for _ in 0..10_000 {
                    value = datum(K::Round(vec![datum(K::Round(vec![
                        datum(K::Symbol(":".into())),
                        datum(K::Name("x".into())),
                        value,
                    ]))]));
                }
                let expected = format!("{}v{}", "(x: ".repeat(10_000), ")".repeat(10_000));
                assert_eq!(render(&value, P::Expr).unwrap(), expected);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn deeply_nested_record_text_parses_and_renders_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut input = String::with_capacity(100_001);
                for _ in 0..10_000 {
                    input.push_str("((: x ");
                }
                input.push('v');
                for _ in 0..10_000 {
                    input.push_str("))");
                }
                let expected = format!("{}v{}", "(x: ".repeat(10_000), ")".repeat(10_000));
                assert_eq!(render_one(&input, P::Expr).unwrap(), expected);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

/// Convert final rows and transfer comments at their original source anchors.
pub fn normal_with_comments(source: &str, document: &Document) -> Result<(String, Vec<Span>)> {
    crate::deep::deep(|| {
        let (module, errors) = crate::rows::from_document(document);
        if let Some(error) = errors.into_iter().next() { return Err(error); }
        let output = normal_source3(document)?;
        Ok(crate::surface::module_comments(source, &module, output, true, false))
    })
}

#[cfg(test)]
mod native_comment_tests {
    #[test]
    fn native_comments_keep_nested_statement_anchors() {
        let source = "(= value (f {\n// outer\n(= v (g {\n// inner\n(= w x) w})) v}))";
        let document = crate::lisp::parse(source).unwrap();
        let (output, lost) = super::normal_with_comments(source, &document).unwrap();
        assert!(lost.is_empty(), "{lost:?}");
        assert_eq!(output.matches("// outer").count(), 1);
        assert_eq!(output.matches("// inner").count(), 1);
        assert!(output.contains("// outer\nv = "), "{output}");
        assert!(output.contains("// inner\nw = "), "{output}");
        let (_, errors) = crate::parse_module(&output);
        assert!(errors.is_empty(), "{errors:?}");
    }
}
