//! Scoped macro input from either source surface, with transparent groups removed.
use super::syntax::{Node, Scope, Syntax};
use super::{Datum, DatumKind};
use crate::span::Span;

fn find(datum: &Datum, span: Span) -> Option<&Datum> {
    let mut pending = vec![datum];
    while let Some(datum) = pending.pop() {
        if datum.span == span {
            return Some(datum);
        }
        match &datum.kind {
            DatumKind::Round(xs)
            | DatumKind::Square(xs)
            | DatumKind::Curly(xs)
            | DatumKind::Bits(xs) => pending.extend(xs.iter().rev()),
            DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => pending.push(x),
            _ => {}
        }
    }
    None
}
/// A written Lisp node for a consumer that already has the new tree.
/// Position normalization remains the caller's choice.
pub fn written_lisp_syntax(span: Span, scope: &Scope, source: &str) -> Syntax {
    lisp_source(span, scope, source)
}

fn lisp_source(span: Span, scope: &Scope, source: &str) -> Syntax {
    let document = super::parse(source).expect("parsed Lisp source remains valid");
    let mut datum = document.iter().find_map(|x| find(x, span));
    if datum.is_none() {
        // Multi-shape values have a synthetic root in the transition AST.
        // Recover the source node's span, then use its original datum so
        // Lisp operator tokens and all occurrence identities stay written.
        if let Some(syntax) =
            super::source_tree::SourceTree::read(scope, source, true).expression(span)
        {
            datum = document.iter().find_map(|x| find(x, syntax.span()));
        }
    }
    let datum = datum.expect("AST node belongs to a Lisp datum");
    // Keep the written rows and occurrence spans when macro syntax uses
    // syntax three. The inactive legacy path needs its old-row adapter.
    Syntax::from_datum(datum, scope)

}
fn symbol(s: &Syntax, expected: &str) -> bool {
    matches!(s.node(), Node::Symbol(value) if value == expected)
}

// ---- The rows of syntax three (docs/design/syntax-3.md §6.8) ----

/// What a datum is read as. The table follows the reader of syntax three
/// (`syn3::lisp`): a group is a Curly with one part where that reader reads
/// the part itself (in a type) or a block of one expression (in an
/// expression). Every other Curly stays: a block, and the `{ }` of a name
/// whose right side is a data type, which is part of the item form.
#[derive(Clone, Copy)]
#[doc(hidden)]
pub enum Mode3 {
    /// An expression; a function body and a branch of a chain are ones.
    Expr,
    /// An item of data or of a call: `e`, `(= label e)`.
    ValueItem,
    /// `(= p e)` or an expression.
    Statement,
    /// A type; the items of a data type, a need and a service pattern
    /// `(, (: db Db))` hold types and atoms only, so they are read as types.
    Type,
    /// The third part of a function: `R`, `_`, or `(<- R needs)`.
    Result,
    /// The right side of a type definition.
    TypeRhs,
    /// A pattern, with the items of its data.
    Pattern,
    /// The parameter part of a function: `(, …)` or `(? (, …) g)`.
    Params,
    SegmentExpr,
    SegmentPattern,
    /// An item: `(@ attribute Item)`, `(= …)`, `(: x T)` or a macro call.
    Item,
    Keep,
}

fn head_is(xs: &[Syntax], expected: &str) -> bool {
    xs.first().is_some_and(|head| symbol(head, expected))
}

/// Whether the Curly `xs` is a group: one part that is not a binding.
fn is_group(xs: &[Syntax]) -> bool {
    match xs {
        [one] => !matches!(one.node(), Node::Round(parts) if head_is(parts, "=")),
        _ => false,
    }
}

/// Whether the head of a definition `(= head value)` is that of a type.
fn is_type_head(head: &Syntax) -> bool {
    match head.node() {
        Node::Up(_) => true,
        Node::Round(parts) => matches!(parts.first().map(Syntax::node), Some(Node::Up(_))),
        _ => false,
    }
}

