//! `Syntax::show_as` in the rows of syntax three (docs/design/syntax-3.md
//! §6.7 and §6.8): the counterparts of the `show_as` cases of `lisp_syntax`,
//! and one case for every position and every row.
#[path = "oracle.rs"]
mod ilang_syntax;
use ilang_syntax::lisp::{
    parse,
    syntax::{Node, Position, Scope, Syntax, TypeHandle, validate_expansion},
};
use std::collections::BTreeSet;

fn scope() -> Scope {
    Scope {
        module: 7,
        marks: BTreeSet::new(),
    }
}

fn root(source: &str) -> Syntax {
    let document = parse(source).unwrap();
    assert_eq!(document.len(), 1, "{source}");
    Syntax::from_datum(&document[0], &scope())
}

fn shown(source: &str, position: Position) -> String {
    root(source)
        .show_as_rows3(position)
        .unwrap_or_else(|e| panic!("{source} as {position:?}: {e:?}"))
}

fn refused(source: &str, position: Position) {
    assert!(
        root(source).show_as_rows3(position).is_err(),
        "{source} as {position:?} must not be written"
    );
}

#[test]
fn the_numbers_of_the_positions_follow_section_6_7() {
    use Position::*;
    let order = [
        ImportItem,
        ExportItem,
        Item,
        Attribute,
        Type,
        TypeItem,
        FunctionResult,
        Expression,
        ValueItem,
        Statement,
        Pattern,
        PatternItem,
        Segment,
        Format,
    ];
    assert_eq!(order, Position::ALL);
    for (id, position) in order.into_iter().enumerate() {
        assert_eq!(position.id(), id);
        assert_eq!(Position::from_id(id), Some(position));
    }
    assert_eq!(Position::from_id(14), None);
}

#[test]
fn show_as_checks_the_requested_syntactic_position() {
    assert_eq!(shown("(+ 1 2)", Position::Expression), "1 + 2");
    refused("`(f x)", Position::Expression);
    refused("(f `x)", Position::Expression);
    // The empty Round has no position; the empty data is `(,)`.
    refused("()", Position::Type);
    refused("()", Position::Expression);
    refused("()", Position::Pattern);
    assert_eq!(shown("(,)", Position::Type), "()");
    assert_eq!(shown("(,)", Position::Expression), "()");
    assert_eq!(shown("(,)", Position::Pattern), "()");
    // One datum is read for the position that is asked for.
    assert_eq!(shown("(= x 1)", Position::Statement), "x = 1");
    assert_eq!(shown("(= x 1)", Position::ValueItem), "x = 1");
    assert_eq!(shown("(= x 1)", Position::PatternItem), "x = 1");
    assert_eq!(shown("(= x 1)", Position::Item), "x = 1");
    refused("(= x 1)", Position::Expression);
    refused("(= x 1)", Position::Type);
}

#[test]
fn show_as_reports_the_offending_part() {
    let syntax = root("(f (g ()))");
    let error = syntax.show_as_rows3(Position::Expression).unwrap_err();
    assert_eq!(syntax.at_span(error.span).unwrap().show(), "()");
    let syntax = root("(, a `b)");
    let error = syntax.show_as_rows3(Position::Expression).unwrap_err();
    assert_eq!(syntax.at_span(error.span).unwrap().show(), "`b");
}

#[test]
fn show_as_groups_a_binary_operand_under_negation() {
    assert_eq!(shown("(- (+ x y))", Position::Expression), "-{ x + y }");
    assert_eq!(
        shown("(* (+ x y) z)", Position::Expression),
        "{ x + y } * z"
    );
    assert_eq!(shown("((|> a f) x)", Position::Expression), "{ a |> f }(x)");
    assert_eq!(shown("(. 1 f)", Position::Expression), "{ 1 }.f");
}

#[test]
fn show_as_distinguishes_chains_pipes_and_matches() {
    let chain = "(| (-> (, 0) _ 1) (-> (, _) _ 2))";
    assert_eq!(shown(chain, Position::Expression), "(0) { 1 } | (_) { 2 }");
    assert_eq!(
        shown(&format!("(|> s {chain})"), Position::Expression),
        "s |> (0) { 1 } | (_) { 2 }"
    );
    assert_eq!(shown("(|> e f)", Position::Expression), "e |> f");
    // The rows before syntax three are not read.
    refused("(| a f)", Position::Expression);
    refused("(-> (y) Int y)", Position::Expression);
    refused("{ (-> (= a (, x)) (-> (y) Int y)) }", Position::Expression);
    refused(
        "(. s { (-> (= a (, x)) (-> (y) Int y)) })",
        Position::Expression,
    );
    refused("(! x N)", Position::Expression);
    refused("(-> (Int) (! (^ R E) V))", Position::Type);
}

#[test]
fn show_as_deep_trees_use_bounded_host_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let like = root("x");
            let nest = |head: Syntax| {
                let mut tree = like.clone();
                for _ in 0..10000 {
                    tree = Syntax::make(Node::Round(vec![head.clone(), tree]), &like).unwrap();
                }
                tree
            };
            let tree = nest(Syntax::make(Node::Up("T".into()), &like).unwrap());
            let expected = format!("{}x{}", "T(".repeat(10000), ")".repeat(10000));
            assert_eq!(tree.show_as_rows3(Position::Type).unwrap(), expected);
            assert_eq!(tree.show_as_rows3(Position::Pattern).unwrap(), expected);
            // Lists, as an expression, an item of data, a statement and the
            // value of an item. (A nested call costs the printer the square
            // of its depth, `call_depth` at every level, so calls are not
            // the deep case here.)
            let mut list = like.clone();
            for _ in 0..10000 {
                list = Syntax::make(Node::Square(vec![list]), &like).unwrap();
            }
            let expected = format!("{}x{}", "[".repeat(10000), "]".repeat(10000));
            assert_eq!(list.show_as_rows3(Position::Expression).unwrap(), expected);
            assert_eq!(list.show_as_rows3(Position::ValueItem).unwrap(), expected);
            assert_eq!(list.show_as_rows3(Position::Statement).unwrap(), expected);
            assert_eq!(list.show_as_rows3(Position::Pattern).unwrap(), expected);
            let value = Syntax::make(Node::Name("value".into()), &like).unwrap();
            let eq = Syntax::make(Node::Symbol("=".into()), &like).unwrap();
            let item = Syntax::make(Node::Round(vec![eq, value, list]), &like).unwrap();
            assert_eq!(
                item.show_as_rows3(Position::Item).unwrap(),
                format!("value = {expected}")
            );
            // An error deep in the tree comes back to the caller.
            let bad = nest(Syntax::make(Node::Symbol("?".into()), &like).unwrap());
            assert!(bad.show_as_rows3(Position::Expression).is_err());
            // Negations: every level is a group.
            let negated = nest(Syntax::make(Node::Symbol("-".into()), &like).unwrap());
            assert_eq!(
                negated.show_as_rows3(Position::Expression).unwrap(),
                format!("{}-x{}", "-{ ".repeat(9999), " }".repeat(9999))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn show_as_only_breaks_lines_where_the_layout_always_does() {
    assert_eq!(
        shown("(-> (,) _ { x y })", Position::Expression),
        "() {\n\tx\n\ty\n}"
    );
    assert_eq!(
        shown("(-> (,) _ { x (- 1) })", Position::Expression),
        "() {\n\tx\n\t-1\n}"
    );
    // A body that is one expression is written in `{ }` on the line.
    assert_eq!(shown("(-> (,) _ x)", Position::Expression), "() { x }");
    let long = format!("(f {})", "argument_with_a_long_name ".repeat(12));
    let text = shown(&long, Position::Expression);
    assert!(text.len() > 200 && !text.contains('\n'), "{text}");
}

#[test]
fn show_as_attribute_item_uses_only_required_line_breaks() {
    let item = root("(@ custom (= f (-> (,) _ Nil)))");
    let text = item.show_as_rows3(Position::Item).unwrap();
    assert_eq!(text, "@custom\nf = () { Nil }");
    let (module, errors) = ilang_syntax::syn3::parse_module(&format!("{text}\n"));
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items.len(), 1);
    assert_eq!(module.items[0].attributes.len(), 1);

    let data = root("(@ (custom \"a\" (= k 1)) (= (, x y) (, 1 2)))");
    let text = data.show_as_rows3(Position::Item).unwrap();
    assert_eq!(text, "@custom(\"a\", k = 1)\n(x, y) = (1, 2)");
    let (module, errors) = ilang_syntax::syn3::parse_module(&format!("{text}\n"));
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(module.items[0].attributes.len(), 1);
}