/// Whether the right side `{ T }` of a type definition keeps its Curly: `T`
/// is a data type or `Opaque`, which without the Curly would make a labelled
/// type or an opaque type.
fn rhs_keeps_curly(mut inner: &Syntax) -> bool {
    loop {
        match inner.node() {
            Node::Curly(xs) if xs.len() == 1 => inner = &xs[0],
            Node::Round(parts) => return head_is(parts, ","),
            Node::Up(name) => return name == "Opaque",
            _ => return false,
        }
    }
}

/// A written expression of the rows of syntax three as a macro sees it:
/// without its groups.
pub fn normalize_expression(s: &Syntax) -> Syntax {
    normalize3(s, Mode3::Expr)
}
/// A written type of the rows of syntax three without its groups.
pub fn normalize_type(s: &Syntax) -> Syntax {
    normalize3(s, Mode3::Type)
}
/// A written pattern of the rows of syntax three without the groups of the
/// types and expressions in it.
pub fn normalize_pattern(s: &Syntax) -> Syntax {
    normalize3(s, Mode3::Pattern)
}
/// A written item of the rows of syntax three without its groups. The kind
/// of the item is read from its form, as the reader of syntax three does.
pub fn normalize_item(s: &Syntax) -> Syntax {
    normalize3(s, Mode3::Item)
}