#[test]
fn show_as_accepts_expressions_already_written_in_normal_surface() {
    let leaves = [
        "x",
        "1",
        "1.5",
        "\"text\"",
        "(1, 2)",
        "(x = 1, y)",
        "(..base, x = 1)",
        "()",
        "[x, y]",
        "[x, ..rest]",
        "f(x)",
        "f(x, label = y)",
        "m.f(..args)",
        "x + y",
        "-{ x + y }",
        "(x) { x }",
        "(x : Int, y = (a, b)) -> Int { x }",
        "(0) { 1 } | (_) { 2 }",
        "x |> (0) { 1 } | (_) { 2 }",
        "x |> f |> g(1)",
        "{\n\ty = x\n\ty\n}",
        "<<1, 2:size(8)>>",
        "app() <- (db, ..env)",
        "(x) <- (log : Log) { log.info(x) }",
        "<A>(x : A) -> A { x }",
        "(p, q ? p > q) { p }",
        "pair.0",
        "Some(x)",
    ];
    let mut checked = 0;
    for leaf in leaves {
        for source in [
            leaf.to_string(),
            format!("f({leaf})"),
            format!("-{{ {leaf} }}"),
            format!("[{leaf}]"),
            format!("{{ {leaf} }} + z"),
            format!("({leaf}, z)"),
            format!("() {{ {leaf} }}"),
            format!("z |> {{ {leaf} }}"),
        ] {
            let Ok(expr) = ilang_syntax::syn3::parse_expr(&source) else {
                continue;
            };
            let datum = ilang_syntax::syn3::lisp::expr_datum(&expr);
            let syntax = Syntax::from_datum(&datum, &scope());
            let text = syntax
                .show_as_rows3(Position::Expression)
                .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
            // The text is read as the same datum.
            let back = ilang_syntax::syn3::parse_expr(&text)
                .unwrap_or_else(|error| panic!("{source:?} -> {text:?}: {error:?}"));
            let again = Syntax::from_datum(&ilang_syntax::syn3::lisp::expr_datum(&back), &scope())
                .show_as_rows3(Position::Expression)
                .unwrap();
            assert_eq!(text, again, "{source:?}");
            checked += 1;
        }
    }
    assert!(checked >= 150, "only {checked} source expressions parsed");
}

#[test]
fn typed_macro_type_view_has_no_text() {
    let like = root("Int");
    let view = Syntax::type_view(
        TypeHandle {
            snapshot: 19,
            index: 3,
        },
        &like,
    );
    assert!(view.show_as_rows3(Position::Type).is_err());
    let nested = Syntax::make(Node::Round(vec![root("List"), view.clone()]), &like).unwrap();
    let error = nested.show_as_rows3(Position::Type).unwrap_err();
    assert!(error.message.contains("TypeView"), "{error:?}");
    assert!(nested.show_as_rows3(Position::Expression).is_err());
}