// Normalize source datums of the rows of syntax three without consuming the
// host stack on nested input.
#[doc(hidden)]
pub fn normalize3(root: &Syntax, mode: Mode3) -> Syntax {
    enum Task {
        Enter(Syntax, Mode3),
        Finish(Syntax, usize, bool),
    }
    let mut work = vec![Task::Enter(root.clone(), mode)];
    let mut built = Vec::new();
    while let Some(task) = work.pop() {
        let (node, mode) = match task {
            Task::Finish(node, count, unwrap) => {
                let children = built.split_off(built.len() - count);
                built.push(if unwrap {
                    children.into_iter().next().expect("group child")
                } else {
                    node.replace_children(children)
                });
                continue;
            }
            Task::Enter(node, mode) => (node, mode),
        };
        let (xs, round, curly, bits) = match node.node() {
            Node::Round(xs) => (xs, true, false, false),
            Node::Curly(xs) => (xs, false, true, false),
            Node::Bits(xs) => (xs, false, false, true),
            Node::Square(xs) => (xs, false, false, false),
            // An atom; a quote is given to the macro as it is written.
            _ => {
                built.push(node);
                continue;
            }
        };
        let mut modes = vec![Mode3::Keep; xs.len()];
        let mut unwrap = false;
        // Another reading of the same datum.
        let mut again = None;
        match mode {
            Mode3::Keep => {
                built.push(node);
                continue;
            }
            Mode3::Type => {
                unwrap = curly && xs.len() == 1;
                if round && head_is(xs, "->") {
                    // `(-> [A] P R)`: the parameter part and the result.
                    modes.fill(Mode3::Type);
                    if let Some(last) = modes.last_mut() {
                        *last = Mode3::Result;
                    }
                } else {
                    modes.fill(Mode3::Type);
                }
            }
            Mode3::Result => {
                // `(<- R needs)` holds types only, as every other result.
                again = Some(Mode3::Type);
            }
            Mode3::TypeRhs => {
                if curly && xs.len() == 1 && rhs_keeps_curly(&xs[0]) {
                    modes[0] = Mode3::Type;
                } else {
                    again = Some(Mode3::Type);
                }
            }
            Mode3::Pattern => {
                if round && xs.len() == 3 && head_is(xs, ":") {
                    modes[2] = Mode3::Type;
                } else if bits {
                    modes.fill(Mode3::SegmentPattern);
                } else {
                    modes.fill(Mode3::Pattern);
                }
            }
            Mode3::Params => {
                if round && xs.len() == 3 && head_is(xs, "?") {
                    modes[1] = Mode3::Pattern;
                    modes[2] = Mode3::Expr;
                } else {
                    // The items, their keys and their alternatives hold
                    // patterns only.
                    again = Some(Mode3::Pattern);
                }
            }
            Mode3::SegmentExpr | Mode3::SegmentPattern => {
                let value = if matches!(mode, Mode3::SegmentExpr) {
                    Mode3::Expr
                } else {
                    Mode3::Pattern
                };
                if round && xs.len() == 3 && head_is(xs, ":") {
                    modes[1] = value;
                    // `f` or `(f e)`.
                    modes[2] = Mode3::Expr;
                } else {
                    again = Some(value);
                }
            }
            Mode3::ValueItem => {
                if round && xs.len() == 3 && head_is(xs, "=") {
                    modes[2] = Mode3::Expr;
                } else {
                    again = Some(Mode3::Expr);
                }
            }
            Mode3::Statement => {
                if round && xs.len() == 3 && head_is(xs, "=") {
                    modes[1] = Mode3::Pattern;
                    modes[2] = Mode3::Expr;
                } else {
                    again = Some(Mode3::Expr);
                }
            }
            Mode3::Item => {
                if round && xs.len() == 3 && head_is(xs, "@") {
                    // `name`, `(. m name)` or `(name items…)`: a call.
                    modes[1] = Mode3::Expr;
                    modes[2] = Mode3::Item;
                } else if round && xs.len() == 3 && head_is(xs, "=") {
                    if is_type_head(&xs[1]) {
                        modes[1] = Mode3::Type;
                        modes[2] = Mode3::TypeRhs;
                    } else {
                        modes[1] = Mode3::Pattern;
                        modes[2] = Mode3::Expr;
                    }
                } else if round && xs.len() == 3 && head_is(xs, ":") {
                    modes[2] = Mode3::Type;
                } else {
                    // A macro call.
                    again = Some(Mode3::Expr);
                }
            }
            Mode3::Expr => {
                if curly {
                    if is_group(xs) {
                        unwrap = true;
                        modes[0] = Mode3::Expr;
                    } else {
                        modes.fill(Mode3::Statement);
                    }
                } else if bits {
                    modes.fill(Mode3::SegmentExpr);
                } else if !round {
                    // A list.
                    modes.fill(Mode3::Expr);
                } else if head_is(xs, ",") {
                    modes.fill(Mode3::ValueItem);
                } else if head_is(xs, "->") {
                    // `(-> [A] params R body)`; the Square holds names.
                    let n = xs.len();
                    if n >= 4 {
                        modes[n - 3] = Mode3::Params;
                        modes[n - 2] = Mode3::Result;
                        modes[n - 1] = Mode3::Expr;
                    }
                } else if matches!(xs.first().map(Syntax::node), Some(Node::Symbol(_))) {
                    // A chain, a pipe, a provide, a member, an operator.
                    modes.fill(Mode3::Expr);
                } else {
                    // A call: the callee and the items of its data.
                    modes.fill(Mode3::ValueItem);
                    if let Some(first) = modes.first_mut() {
                        *first = Mode3::Expr;
                    }
                }
            }
        }
        if let Some(mode) = again {
            work.push(Task::Enter(node, mode));
            continue;
        }
        let children = xs.clone();
        work.push(Task::Finish(node, children.len(), unwrap));
        for (child, mode) in children.into_iter().zip(modes).rev() {
            work.push(Task::Enter(child, mode));
        }
    }
    built.pop().expect("normalization produced a root")
}
#[cfg(test)]
mod tests {
    use migration_syntax::lisp::source_syntax::*;
    use migration_syntax::lisp::syntax::{Scope, Syntax, Node};
    use migration_syntax::span::Span;
    use migration_syntax::lisp::syntax::ROWS3;
    use migration_syntax::lisp::source_syntax::{normalize_expression as expr, normalize_type as ty};
    use migration_syntax::lisp::source_syntax::{
        source_item_syntax,
    };
    use std::collections::BTreeSet;