/// Every row of §6.8, and the rows that it leaves as they were, by position.
#[test]
fn show_as_writes_every_row_in_its_position() {
    use Position::*;
    let cases: &[(Position, &str, &str)] = &[
        // Files and items.
        (ImportItem, "(/ std list)", "std/list"),
        (ImportItem, "(= l (/ std list))", "l = std/list"),
        (
            ImportItem,
            "(= (, Map get) (/ std map))",
            "(Map, get) = std/map",
        ),
        (
            ImportItem,
            "(= m (, Map get) (/ std map))",
            "m = (Map, get) = std/map",
        ),
        (ExportItem, "name", "name"),
        (ExportItem, "Type", "Type"),
        (ExportItem, "(Type ..)", "Type(..)"),
        (Item, "(= U (, T1 (= x T2)))", "U = (T1, x = T2)"),
        (Item, "(= (U A) (, A))", "U(A) = (A)"),
        (Item, "(= U (,))", "U = ()"),
        (Item, "(= U Opaque)", "U = Opaque"),
        (Item, "(= (U A) Opaque)", "U(A) = Opaque"),
        (Item, "(= U T)", "U = T"),
        (Item, "(= (U A) (List A))", "U(A) = List(A)"),
        (Item, "(= U { (, T1 T2) })", "U = { (T1, T2) }"),
        (Item, "(= T (| C1 C2))", "T = C1 | C2"),
        (Item, "(: f (-> (, Int) Int))", "f : (Int) -> Int"),
        (Item, "(= x 1)", "x = 1"),
        (
            Item,
            "(test \"name\" (-> (,) _ Nil))",
            "test(\"name\", () { Nil })",
        ),
        (Item, "((. t test) \"name\")", "t.test(\"name\")"),
        (Item, "{ (= x 1) (= y 2) }", "x = 1\ny = 2"),
        (
            Item,
            "(= connect (-> (, (: host String) (= port (: p Int))) Conn (open host p)))",
            "connect = (host : String, port = p : Int) -> Conn { open(host, p) }",
        ),
        (
            Item,
            "(= pick (| (-> (, 0 y) _ y) (-> (, x _) _ x)))",
            "pick = (0, y) { y } | (x, _) { x }",
        ),
        (Attribute, "macro", "@macro"),
        (Attribute, "(. m custom)", "@m.custom"),
        (
            Attribute,
            "(external \"env\" \"f\")",
            "@external(\"env\", \"f\")",
        ),
        (Attribute, "((. m custom) (= k 1))", "@m.custom(k = 1)"),
        // Types.
        (Type, "Int", "Int"),
        (Type, "x", "x"),
        (Type, "_", "_"),
        (Type, "\"tag\"", "\"tag\""),
        (Type, "(- 1)", "-1"),
        (Type, "(, T1 (= x T2))", "(T1, x = T2)"),
        (Type, "(, T)", "(T)"),
        (Type, "(,)", "()"),
        (Type, "(List Int)", "List(Int)"),
        (Type, "(. m T)", "m.T"),
        (Type, "((. m Map) K V)", "m.Map(K, V)"),
        (Type, "(f Int)", "f(Int)"),
        (Type, "(| A B)", "A | B"),
        (Type, "(& A (| B C))", "A & { B | C }"),
        (Type, "(- A B)", "A - B"),
        (Type, "(-> (, T1 (= x T2)) R)", "(T1, x = T2) -> R"),
        (Type, "(-> (,) R)", "() -> R"),
        (
            Type,
            "(-> (, T) (<- R (, Db Log)))",
            "(T) <- (Db, Log) -> R",
        ),
        (Type, "(-> (, T) (<- R N))", "(T) <- N -> R"),
        (
            Type,
            "(-> (, T) (<- R (, .. N Db)))",
            "(T) <- (..N, Db) -> R",
        ),
        (Type, "(-> [A] (, A) A)", "<A>(A) -> A"),
        (Type, "(-> [A N] (, A) (<- A N))", "<A, N>(A) <- N -> A"),
        (Type, "(-> (| (, T) (, T T)) R)", "{ (T) | (T, T) } -> R"),
        (Type, "(-> Args R)", "{ Args } -> R"),
        (Type, "{ Int }", "Int"),
        (TypeItem, "T", "T"),
        (TypeItem, "(= label T)", "label = T"),
        (TypeItem, "(-> (, A) B)", "(A) -> B"),
        (FunctionResult, "R", "-> R"),
        (FunctionResult, "_", "-> _"),
        (FunctionResult, "(List Int)", "-> List(Int)"),
        (FunctionResult, "(<- R (, Db Log))", "<- (Db, Log) -> R"),
        (FunctionResult, "(<- R N)", "<- N -> R"),
        (FunctionResult, "(<- R (, .. N Db))", "<- (..N, Db) -> R"),
        (FunctionResult, "(<- A N)", "<- N -> A"),
        (
            FunctionResult,
            "(<- User (, (: db Db)))",
            "<- (db : Db) -> User",
        ),
        (FunctionResult, "(<- _ (, (: log Log)))", "<- (log : Log)"),
        (FunctionResult, "(<- _ (, .. (: env N)))", "<- (..env : N)"),
        (
            FunctionResult,
            "(<- _ (, (: db Db) .. (: env N)))",
            "<- (db : Db, ..env : N)",
        ),
        (FunctionResult, "(<- _ N)", "<- N -> _"),
        // Expressions and statements.
        (Expression, "(f e1 (= x e2))", "f(e1, x = e2)"),
        (Expression, "(f)", "f()"),
        (Expression, "(f .. e)", "f(..e)"),
        (Expression, "(, e1 (= x e2))", "(e1, x = e2)"),
        (Expression, "(, e)", "(e)"),
        (Expression, "(, .. e (= x v))", "(..e, x = v)"),
        (Expression, "(-> (, p1 (= x p2)) _ e)", "(p1, x = p2) { e }"),
        (Expression, "(-> (, p) R e)", "(p) -> R { e }"),
        (Expression, "(-> (,) _ e)", "() { e }"),
        (
            Expression,
            "(-> (, (: id String)) (<- User (, (: db Db))) e)",
            "(id : String) <- (db : Db) -> User { e }",
        ),
        (
            Expression,
            "(-> (, x) (<- _ (, (: log Log))) e)",
            "(x) <- (log : Log) { e }",
        ),
        (
            Expression,
            "(-> (, f) (<- _ (, .. (: env N))) e)",
            "(f) <- (..env : N) { e }",
        ),
        (
            Expression,
            "(-> [A N] (, (: x A)) A e)",
            "<A, N>(x : A) -> A { e }",
        ),
        (Expression, "(<- e (, s1 .. env))", "e <- (s1, ..env)"),
        (Expression, "(<- (app) (live))", "app() <- live()"),
        (Expression, "(-> (? (, p1 p2) g) _ e)", "(p1, p2 ? g) { e }"),
        (
            Expression,
            "(-> (, (| 0 1) (= k (| a b))) _ e)",
            "(0 | 1, k = a | b) { e }",
        ),
        (
            Expression,
            "(| (-> (, a) _ a) (-> (, b) _ b))",
            "(a) { a } | (b) { b }",
        ),
        (Expression, "(|> e f)", "e |> f"),
        (
            Expression,
            "(|> opt (| (-> (? (, (Some v)) ((. string is_empty) v)) _ \"empty\") (-> (, (Some v)) _ v) (-> (, None) _ \"none\")))",
            "opt |> (Some(v) ? string.is_empty(v)) { \"empty\" } | (Some(v)) { v } | (None) { \"none\" }",
        ),
        (Expression, "(. a b)", "a.b"),
        (Expression, "(. m C)", "m.C"),
        (Expression, "(. pair 0)", "pair.0"),
        (Expression, "[a b .. rest]", "[a, b, ..rest]"),
        (
            Expression,
            "<<1 (: x (size 8)) (: y utf8)>>",
            "<<1, x:size(8), y:utf8>>",
        ),
        (Expression, "(<> \"a\" b)", "\"a\" <> b"),
        (Expression, "(&& a (|| b c))", "a && { b || c }"),
        (Expression, "(-. x)", "-.x"),
        (Expression, "{ (= t (, a b)) t }", "{\n\tt = (a, b)\n\tt\n}"),
        (ValueItem, "e", "e"),
        (ValueItem, "(= label e)", "label = e"),
        (ValueItem, "(f x)", "f(x)"),
        (ValueItem, "(= label (+ a b))", "label = a + b"),
        (Statement, "(= x (f y))", "x = f(y)"),
        (Statement, "(= (= t (, a b)) e)", "t = (a, b) = e"),
        (Statement, "(f y)", "f(y)"),
        (Statement, "(= (, a b) (, 1 2))", "(a, b) = (1, 2)"),
        // Patterns.
        (Pattern, "(, p1 (= x p2))", "(p1, x = p2)"),
        (Pattern, "(, p ..)", "(p, ..)"),
        (Pattern, "(, p .. r)", "(p, ..r)"),
        (Pattern, "C", "C"),
        (Pattern, "(C p1 (= x p2))", "C(p1, x = p2)"),
        (Pattern, "(C p ..)", "C(p, ..)"),
        (Pattern, "((. m C) p)", "m.C(p)"),
        (Pattern, "(= p1 p2 p3)", "p1 = p2 = p3"),
        (
            Pattern,
            "(, (= size whole (, w h)))",
            "(size = whole = (w, h))",
        ),
        (Pattern, "(, (= (, a b) args))", "((a, b) = args)"),
        (Pattern, "(: x Int)", "x : Int"),
        (Pattern, "_", "_"),
        (Pattern, "(- 1)", "-1"),
        (Pattern, "\"text\"", "\"text\""),
        (Pattern, "(<> \"pre\" rest)", "\"pre\" <> rest"),
        (Pattern, "[x .. rest]", "[x, ..rest]"),
        (Pattern, "[x ..]", "[x, ..]"),
        (Pattern, "<<1 (: x (size 8))>>", "<<1, x:size(8)>>"),
        // The parameter part of a function is in the position `Pattern`.
        (Pattern, "(, (| p1 p2) y)", "(p1 | p2, y)"),
        (Pattern, "(? (, p1 p2) (> p1 p2))", "(p1, p2 ? p1 > p2)"),
        (Pattern, "(? (,) ready)", "(? ready)"),
        (PatternItem, "p", "p"),
        (PatternItem, "(= key p)", "key = p"),
        (
            PatternItem,
            "(= size whole (, w h))",
            "size = whole = (w, h)",
        ),
        (PatternItem, "(= (, a b) args)", "(a, b) = args"),
        (PatternItem, "..", ".."),
        (PatternItem, "(| p1 p2)", "p1 | p2"),
        (PatternItem, "(= key (| p1 p2))", "key = p1 | p2"),
        (PatternItem, "(: x T)", "x : T"),
        (PatternItem, "(C p)", "C(p)"),
        // Bit arrays.
        (Segment, "x", "x"),
        (Segment, "(: x utf8)", "x:utf8"),
        (Segment, "(: (+ a b) (size 8))", "a + b:size(8)"),
        (Segment, "(: _ (size n))", "_:size(n)"),
        (Format, "utf8", "utf8"),
        (Format, "(size 8)", "size(8)"),
        (Format, "(size (* n 8))", "size(n * 8)"),
    ];
    for (position, lisp, normal) in cases {
        assert_eq!(&shown(lisp, *position), normal, "{lisp} as {position:?}");
    }
}