    #[test]
    fn accepted_lisp_item_uses_its_parsed_kind() {
        // Syntax-three empty data is `(,)`; the legacy item reader rejects
        // this function head, so the accepted item must not be reparsed there.
        let source = "(test \"quote\" (-> (,) _ { Nil }))";
        let (module, errors) = migration_syntax::lisp::parse_source(source);
        assert!(errors.is_empty(), "{errors:?}");
        let scope = Scope {
            module: 0,
            marks: BTreeSet::new(),
        };
        let syntax = source_item_syntax(&module.items[0], &scope, source, true);
        assert_eq!(
            syntax.show(),
            if ROWS3 {
                "(test \"quote\" (-> (,) _ Nil))"
            } else {
                "(test \"quote\" (-> () _ {Nil}))"
            }
        );
    }

    #[test]
    fn single_field_records_are_not_transparent_groups() {
        let scope = Scope {
            module: 0,
            marks: BTreeSet::new(),
        };
        let datum = migration_syntax::lisp::parse("((: x { T }))").unwrap();
        let syntax = Syntax::from_datum(&datum[0], &scope);
        assert_eq!(ty(&syntax).show(), "((: x T))");
        let datum = migration_syntax::lisp::parse("((: x { y }))").unwrap();
        let syntax = Syntax::from_datum(&datum[0], &scope);
        assert_eq!(expr(&syntax).show(), "((: x y))");
    }

    #[test]
    fn deeply_nested_source_normalizes_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let scope = Scope {
                    module: 0,
                    marks: BTreeSet::new(),
                };
                let datum = migration_syntax::lisp::parse("x").unwrap();
                let mut value = Syntax::from_datum(&datum[0], &scope);
                for _ in 0..20_000 {
                    value = Syntax::make(Node::Curly(vec![value.clone()]), &value).unwrap();
                }
                assert_eq!(expr(&value).show(), "x");
                assert_eq!(ty(&value).show(), "x");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    // ---- The rows of syntax three ----

    fn scope() -> Scope {
        Scope {
            module: 0,
            marks: BTreeSet::new(),
        }
    }

    fn normal3(source: &str, mode: Mode3) -> String {
        let datum = migration_syntax::lisp::parse(source).unwrap();
        assert_eq!(datum.len(), 1, "{source}");
        normalize3(&Syntax::from_datum(&datum[0], &scope()), mode).show()
    }

    #[test]
    fn groups_of_syntax_three_are_removed_by_position() {
        let cases: &[(Mode3, &str, &str)] = &[
            // Expressions: a Curly with one expression is a group.
            (Mode3::Expr, "{ x }", "x"),
            (Mode3::Expr, "{ { x } }", "x"),
            (Mode3::Expr, "(f { x } (= l { y }))", "(f x (= l y))"),
            (Mode3::Expr, "({ f } x)", "(f x)"),
            (
                Mode3::Expr,
                "(, { x } .. { y } (= l { z }))",
                "(, x .. y (= l z))",
            ),
            (Mode3::Expr, "[{ x } .. { y }]", "[x .. y]"),
            (Mode3::Expr, "(+ { a } (* { b } c))", "(+ a (* b c))"),
            (Mode3::Expr, "(|> { e } { f })", "(|> e f)"),
            (
                Mode3::Expr,
                "(<- { e } (, { s } .. { env }))",
                "(<- e (, s .. env))",
            ),
            (Mode3::Expr, "(. { a } b)", "(. a b)"),
            // A block stays; its statements are read.
            (Mode3::Expr, "{ (= x { 1 }) { x } }", "{(= x 1) x}"),
            (Mode3::Expr, "{ { a b } }", "{a b}"),
            (Mode3::Expr, "{ (f) { g } }", "{(f) g}"),
            // A function: the body is one expression, a block or not.
            (Mode3::Expr, "(-> (, x) _ { x })", "(-> (, x) _ x)"),
            (
                Mode3::Expr,
                "(-> (, x) _ { (= y x) y })",
                "(-> (, x) _ {(= y x) y})",
            ),
            (Mode3::Expr, "(-> (, x) _ (f { x }))", "(-> (, x) _ (f x))"),
            (
                Mode3::Expr,
                "(-> (, (: x { Int }) (= k (: y { T }))) { R } e)",
                "(-> (, (: x Int) (= k (: y T))) R e)",
            ),
            (
                Mode3::Expr,
                "(-> [A N] (, (: x { A })) (<- { A } (, (: db { Db }) .. (: env { N }))) { x })",
                "(-> [A N] (, (: x A)) (<- A (, (: db Db) .. (: env N))) x)",
            ),
            (
                Mode3::Expr,
                "(-> (? (, p (| 0 1)) { (> p 0) }) _ { p })",
                "(-> (? (, p (| 0 1)) (> p 0)) _ p)",
            ),
            (
                Mode3::Expr,
                "(|> e (| (-> (, (Some v)) _ { v }) (-> (, None) _ { 0 })))",
                "(|> e (| (-> (, (Some v)) _ v) (-> (, None) _ 0)))",
            ),
            (
                Mode3::Expr,
                "<<(: { x } (size { 8 })) { y }>>",
                "<< (: x (size 8)) y >>",
            ),
            // A quote is given as it is written.
            (Mode3::Expr, "(f `(g { x }))", "(f `(g {x}))"),
            // Types: a Curly with one type is a group.
            (Mode3::Type, "{ Int }", "Int"),
            (Mode3::Type, "(List { Int })", "(List Int)"),
            (Mode3::Type, "(, { A } (= x { B }))", "(, A (= x B))"),
            (
                Mode3::Type,
                "(| { A } { (-> (, { B }) { C }) })",
                "(| A (-> (, B) C))",
            ),
            (
                Mode3::Type,
                "(-> [A] (| (, { A }) (, A A)) (<- { R } (, { Db } .. { N })))",
                "(-> [A] (| (, A) (, A A)) (<- R (, Db .. N)))",
            ),
            // Patterns have no groups; the types in them do.
            (
                Mode3::Pattern,
                "(, (: x { Int }) (= k p (C (: y { T }))) ..)",
                "(, (: x Int) (= k p (C (: y T))) ..)",
            ),
            (
                Mode3::Pattern,
                "<<(: x (size { n }))>>",
                "<< (: x (size n)) >>",
            ),
            // Items.
            (Mode3::Item, "(= x { 1 })", "(= x 1)"),
            (
                Mode3::Item,
                "(= (, a (: b { Int })) { e })",
                "(= (, a (: b Int)) e)",
            ),
            (
                Mode3::Item,
                "(: f { (-> (, { Int }) Int) })",
                "(: f (-> (, Int) Int))",
            ),
            (
                Mode3::Item,
                "(= U (, { T1 } (= x { T2 })))",
                "(= U (, T1 (= x T2)))",
            ),
            (
                Mode3::Item,
                "(= (U A) { (List { A }) })",
                "(= (U A) (List A))",
            ),
            (Mode3::Item, "(= U Opaque)", "(= U Opaque)"),
            // The Curly of a name whose right side is a data type stays.
            (Mode3::Item, "(= U { (, { T1 } T2) })", "(= U {(, T1 T2)})"),
            (Mode3::Item, "(= U { Opaque })", "(= U {Opaque})"),
            (Mode3::Item, "(= U { { (, T) } })", "(= U {(, T)})"),
            (Mode3::Item, "(= U { (| A B) })", "(= U (| A B))"),
            (
                Mode3::Item,
                "(@ (custom { 1 } (= k { 2 })) (@ macro (= m (-> (, x) _ { x }))))",
                "(@ (custom 1 (= k 2)) (@ macro (= m (-> (, x) _ x))))",
            ),
            (
                Mode3::Item,
                "(test \"name\" { (-> (,) _ { Nil }) })",
                "(test \"name\" (-> (,) _ Nil))",
            ),
        ];
        for (mode, source, expected) in cases {
            assert_eq!(&normal3(source, *mode), expected, "{source}");
        }
    }