#[test]
fn show_as_refuses_what_a_position_does_not_read() {
    use Position::*;
    for (position, lisp) in [
        (ImportItem, "name"),
        (ImportItem, "(/ one)"),
        (ExportItem, "(f x)"),
        (ExportItem, "1"),
        (Item, "x"),
        (Item, "(= x)"),
        (Item, "{ (= x 1) y }"),
        (Attribute, "1"),
        (Attribute, "(1 2)"),
        (Type, "(, .. N)"),
        (Type, "(| A)"),
        (Type, "1.5"),
        (TypeItem, ".."),
        (TypeItem, "(= 1 T)"),
        (FunctionResult, "(<- R)"),
        (FunctionResult, "(<- R (, (: db Db) Log))"),
        (Expression, "_"),
        (Expression, "(| f)"),
        (Expression, "(-> (, x) _)"),
        (Expression, "(-> (, x ..) _ e)"),
        (Expression, "{ (= x 1) }"),
        (Expression, "{}"),
        (ValueItem, ".."),
        (Statement, "(= x)"),
        (Pattern, "(| p1 p2)"),
        (Pattern, "(f x)"),
        (Pattern, "(? p g)"),
        (Pattern, "(? (, p ..) g)"),
        (Pattern, "(, p .. r s)"),
        (PatternItem, "(f x)"),
        (Segment, "(: x 1)"),
        (Format, "1"),
        (Format, "(size 1 2)"),
    ] {
        refused(lisp, position);
    }
}