    #[test]
    fn written_atoms_keep_their_spans_and_removed_groups_their_content() {
        let source = "(-> (, x) _ { (+ { x } 1) })";
        let datum = migration_syntax::lisp::parse(source).unwrap();
        let syntax = normalize3(&Syntax::from_datum(&datum[0], &scope()), Mode3::Expr);
        let Node::Round(parts) = syntax.node() else {
            panic!()
        };
        assert_eq!(syntax.span(), Span::new(0, source.len() as u32));
        let Node::Round(sum) = parts[3].node() else {
            panic!()
        };
        let text = |s: &Syntax| &source[s.span().start as usize..s.span().end as usize];
        assert_eq!(text(&parts[3]), "(+ { x } 1)");
        assert_eq!(text(&sum[0]), "+");
        assert_eq!(text(&sum[1]), "x");
        assert_eq!(text(&sum[2]), "1");
    }

    /// The table gives what the reader and the datum builder of syntax three
    /// give for the same text: the tree without groups, written as datums.
    #[test]
    fn the_table_agrees_with_the_reader_of_syntax_three() {
        use migration_syntax::syn3::{ast::Module, lisp, surface};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        let mut dirs = vec![
            root.join("std"),
            root.join("lib"),
            root.join("examples"),
            root.join("tests/spec"),
        ];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|e| e == "is") {
                    files.push(path);
                }
            }
        }
        assert!(files.len() >= 10, "only {} Lisp files", files.len());
        let mut checked = 0;
        for path in files {
            let source = std::fs::read_to_string(&path).unwrap();
            let Ok(document) = migration_syntax::lisp::parse(&source) else {
                continue;
            };
            for datum in document.iter() {
                let Ok(item) = lisp::item_from_datum(datum) else {
                    // A header, or a file that is an error on purpose.
                    continue;
                };
                let module = surface::ungrouped(&Module {
                    items: vec![item],
                    ..Module::default()
                });
                let expected = migration_syntax::lisp::show(&lisp::item_datum(&module.items[0]));
                let syntax = Syntax::from_datum(datum, &scope());
                assert_eq!(
                    normalize3(&syntax, Mode3::Item).show(),
                    expected,
                    "{}",
                    path.display()
                );
                checked += 1;
            }
        }
        assert!(checked >= 50, "only {checked} items");
    }

    #[test]
    fn deeply_nested_source_of_syntax_three_normalizes_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let datum = migration_syntax::lisp::parse("x").unwrap();
                let mut value = Syntax::from_datum(&datum[0], &scope());
                for _ in 0..20_000 {
                    value = Syntax::make(Node::Curly(vec![value.clone()]), &value).unwrap();
                }
                for mode in [Mode3::Expr, Mode3::Type, Mode3::Statement, Mode3::ValueItem] {
                    assert_eq!(normalize3(&value, mode).show(), "x");
                }
                let f = Syntax::from_datum(&migration_syntax::lisp::parse("f").unwrap()[0], &scope());
                let mut call = value.clone();
                for _ in 0..20_000 {
                    call = Syntax::make(Node::Round(vec![f.clone(), call]), &f).unwrap();
                }
                let shown = normalize3(&call, Mode3::Expr).show();
                assert_eq!(
                    shown,
                    format!("{}x{}", "(f ".repeat(20_000), ")".repeat(20_000))
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