/// `validate_expansion` and `match_quote` do not read rows: they see the
/// attribute form and the brackets, which syntax three keeps. These are the
/// cases of `lisp_syntax` with the forms of §6.8.
#[test]
fn quotes_and_expansion_checks_work_on_the_rows_of_syntax_three() {
    use ilang_syntax::lisp::syntax::Substitution;
    let pattern = root("`(-> (, ~..params) ~result ~body)");
    let value = root("(-> (, x (= k y)) (<- _ (, (: log Log))) (f x))");
    let captures = pattern.match_quote(&value).unwrap().unwrap();
    assert_eq!(captures.len(), 3);
    assert!(matches!(&captures[0].value, Substitution::Many(items) if items.len() == 2));
    let Substitution::Many(items) = &captures[0].value else {
        panic!()
    };
    assert_eq!(
        items[1].show_as_rows3(Position::PatternItem).unwrap(),
        "k = y"
    );
    let Substitution::One(result) = &captures[1].value else {
        panic!()
    };
    assert_eq!(
        result.show_as_rows3(Position::FunctionResult).unwrap(),
        "<- (log : Log)"
    );
    // A pattern of the rows before syntax three takes the `,` as a parameter.
    let old = root("`(-> (~..params) ~result ~body)");
    let captures = old.match_quote(&value).unwrap().unwrap();
    assert!(matches!(&captures[0].value, Substitution::Many(items) if items.len() == 3));
    assert!(
        root("`(-> () _ ~body)")
            .match_quote(&root("(-> (,) _ x)"))
            .unwrap()
            .is_none()
    );
    assert!(root("`(-> (, ~..ps x) _ ~b)").match_quote(&value).is_err());

    let argument = root("(@ (external \"env\" \"f\") (: f (-> (, Int) Int)))");
    validate_expansion(&argument, std::slice::from_ref(&argument)).unwrap();
    let rebuilt = root("(@ (external \"env\" \"f\") (: f (-> (, Int) Int)))");
    assert!(validate_expansion(&rebuilt, std::slice::from_ref(&argument)).is_err());
    assert!(validate_expansion(&root("(@ macro (= f (-> (,) _ x)))"), &[]).is_err());
    assert!(validate_expansion(&root("(-> (, x) _ `x)"), &[]).is_err());
    validate_expansion(&root("(= f (-> (, x) (<- _ (, (: db Db))) x))"), &[]).unwrap();
}

#[test]
fn source_tree_keeps_operator_tokens_inside_interpolation() {
    use ilang_syntax::lisp::source_tree::SourceTree;
    use ilang_syntax::syn3::ast::ItemKind;
    for source in [
        "f = (x) { x + 2 }",
        "f = (_x) { _x + 2 }",
        r#"f = (x) { "a\(x + 2)b" }"#,
    ] {
        let (module, diagnostics) = ilang_syntax::syn3::parse_module(source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let ItemKind::Bind(binding) = &module.items[0].kind else {
            panic!()
        };
        let syntax = SourceTree::new(&scope(), source)
            .expression(binding.value.span.0)
            .unwrap();
        let mut pending = vec![&syntax];
        let mut operators = 0;
        while let Some(node) = pending.pop() {
            match node.node() {
                Node::Round(xs) | Node::Curly(xs) | Node::Square(xs) | Node::Bits(xs) => {
                    pending.extend(xs)
                }
                Node::Discard(name) if name == "_x" => {
                    panic!("a single underscore starts a name, not a discard")
                }
                Node::Symbol(symbol) if symbol == "+" => {
                    let span = node.span();
                    assert_eq!(&source[span.start as usize..span.end as usize], "+");
                    operators += 1;
                }
                _ => {}
            }
        }
        assert_eq!(operators, 1);
    }
}

#[test]
fn written_sum_member_visibility_survives_transition_absorption() {
    use ilang_syntax::lisp::source_tree::SourceTree;
    for (exports, expected) in [
        ("T", (false, false)),
        ("C", (true, false)),
        ("C(..)", (true, true)),
        ("T(..)", (true, true)),
    ] {
        let source = format!("({exports})\nC = (Int)\nD = ()\nT = C | D\n");
        let tree = SourceTree::new(&scope(), &source);
        assert_eq!(tree.definition_visibility("C"), Some(expected), "{exports}");
        let (_, datum) = tree.definition("C").unwrap();
        assert_eq!(ilang_syntax::lisp::show(&datum), "(= C (, Int))");
    }
}

#[test]
fn source_tree_index_keeps_the_written_operator_and_transition_literal_span() {
    use ilang_syntax::ast::{ExprKind, ItemKind};
    use ilang_syntax::lisp::source_tree::SourceTree;
    let source = "value = pair.12";
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let ItemKind::Binding(binding) = &module.items[0].kind else {
        panic!()
    };
    let ExprKind::TupleIndex { index_span, .. } = binding.value.kind else {
        panic!()
    };
    let syntax = SourceTree::new(&scope(), source)
        .expression(binding.value.span)
        .unwrap();
    let Node::Round(parts) = syntax.node() else {
        panic!()
    };
    let span = parts[0].span();
    assert_eq!(&source[span.start as usize..span.end as usize], ".");
    assert_eq!(parts[2].span(), index_span);
}

#[test]
fn source_tree_finds_the_original_multi_shape_argument() {
    use ilang_syntax::{ast, lisp::source_tree::SourceTree};
    let source = "f = (x) { x } | (x, y) { x + y }";
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let ast::ItemKind::Binding(binding) = &module.items.last().unwrap().kind else {
        panic!()
    };
    let syntax = SourceTree::new(&scope(), source)
        .expression(binding.value.span)
        .unwrap();
    assert!(
        syntax.show().starts_with("(| (-> (, x)"),
        "{}",
        syntax.show()
    );
    assert!(!syntax.show().contains("%multi"));
}

#[test]
fn item_result_keeps_its_sum_absorption_boundary() {
    use ilang_syntax::lisp::{
        parse,
        scoped::ScopedSource,
        syntax::{Scope, Syntax},
    };
    let source = "C = ()\n";
    let (mut native, errors) = ilang_syntax::syn3::parse_module(source);
    assert!(errors.is_empty());
    let (mut old, errors) = ilang_syntax::syn3::lower::lower(&native);
    assert!(errors.is_empty());
    let mut written = ilang_syntax::syn3::lower::Transition::default();
    written.record_items(&mut old.items);
    let scope = Scope {
        module: 0,
        marks: Default::default(),
    };
    let result = Syntax::from_datum(&parse("(= T C)").unwrap()[0], &scope);
    let call = Syntax::from_datum(&parse("m").unwrap()[0], &scope);
    let mut before = ScopedSource::new(source.into());
    old.items.extend(before.items(&result, &call).unwrap());
    let mut after = ScopedSource::new(source.into());
    native.items.extend(after.items3(&result, &call).unwrap());
    let (lowered, errors) = ilang_syntax::syn3::lower::lower_with_transition(
        &native,
        false,
        Some(after.original_len()),
        &written,
        Some(after.transition()),
    );
    assert!(errors.is_empty());
    assert_eq!(
        ilang_syntax::print::print_module(&old),
        ilang_syntax::print::print_module(&lowered)
    );
}
