//! Syntax three (docs/design/syntax-3.md §2, §3, §6.8, with the services
//! amendment): each production and each rule P1 to P13, the examples of the
//! note, and the examples of the services proposal.

use ilang_syntax::syn3::ast::*;
use ilang_syntax::syn3::{self, convert, lisp};

/// Parses with no error, and checks `parse(print(t)) == t`, that the
/// formatter gives text that reads as the same tree and is stable, and the
/// same through the Lisp surface.
fn ok(source: &str) -> Module {
    let (module, diags) = syn3::parse_module(source);
    assert!(diags.is_empty(), "{source}\n{diags:#?}");
    let printed = syn3::print_module(&module);
    let (again, diags) = syn3::parse_module(&printed);
    assert!(diags.is_empty(), "printed:\n{printed}\n{diags:#?}");
    assert_eq!(again, module, "printed:\n{printed}");
    let formatted = syn3::format(source).expect("formats");
    let (again, diags) = syn3::parse_module(&formatted);
    assert!(diags.is_empty(), "formatted:\n{formatted}\n{diags:#?}");
    assert_eq!(again, module, "formatted:\n{formatted}");
    assert_eq!(
        syn3::format(&formatted).unwrap(),
        formatted,
        "the formatter is stable"
    );
    let text = lisp::to_lisp(&module);
    let (read, diags) = lisp::from_lisp(&text);
    assert!(diags.is_empty(), "lisp:\n{text}\n{diags:#?}");
    assert_eq!(lisp::to_lisp(&read), text, "lisp:\n{text}");
    module
}

fn bad(source: &str, needle: &str) {
    let (_, diags) = syn3::parse_module(source);
    assert!(
        diags.iter().any(|d| d.message.contains(needle)),
        "{source}\nexpected an error with `{needle}`, got {diags:#?}"
    );
}

fn value(source: &str) -> Expr {
    let module = ok(source);
    match &module.items.last().unwrap().kind {
        ItemKind::Bind(b) => b.value.clone(),
        other => panic!("not a binding: {other:?}"),
    }
}

/// The printed form of an expression read from `source`.
fn expr(source: &str) -> String {
    syn3::print_expr(&syn3::parse_expr(source).unwrap_or_else(|d| panic!("{source}\n{d:#?}")))
}

fn ty(source: &str) -> String {
    syn3::print_type(&syn3::parse_type(source).unwrap_or_else(|d| panic!("{source}\n{d:#?}")))
}

fn pat(source: &str) -> String {
    syn3::print_pattern(&syn3::parse_pattern(source).unwrap_or_else(|d| panic!("{source}\n{d:#?}")))
}

fn chain(e: &Expr) -> &[Function] {
    match &e.kind {
        ExprKind::Chain(fs) => fs,
        other => panic!("not a chain: {other:?}"),
    }
}

fn lisp_of(source: &str) -> String {
    lisp::to_lisp(&ok(source)).trim_end().to_string()
}

// ---- §3.1 Types ----

#[test]
fn types() {
    for t in [
        "Int",
        "a.User",
        "List(Int)",
        "a.Map(String, Int)",
        "size",
        "m.size",
        "m.codec(Int, 3)",
        "_",
        "1",
        "-1",
        "\"a\"",
        "()",
        "(Int)",
        "(Int, String)",
        "(name = String, age = Int)",
        "(String, port = Int)",
        "(Int, Int) -> Int",
        "() -> Int",
        "(String, port = Int) -> Conn",
        "{ (Float) | (Float, Float) } -> Float",
        "((Float) | (Float, Float)) -> Float",
        "A | B | C",
        "A & B",
        "A - B - C",
        "A | B & C - D",
        "{ A | B } & C",
        "{ (Int) -> Int } | Nil",
        "Nil | (Int) -> Int",
        "(Int) -> (Int) -> Int",
        "((Int) -> Int) -> Int",
    ] {
        assert_eq!(ty(t), t);
    }
    // A group leaves no node.
    assert_eq!(ty("{ Int }"), "Int");
    assert_eq!(ty("{ (Int) } -> Int"), "(Int) -> Int");
    assert_eq!(ty("- { 1 }"), "-1");
}

#[test]
fn service_types() {
    for t in [
        "(String) <- (Db, Fail(NotFound), Log) -> User",
        "(A) <- N -> B",
        "() <- (..N, Fail(E)) -> A",
        "<A>(A) -> A",
        "<A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)",
        "<A>{ (A) | (A, A) } <- (Log) -> A",
        "(write = <A>(A) -> Nil)",
    ] {
        assert_eq!(ty(t), t);
    }
}

#[test]
fn type_errors() {
    // P10: `:` inside a type.
    bad("f: (a: Int) -> Int", "`:` is not written in a type");
    bad("T = (name: String)", "`:` is not written in a type");
    // P0: `^` and `!` alone are no symbols.
    bad("f: (Int) -> Int ^ E", "`^` is not a symbol of i");
    bad("f: (Int) -> Int ! Log", "`!` is not a symbol of i");
    ok("x = a != b\n");
    bad("f: (..N) -> Int", "a data type has no spread");
    // §2.3: a positional item may not follow a labelled item.
    bad(
        "f: (port = Int, String) -> Conn",
        "by position may not come after",
    );
}

// ---- §3.2 Items and type definitions ----

#[test]
fn type_definitions() {
    let m = ok(
        "Some(A) = (A)\nNone = ()\nUser = (name = String, age = Int)\nOption(A) = Some(A) | None\nMeters = Int\nPoint = { (Int, Int) }\nWrap = (Int) | Nil\nHandler = (Request) -> Response\nElement = Opaque\nHandle(A) = Opaque\nCodec(Size, _) = size\n",
    );
    let kinds: Vec<&TypeRhs> = m
        .items
        .iter()
        .map(|i| match &i.kind {
            ItemKind::TypeDef(d) => &d.rhs,
            _ => unreachable!(),
        })
        .collect();
    // §2.8: the kind follows the outermost written form.
    assert!(matches!(kinds[0], TypeRhs::Data(items) if items.len() == 1));
    assert!(matches!(kinds[1], TypeRhs::Data(items) if items.is_empty()));
    assert!(matches!(kinds[2], TypeRhs::Data(items) if items.len() == 2));
    assert!(matches!(kinds[3], TypeRhs::Type(t) if matches!(t.kind, TypeKind::Union(_))));
    assert!(matches!(kinds[4], TypeRhs::Type(t) if matches!(t.kind, TypeKind::Named { .. })));
    // G16: `{ (Int, Int) }` is a name, not a labelled type.
    assert!(matches!(kinds[5], TypeRhs::Type(t) if matches!(t.kind, TypeKind::Data(_))));
    // G17.
    assert!(matches!(kinds[6], TypeRhs::Type(t) if matches!(t.kind, TypeKind::Union(_))));
    assert!(matches!(kinds[7], TypeRhs::Type(t) if matches!(t.kind, TypeKind::Fn(_))));
    assert!(matches!(kinds[8], TypeRhs::Opaque));
    assert!(matches!(kinds[9], TypeRhs::Opaque));
    assert!(matches!(kinds[10], TypeRhs::Type(t) if matches!(t.kind, TypeKind::ValueOf { .. })));
    assert_eq!(
        syn3::print_module(&m),
        "Some(A) = (A)\n\nNone = ()\n\nUser = (name = String, age = Int)\n\nOption(A) = Some(A) | None\n\nMeters = Int\n\nPoint = { (Int, Int) }\n\nWrap = (Int) | Nil\n\nHandler = (Request) -> Response\n\nElement = Opaque\n\nHandle(A) = Opaque\n\nCodec(Size, _) = size\n"
    );
    bad(
        "Tree(A) { Leaf  Node(A) }",
        "the constructor list `Name { … }` is removed",
    );
}

#[test]
fn node_of_i_syntax() {
    ok(
        "Node = (Name | Discard | Up | Symbol | Int | Float | Text | Round | Square | Curly | Bits | Quote | Unquote | Splice)\nName = (String)\nRound = (List(Syntax))\nQuote = (Syntax)\n",
    );
}

#[test]
fn header_items_declarations_attributes() {
    let m = ok(
        "//// A module.\n(\n\ti/fail\n\treq = i/request\n\t(Option, Some, None) = std/option\n)\n\n(\n\tUser(..)\n\tOption\n\tfind\n)\n\n/// Reads.\n@external(\"i:fs/v1\", \"read\")\nread: (String) <- (Fail(String)) -> String\n\n@host(\"app:storage/v1\", \"async\")\nStorage = (read = (String) <- (Fail(String)) -> String)\n\n@macro\nid = (x) { x }\n\ntest.suite(\"name\", () { 1 })\n",
    );
    assert_eq!(m.doc, [" A module."]);
    assert_eq!(m.imports.len(), 3);
    assert_eq!(m.exports.len(), 3);
    assert!(m.exports[0].constructors);
    assert_eq!(m.items[0].doc, [" Reads."]);
    assert!(matches!(m.items[0].kind, ItemKind::Decl { .. }));
    assert!(matches!(m.items[3].kind, ItemKind::Macro { .. }));
    // Type parameters are on the function type, not on the name.
    ok("first: <A>(List(A)) -> Option(A)\n");
    bad("x = 1\n(a, b)\n", "imports or the exports");
}

// ---- §3.3 Expressions ----

#[test]
fn data_values() {
    for e in [
        "(1, \"a\")",
        "(name = \"ada\", age = 36)",
        "(\"a.com\", port = 443)",
        "()",
        "(pair)",
        "(..ada, age = 37)",
        "(..pair, True)",
        "User(..u, age = 37)",
        "f()",
        "f(x)",
        "f(a, port = 1)",
        "f(..t)",
        "d.0",
        "d.f",
        "m.User(name = \"ada\")",
        "[1, 2, ..rest]",
        "[]",
        "<<1, x:size(8), y:utf8>>",
    ] {
        assert_eq!(expr(e), e);
    }
    // P3: `==` is one token, so the item is positional.
    assert!(matches!(
        syn3::parse_expr("(a == b)").unwrap().kind,
        ExprKind::Data(items) if matches!(items[0], DItem::Pos(_))
    ));
    bad(
        "x = (port = 443, \"a.com\")",
        "by position may not come after",
    );
    bad("x = f(name: 1)", "written `label = value`");
}

#[test]
fn operators_and_groups() {
    for e in [
        "a + b * c",
        "{ a + b } * c",
        "a - { b - c }",
        "a - b - c",
        "-a",
        "-.x",
        "-{ a + b }",
        "a || b && c == d",
        "a <> b <> c",
        "{ a - 48 } * 100",
        "x < -1",
    ] {
        assert_eq!(expr(e), e);
    }
    // A block with only an expression is a group, and stays one.
    assert!(matches!(
        syn3::parse_expr("{ a }").unwrap().kind,
        ExprKind::Block(b) if b.stmts.len() == 1
    ));
}

#[test]
fn p1_function_or_data() {
    // `)` followed by `->`, or by `{` on the same line: a function.
    assert_eq!(
        chain(&value("f = (a: Int, b: Int) -> Int { a + b }")).len(),
        1
    );
    assert_eq!(chain(&value("f = (n) { n + 1 }")).len(), 1);
    assert_eq!(chain(&value("f = () { clock.read() }")).len(), 1);
    // G6, G27.
    assert!(matches!(value("f = (x) { y }").kind, ExprKind::Chain(_)));
    assert!(matches!(
        value("f = (x = 1) { 2 }").kind,
        ExprKind::Chain(_)
    ));
    assert!(matches!(value("f = (x = 1)").kind, ExprKind::Data(_)));
    // G32: the outer `(` is data, the inner item a function.
    match value("p = ((a) { a }, 1)").kind {
        ExprKind::Data(items) => {
            assert!(matches!(&items[0], DItem::Pos(e) if matches!(e.kind, ExprKind::Chain(_))))
        }
        other => panic!("{other:?}"),
    }
    // A `{` on the next line is not a body.
    let m = ok("f = () {\n\tx = (a)\n\t{ a }\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 2);
    bad("f = (a) -> Int\n", "expected the function body");
}

#[test]
fn p2_call() {
    // G7: `f(x) { … }` is a call and then a block statement.
    let m = ok("g = () {\n\tf(x)\n\t{ 1 }\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    let body = &chain(&b.value)[0].body;
    assert!(matches!(&body.stmts[0], Stmt::Expr(e) if matches!(e.kind, ExprKind::Call { .. })));
    assert!(matches!(&body.stmts[1], Stmt::Expr(e) if matches!(e.kind, ExprKind::Block(_))));
    // A `(` on a new line begins a new statement.
    let m = ok("g = () {\n\tf\n\t(a, b)\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 2);
}

#[test]
fn p4_branch_join() {
    let gcd =
        value("gcd =\n\t(a: Int, 0) -> Int { a }\n\t| (a: Int, b: Int) -> Int { gcd(b, a % b) }\n");
    assert_eq!(chain(&gcd).len(), 2);
    // G1: the body's `{` ends the `->` part; `|` after `}` is a join.
    assert_eq!(chain(&value("f = (p) -> A { e } | (q) { f }")).len(), 2);
    // G2: a forgotten body gives one function whose result is a union.
    let g2 = value("f = (a) -> Int | (b) -> Int { 1 }");
    let fs = chain(&g2);
    assert_eq!(fs.len(), 1);
    assert!(matches!(&fs[0].ret, Some(t) if matches!(t.kind, TypeKind::Union(_))));
    // G33: the labelled item's value is a chain.
    match value("x = (f = (a) { a } | (b) { b })").kind {
        ExprKind::Data(items) => {
            assert!(matches!(&items[0], DItem::Label(_, e) if chain(e).len() == 2))
        }
        other => panic!("{other:?}"),
    }
    // G35: the call applies to the whole chain.
    match value("x = (a) { 1 } | (b) { 2 }(y)").kind {
        ExprKind::Call { callee, .. } => assert_eq!(chain(&callee).len(), 2),
        other => panic!("{other:?}"),
    }
    // G9: an inner chain ends at the outer `}`.
    let nested = value("f = (a) { a |> (1) { 2 } | (_) { 3 } } | (b, c) { 4 }");
    assert_eq!(chain(&nested).len(), 2);
    // G22, D3.
    bad("h = f | g", "`|` joins only function literals");
    bad("h = (a) { 1 } | g", "`|` joins only function literals");
    // Overloading by shape (§2.6).
    let show = value(
        "show =\n\t(n: Int) -> String { int.to_string(n) }\n\t| (s: String) -> Int { string.length(s) }\n\t| (a: Int, b: Int) -> Bool { a == b }\n\t| (host: String, port = p: Int) -> Conn { open(host, p) }\n",
    );
    assert_eq!(chain(&show).len(), 4);
}

#[test]
fn p5_arrow_part() {
    // A `{` right after `->`, `|`, `&` or `-` is a type group; the first
    // other `{` is the body.
    let f = value("f = (x) -> { Int } | { String } { x }");
    assert!(matches!(&chain(&f)[0].ret, Some(t) if matches!(t.kind, TypeKind::Union(_))));
    let f = value("f = (x) -> { (Int) -> Int } { g }");
    assert!(matches!(&chain(&f)[0].ret, Some(t) if matches!(t.kind, TypeKind::Fn(_))));
    bad("f = (x) -> Int ^ E { x }", "expected the function body");
}

#[test]
fn p6_statements() {
    let m = ok(
        "f = () {\n\ta = b = c = d\n\tt = (a, b) = pair\n\t(x, y) = whole = e\n\tx: (n = Int) = (n = 1)\n\ta = (n = 1) = b\n\tUser(name = \"ada\")\n\t(w = 3, h = 4, area = w * h)\n\tOk(v) = parse(text)\n\tv\n}\n",
    );
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    let stmts = &chain(&b.value)[0].body.stmts;
    let parts = |s: &Stmt| match s {
        Stmt::Bind(b) => match &b.pattern.kind {
            PatternKind::And(ps) => ps.len(),
            _ => 1,
        },
        Stmt::Expr(_) => 0,
    };
    // G38, G12, G30, G18, G11.
    assert_eq!(
        stmts.iter().map(parts).collect::<Vec<_>>(),
        [3, 2, 2, 1, 2, 0, 0, 1, 0]
    );
    assert!(matches!(&stmts[5], Stmt::Expr(e) if matches!(e.kind, ExprKind::Call { .. })));
    assert!(matches!(&stmts[6], Stmt::Expr(e) if matches!(e.kind, ExprKind::Data(_))));
    bad("f = () { x = 1 }", "a block ends with an expression");
    bad("f = () {}", "`{}` and the empty match are removed");
    // The removed environment statements.
    bad("f = () {\n\t! Db = make()\n\t1\n}\n", "expected");
}

#[test]
fn p11_pipe() {
    // Lowest of the operators, left associative.
    match syn3::parse_expr("a == b |> f").unwrap().kind {
        ExprKind::Pipe { lhs, .. } => assert!(matches!(lhs.kind, ExprKind::Binary { .. })),
        other => panic!("{other:?}"),
    }
    match syn3::parse_expr("a |> f |> g").unwrap().kind {
        ExprKind::Pipe { lhs, .. } => assert!(matches!(lhs.kind, ExprKind::Pipe { .. })),
        other => panic!("{other:?}"),
    }
    for e in [
        "a |> f",
        "a |> m.f",
        "a |> f(b, port = 1)",
        "a |> f(..t)",
        "{ x |> g } == 2",
        "x == 1 |> core.not",
        "opt |> (Some(v) ? empty(v)) { \"empty\" } | (Some(v)) { v } | (None) { \"none\" }",
        "day |> (Sat | Sun) { \"rest\" } | (_) { \"work\" }",
        "p |> ((0, y)) { y } | ((x, _)) { x }",
    ] {
        assert_eq!(expr(e), e);
    }
    // G5.
    bad(
        "y = x |> g == 2",
        "the right side of `|>` must be a name, a call",
    );
    // G4: the whole chain is the right operand.
    match syn3::parse_expr("x |> (p) { e } | (q) { f }").unwrap().kind {
        ExprKind::Pipe { rhs, .. } => assert_eq!(chain(&rhs).len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn p13_lines() {
    // A line that begins with `|` continues; a line that begins with `(`
    // begins a new statement; inside `( )` a break is a `,`.
    let m = ok("f = () {\n\tr = x |>\n\t\t(1) { a }\n\t\t| (_) { b }\n\t(r, 1)\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 2);
    assert_eq!(
        syn3::parse_expr("f(\n\ta\n\tb = 1\n)").unwrap(),
        syn3::parse_expr("f(a, b = 1)").unwrap()
    );
    // G40.
    ok("sign = (-1) { \"neg\" }\n\t| (0) { \"zero\" }\n\t| (_) { \"pos\" }\n");
}

// ---- §3.4 Patterns ----

#[test]
fn patterns() {
    for p in [
        "x",
        "_",
        "_rest",
        "x: Int",
        "_: Int | String",
        "1",
        "-1",
        "-.1.5",
        "\"a\"",
        "\"a\" <> rest",
        "rest <> \"z\"",
        "\"a\" <> mid <> \"z\"",
        "None",
        "Some(v)",
        "m.Some(v)",
        "User(name = n, age = a)",
        "User(age = a, ..)",
        "C(x, ..)",
        "C(..)",
        "(x, y)",
        "()",
        "(name = n, age = a)",
        "(first, ..rest)",
        "(first, ..)",
        "(size = whole = (w, h))",
        "((a, b) = args)",
        "(_ = x = y)",
        "[]",
        "[x, ..xs]",
        "[x, ..]",
        "[..]",
        "<<a:size(8), rest:bytes>>",
        "a = b = (c, d)",
        "(port = p: Int)",
        "(label: T = x)",
    ] {
        assert_eq!(pat(p), p);
    }
    // P7: inside a data pattern a leading `name =` is a key.
    match syn3::parse_pattern("(size = whole = (w, h))").unwrap().kind {
        PatternKind::Data(items) => match &items[0] {
            DPItem::Key(k, p) => {
                assert_eq!(k.name, "size");
                assert!(matches!(&p.kind, PatternKind::And(ps) if ps.len() == 2));
            }
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    // At statement level it is a conjunction.
    assert!(matches!(
        syn3::parse_pattern("size = whole").unwrap().kind,
        PatternKind::And(_)
    ));
    bad("f = ((a, ..r, b)) { 1 }", "the rest `..` is the last item");
    bad("f = () {\n\t(a ? a > 1) = t\n\t1\n}\n", "expected");
}

#[test]
fn parameters_alternatives_guards() {
    let f = value("connect = (host: String, port = p: Int) -> Conn { open(host, p) }");
    let f = &chain(&f)[0];
    assert!(f.params[0].key.is_none());
    assert_eq!(f.params[1].key.as_ref().unwrap().name, "port");
    // G20: `|` is tighter than `,`.
    let f = value("f = (Sat | Sun, x) { 1 }");
    assert_eq!(chain(&f)[0].params[0].choices.len(), 2);
    assert_eq!(chain(&f)[0].params.len(), 2);
    // G21, P8: after a `:` type, `|` continues the union.
    let f = value("f = (x: Int | y) { 1 }");
    assert_eq!(chain(&f)[0].params[0].choices.len(), 1);
    let f = value("f = (True | _: Int) { 1 }");
    assert_eq!(chain(&f)[0].params[0].choices.len(), 2);
    // P9: the guard is after the last item and ends at `)`.
    let f = value("f = (a: Int, b: Int ? a > b) -> Int { a - b }");
    assert!(chain(&f)[0].guard.is_some());
    assert_eq!(chain(&f)[0].params.len(), 2);
    let f = value("f = (Some(v) ? string.is_empty(v)) { \"empty\" }");
    assert!(chain(&f)[0].guard.is_some());
    // G8: a function inside the guard is inside its own brackets.
    ok("f = (xs ? list.any(xs, (x) { x > 0 })) { 1 }\n");
    ok("f = (\n\ta: Int\n\tb: Int\n\t? a > b\n) { a }\n");
    // G36: a literal key pattern, not a default.
    let f = value("run = (mode = \"fast\") { quick() } | (mode = \"slow\") { careful() }");
    assert_eq!(chain(&f).len(), 2);
    ok("log_all = ((a, b) = args) { log(args) }\n");
    // G31.
    bad("f = (..rest) { 1 }", "no rest `..` at its top");
    bad("f = (a = 1, b) { 1 }", "by position may not come after");
    bad("f = (a ? a > 1, b) { 1 }", "after the guard");
    // Environment parameters are removed.
    bad("f = (a, log ! Log) { 1 }", "expected");
}

// ---- The services amendment ----

#[test]
fn proposal_examples() {
    let m = ok(r#"find_user = (id: String) <- (db: Db, log: Log) -> User {
	log.write(id)
	db.find(id)
}

Fail(E) = (raise = (E) -> Never)

raise = <E>(error: E) <- (fail: Fail(E)) -> Never { fail.raise(error) }

catch = <A, E, N>(body: () <- (..N, Fail(E)) -> A) -> Result(A, E) {
	control.boundary((b) {
		Ok(body() <- (Fail(raise = (e) { control.escape(b, Error(e)) })))
	})
}

map = <A, B, N>(xs: List(A), f: (A) <- N -> B) -> List(B) {
	xs |>
		([]) { [] }
		| ([x, ..rest]) { [f(x), ..map(rest, f)] }
}

base = () <- (io: Io) -> (Config, Log) {
	(Config(url = "…"), Log(write = (s) { io.print(s) }))
}

live = () -> (Config, Log, Db) {
	s = base()
	d = db_layer() <- s
	(..s, ..d)
}

main = () { app() <- live() }

test = () { app() <- (..live(), fake_db) }

keep = <S, A, N>(f: (S) <- N -> A) <- (..env: N) -> (S) -> A { (x) { f(x) <- env } }
"#);
    let function = |i: usize| match &m.items[i].kind {
        ItemKind::Bind(b) => chain(&b.value)[0].clone(),
        other => panic!("{other:?}"),
    };
    let find_user = function(0);
    let services = find_user.services.unwrap();
    assert_eq!(services.len(), 2);
    assert_eq!(services[0].name.name, "db");
    assert!(!services[0].rest);
    assert!(find_user.ret.is_some());
    let raise = function(2);
    assert_eq!(raise.generics.len(), 1);
    let catch = function(3);
    assert_eq!(catch.generics.len(), 3);
    // The needs of `catch` are inferred; its parameter's type writes them.
    assert!(catch.services.is_none());
    match &catch.params[0].choices[0].kind {
        PatternKind::Typed { ty, .. } => match &ty.kind {
            TypeKind::Fn(f) => assert!(matches!(
                f.services.as_ref().map(|s| &s.kind),
                Some(TypeKind::Data(items)) if matches!(items[0], TItem::Spread(_))
            )),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    // `..env: N` binds the whole environment.
    let keep = function(9);
    assert!(matches!(
        keep.services.as_deref(),
        Some([SItem { rest: true, .. }])
    ));
    // A bare row variable is written only in a type; a service has a name.
    bad("f = (x) <- N -> Int { x }", "bare row variable");
    bad("f = (x) <- (Db) { x }", "written `name: Type`");
    bad("f = (x) <- () { x }", "leave the `<-` part out");
    bad("f = (x) <- (..env: N, db: Db) { x }", "is the last item");
    bad("f: (Int) <- (db = Db) -> Int", "a need has no label");
    bad("f: (Int) <- (..N, ..M) -> Int", "at most one row variable");
    // `d = db_layer() <- s` provides to the call.
    let live = function(6);
    assert!(matches!(
        &live.body.stmts[1],
        Stmt::Bind(b) if matches!(b.value.kind, ExprKind::Provide { .. })
    ));
}

#[test]
fn provide() {
    for e in [
        "f(x) <- (s)",
        "app() <- live()",
        "f(x) <- env",
        "{ a } <- (s)",
        "x |> f <- (s)",
        "f() <- (a) <- (b)",
        "{ f() <- (s) } + 1",
        "g({ f() <- (s) }.x)",
        "(a, b) <- (s)",
        "f(x) <- (..live(), fake_db)",
    ] {
        assert_eq!(expr(e), e);
    }
    // The weakest level, below `|>`; the left operand is the whole
    // expression to its left; left associative.
    match syn3::parse_expr("x |> f <- (s)").unwrap().kind {
        ExprKind::Provide { target, .. } => assert!(matches!(target.kind, ExprKind::Pipe { .. })),
        other => panic!("{other:?}"),
    }
    match syn3::parse_expr("f() <- (a) <- (b)").unwrap().kind {
        ExprKind::Provide { target, .. } => {
            assert!(matches!(target.kind, ExprKind::Provide { .. }))
        }
        other => panic!("{other:?}"),
    }
    // The right operand is one postfix expression.
    match syn3::parse_expr("f() <- s.x(1)").unwrap().kind {
        ExprKind::Provide { services, .. } => {
            assert!(matches!(services.kind, ExprKind::Call { .. }))
        }
        other => panic!("{other:?}"),
    }
    // P1: `( … ) <- ( … )` is a function head only when the service group
    // is followed on the same line by `->` or `{`.
    assert!(matches!(
        syn3::parse_expr("(a) <- (s)").unwrap().kind,
        ExprKind::Provide { .. }
    ));
    assert!(matches!(
        syn3::parse_expr("(a) <- (s: S) { a }").unwrap().kind,
        ExprKind::Chain(_)
    ));
    assert!(matches!(
        syn3::parse_expr("(a) <- (s: S) -> Int { a }").unwrap().kind,
        ExprKind::Chain(_)
    ));
    // A line that begins or ends with `<-` continues.
    let m = ok("f = () {\n\tapp()\n\t\t<- live()\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 1);
    let m = ok("f = () {\n\tapp() <-\n\t\tlive()\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 1);
    bad("x = f() <- (db = d)", "provided services have no labels");
    // A block provided with services, with statements.
    ok("f = () {\n\t{\n\t\ta = 1\n\t\tg(a)\n\t} <- (Db(find = (id) { id }))\n}\n");
}

#[test]
fn generics() {
    // `<` after a line end begins a statement when `UpName … > (` follows.
    let m = ok("f = () {\n\tlog()\n\t<A>(x: A) { x }\n}\n");
    let ItemKind::Bind(b) = &m.items[0].kind else {
        unreachable!()
    };
    assert_eq!(chain(&b.value)[0].body.stmts.len(), 2);
    // Each branch of a chain writes its own `<…>`.
    let f = value("f = <A>(x: A) { 1 } | <B>(x: B, y: B) { 2 }");
    assert_eq!(chain(&f).len(), 2);
    assert_eq!(chain(&f)[1].generics[0].name, "B");
    // A comparison is still a comparison.
    assert_eq!(expr("a < b"), "a < b");
    assert_eq!(expr("a < B"), "a < B");
    // The type parameters of a binding head are removed.
    bad(
        "id(A) = (x: A) { x }",
        "type parameters are written on the function",
    );
    bad(
        "f = () {\n\tid(A): (A) -> A = g\n\tid\n}\n",
        "type parameters are written on the function",
    );
    ok("id = <A,>(x: A) { x }\n");
}

#[test]
fn lexer() {
    use ilang_syntax::lexer::{lex, lex3};
    use ilang_syntax::token::TokenKind as T;
    let kinds = |s: &str| {
        lex3(s)
            .tokens
            .into_iter()
            .map(|t| t.kind)
            .collect::<Vec<_>>()
    };
    assert!(kinds("a <- b").contains(&T::LArrow));
    assert!(kinds("x<-1").contains(&T::LArrow));
    assert!(!kinds("x < -1").contains(&T::LArrow));
    // `<<` is taken first, so bit arrays are unchanged.
    assert!(!kinds("<<-1>>").contains(&T::LArrow));
    // The current lexer is unchanged.
    assert!(lex("x<-1").tokens.iter().all(|t| t.kind != T::LArrow));
}

// ---- §6.5 Printer and formatter ----

#[test]
fn formatter_layout() {
    let long = "kind = day |> (Mon) { \"first day of the week\" } | (Tue) { \"second day of the week\" } | (_) { \"another day\" }\n";
    assert_eq!(
        syn3::format(long).unwrap(),
        "kind = day |>\n\t(Mon) { \"first day of the week\" }\n\t| (Tue) { \"second day of the week\" }\n\t| (_) { \"another day\" }\n"
    );
    let gcd = "gcd = (a: Int, 0) -> Int { a } | (a: Int, b: Int) -> Int { gcd(b, a % b) } | (a: Int, b: Int, c: Int) -> Int { gcd(gcd(a, b), c) }\n";
    assert_eq!(
        syn3::format(gcd).unwrap(),
        "gcd =\n\t(a: Int, 0) -> Int { a }\n\t| (a: Int, b: Int) -> Int { gcd(b, a % b) }\n\t| (a: Int, b: Int, c: Int) -> Int { gcd(gcd(a, b), c) }\n"
    );
    assert_eq!(
        syn3::format("kind = day |> (Sat | Sun) { \"rest\" } | (_) { \"work\" }\n").unwrap(),
        "kind = day |> (Sat | Sun) { \"rest\" } | (_) { \"work\" }\n"
    );
    // Comments stay; literals are printed as they are written.
    let source = "// before\nf = (x) {\n\t// first\n\ta = 0xFF // trailing\n\n\t// second\n\tg(a, x)\n\t// last\n}\n";
    assert_eq!(syn3::format(source).unwrap(), source);
    let source = "x = list.map(xs, (x) {\n\ty = x + 1\n\ty * 2\n})\n";
    assert_eq!(syn3::format(source).unwrap(), source);
}

// ---- §6.8 Lisp surface ----

#[test]
fn lisp_rows() {
    for (normal, lisp) in [
        ("U = (T1, x = T2)", "(= U (, T1 (= x T2)))"),
        ("U(A) = (A)", "(= (U A) (, A))"),
        ("U = ()", "(= U (,))"),
        ("U = Opaque", "(= U Opaque)"),
        ("U(A) = Opaque", "(= (U A) Opaque)"),
        ("U = T", "(= U T)"),
        ("U(A) = List(A)", "(= (U A) (List A))"),
        ("U = { (T1, T2) }", "(= U {(, T1 T2)})"),
        ("x: (T1, x = T2) -> R", "(: x (-> (, T1 (= x T2)) R))"),
        ("x: () -> R", "(: x (-> (,) R))"),
        ("x: { (T) | (T, T) } -> R", "(: x (-> (| (, T) (, T T)) R))"),
        ("y = f(e1, x = e2)", "(= y (f e1 (= x e2)))"),
        ("y = f()", "(= y (f))"),
        ("y = f(..e)", "(= y (f .. e))"),
        ("y = (e1, x = e2)", "(= y (, e1 (= x e2)))"),
        ("y = (e)", "(= y (, e))"),
        ("y = ()", "(= y (,))"),
        ("y = (..e, x = v)", "(= y (, .. e (= x v)))"),
        ("y = (p1, x = p2) { e }", "(= y (-> (, p1 (= x p2)) _ e))"),
        ("y = (p) -> R { e }", "(= y (-> (, p) R e))"),
        ("y = () { e }", "(= y (-> (,) _ e))"),
        ("y = (p1, p2 ? g) { e }", "(= y (-> (? (, p1 p2) g) _ e))"),
        (
            "y = (a) { 1 } | (b) { 2 }",
            "(= y (| (-> (, a) _ 1) (-> (, b) _ 2)))",
        ),
        ("y = e |> f", "(= y (|> e f))"),
        (
            "y = e |> (a) { 1 } | (b) { 2 }",
            "(= y (|> e (| (-> (, a) _ 1) (-> (, b) _ 2))))",
        ),
        ("(p1, x = p2) = v", "(= (, p1 (= x p2)) v)"),
        ("(p, ..) = v", "(= (, p ..) v)"),
        ("(p, ..r) = v", "(= (, p .. r) v)"),
        (
            "y = (C(p1, x = p2)) { 1 }",
            "(= y (-> (, (C p1 (= x p2))) _ 1))",
        ),
        ("y = (C(p, ..), D) { 1 }", "(= y (-> (, (C p ..) D) _ 1))"),
        (
            "(size = whole = (w, h)) = box",
            "(= (, (= size whole (, w h))) box)",
        ),
        (
            "y = ((a, b) = args) { 1 }",
            "(= y (-> (, (= (, a b) args)) _ 1))",
        ),
        ("t = (a, b) = e", "(= (= t (, a b)) e)"),
        (
            "y = (Sat | Sun, x) { 1 }",
            "(= y (-> (, (| Sat Sun) x) _ 1))",
        ),
        // The services forms.
        (
            "x: (T) <- (Db, Log) -> R",
            "(: x (-> (, T) (<- R (, Db Log))))",
        ),
        ("x: (T) <- N -> R", "(: x (-> (, T) (<- R N)))"),
        (
            "x: (T) <- (..N, Db) -> R",
            "(: x (-> (, T) (<- R (, .. N Db))))",
        ),
        ("x: <A>(A) -> A", "(: x (-> [A] (, A) A))"),
        (
            "y = (id: String) <- (db: Db) -> User { e }",
            "(= y (-> (, (: id String)) (<- User (, (: db Db))) e))",
        ),
        (
            "y = (id) <- (db: Db) { e }",
            "(= y (-> (, id) (<- _ (, (: db Db))) e))",
        ),
        (
            "y = <A, N>(x: A) <- (..env: N) { e }",
            "(= y (-> [A N] (, (: x A)) (<- _ (, .. (: env N))) e))",
        ),
        ("y = f(x) <- (s)", "(= y (<- (f x) (, s)))"),
        ("y = app() <- live()", "(= y (<- (app) (live)))"),
    ] {
        assert_eq!(lisp_of(normal), lisp, "{normal}");
        let (read, diags) = lisp::from_lisp(lisp);
        assert!(diags.is_empty(), "{lisp}\n{diags:#?}");
        assert_eq!(read, ok(normal), "{lisp}");
    }
}

#[test]
fn lisp_examples_of_the_note() {
    let normal = "connect = (host: String, port = p: Int) -> Conn { open(host, p) }\npick = (0, y) { y } | (x, _) { x }\nlabel = (opt) { opt |> (Some(v) ? string.is_empty(v)) { \"empty\" } | (Some(v)) { v } | (None) { \"none\" } }\n";
    let lisp_text = "(= connect (-> (, (: host String) (= port (: p Int))) Conn (open host p)))\n(= pick (| (-> (, 0 y) _ y) (-> (, x _) _ x)))\n(= label (-> (, opt) _\n  (|> opt (| (-> (? (, (Some v)) ((. string is_empty) v)) _ \"empty\")\n             (-> (, (Some v)) _ v)\n             (-> (, None) _ \"none\")))))\n";
    let (read, diags) = lisp::from_lisp(lisp_text);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(read, ok(normal));
    // The normal printer writes a body that is not a block in `{ }`.
    assert_eq!(
        syn3::print_module(&read),
        "connect = (host: String, port = p: Int) -> Conn { open(host, p) }\n\npick = (0, y) { y } | (x, _) { x }\n\nlabel = (opt) {\n\topt |>\n\t\t(Some(v) ? string.is_empty(v)) { \"empty\" }\n\t\t| (Some(v)) { v }\n\t\t| (None) { \"none\" }\n}\n"
    );
    let test_is = "(= text (-> (, x) _\n  (|> ((. syntax node) x)\n      (| (-> (, ((. syntax Node) ((. syntax Text) s))) _ s)\n         (-> (, _) _ (panic \"test name must be a string literal\"))))))\n(= append (-> (, xs ys) _\n  (|> xs (| (-> (, []) _ ys) (-> (, [x .. rest]) _ [x .. (append rest ys)])))))\n";
    let (read, diags) = lisp::from_lisp(test_is);
    assert!(diags.is_empty(), "{diags:#?}");
    let (again, diags) = lisp::from_lisp(&lisp::to_lisp(&read));
    assert!(diags.is_empty());
    assert_eq!(again, read);
    let (normal, diags) = syn3::parse_module(&syn3::print_module(&read));
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(normal, read);
}

#[test]
fn lisp_errors() {
    for (text, needle) in [
        ("(= y ())", "the empty data is `(,)`"),
        ("(= y (-> (a b) _ 1))", "data pattern `(, …)`"),
        ("(= y (| a f))", "expected a function"),
        ("(= y (-> (, .. r) _ 1))", "no rest"),
    ] {
        let (_, diags) = lisp::from_lisp(text);
        assert!(
            diags.iter().any(|d| d.message.contains(needle)),
            "{text}\n{diags:#?}"
        );
    }
}

// ---- §7.3 The converter ----

fn converted(old: &str) -> String {
    let (module, diags) = ilang_syntax::parse_module(old);
    assert!(diags.is_empty(), "{old}\n{diags:#?}");
    let lexed = ilang_syntax::lexer::lex(old);
    let (new, _) = convert::convert(&module, Some(&lexed), &convert::Signatures::default());
    let text = syn3::print_with_comments(&new, old, &lexed.comments);
    let (read, diags) = syn3::parse_module(&text);
    assert!(diags.is_empty(), "{text}\n{diags:#?}");
    assert_eq!(read, new, "{text}");
    let plain = syn3::print_module(&new);
    let (read, diags) = syn3::parse_module(&plain);
    assert!(diags.is_empty(), "{plain}\n{diags:#?}");
    assert_eq!(read, new, "{plain}");
    text
}

#[test]
fn convert_types() {
    assert_eq!(
        converted("Tree(A) {\n\tLeaf\n\tNode(left: Tree(A), value: A, right: Tree(A))\n}\n"),
        "Leaf = ()\nNode(A) = (left = Tree(A), value = A, right = Tree(A))\nTree(A) = Leaf | Node(A)\n"
    );
    assert_eq!(
        converted("User { User(name: String, age: Int) }\n"),
        "User = (name = String, age = Int)\n"
    );
    assert_eq!(converted("Element {}\n"), "Element = Opaque\n");
    assert_eq!(
        converted("Point = (x: Int, y: Int)\n"),
        "Point = { (x = Int, y = Int) }\n"
    );
    assert_eq!(
        converted("Pair(A, B) = (A, B)\n"),
        "Pair(A, B) = { (A, B) }\n"
    );
    assert_eq!(converted("Meters = Int\n"), "Meters = Int\n");
    // `T` alone is exported with its members.
    assert_eq!(
        converted("(\n\tOption\n)\n\nOption(A) {\n\tSome(A)\n\tNone\n}\n"),
        "(\n\tOption\n\tSome\n\tNone\n)\n\nSome(A) = (A)\nNone = ()\nOption(A) = Some(A) | None\n"
    );
}

#[test]
fn convert_clauses_and_matches() {
    assert_eq!(
        converted(
            "describe = (opt: Option(String)) -> String {\n\topt |> {\n\t\tSome(v) ? string.is_empty(v) -> \"empty\"\n\t\tSome(v) -> v\n\t\tNone -> \"none\"\n\t}\n}\n"
        ),
        "describe = (opt: Option(String)) -> String {\n\topt |>\n\t\t(Some(v) ? string.is_empty(v)) { \"empty\" }\n\t\t| (Some(v)) { v }\n\t\t| (None) { \"none\" }\n}\n"
    );
    // A tuple pattern becomes the one item `((a, b))`.
    assert_eq!(
        converted(
            "pick = (p: (Int, Int)) -> Int {\n\tp |> {\n\t\t(0, y) -> y\n\t\t(x, _) -> x\n\t}\n}\n"
        ),
        "pick = (p: (Int, Int)) -> Int { p |> ((0, y)) { y } | ((x, _)) { x } }\n"
    );
    assert_eq!(
        converted("x = list.map(opts, { Some(u) -> u.name  None -> \"\" })\n"),
        "x = list.map(opts, (Some(u)) { u.name } | (None) { \"\" })\n"
    );
    assert_eq!(
        converted("k = d |> { Sat | Sun -> 1  _ -> 2 }\n"),
        "k = d |> (Sat | Sun) { 1 } | (_) { 2 }\n"
    );
    assert_eq!(
        converted("absurd = (x: Never) { x |> {} }\n"),
        "absurd = (x: Never) { core.never(x) }\n"
    );
    // The pipe is the weakest operator in both syntaxes, so the grouping is kept as written.
    assert_eq!(converted("ok = a <> b |> f\n"), "ok = a <> b |> f\n");
    assert_eq!(
        converted("ok = a <> { b |> f }\n"),
        "ok = a <> { b |> f }\n"
    );
    assert_eq!(
        converted("ok = { x |> f } == y\n"),
        "ok = { x |> f } == y\n"
    );
}

#[test]
fn convert_patterns_and_data() {
    assert_eq!(
        converted("swap = (t = (a, b)) { (b, a) }\n"),
        "swap = ((a, b) = t) { (b, a) }\n"
    );
    assert_eq!(
        converted("f = (x = label) { x }\n"),
        "f = (_ = x = label) { x }\n"
    );
    assert_eq!(
        converted("f = (x = label: T) { x }\n"),
        "f = (label: T = x) { x }\n"
    );
    assert_eq!(
        converted("f = () {\n\tt = (a, b) = pair\n\tOk(v) = parse(text)\n\tv\n}\n"),
        "f = () {\n\tt = (a, b) = pair\n\tOk(v) = parse(text)\n\tv\n}\n"
    );
    assert_eq!(
        converted("ada = (name: \"ada\", age: 36)\n"),
        "ada = (name = \"ada\", age = 36)\n"
    );
    assert_eq!(
        converted("older = (..ada, age: 37)\n"),
        "older = (..ada, age = 37)\n"
    );
    // Constructors of the file: fields by label, no punning.
    assert_eq!(
        converted(
            "User { User(name: String, age: Int) }\nu = User(\"ada\", 36)\nv = User(..u, age: 37)\nadult = (u: User) { u |> { User(name, age) -> age > 17 } }\nyoung = (u: User) { u |> { User(n = name, ..) -> n } }\n"
        ),
        "User = (name = String, age = Int)\nu = User(name = \"ada\", age = 36)\nv = User(..u, age = 37)\nadult = (u: User) { u |> (User(name = name, age = age)) { age > 17 } }\nyoung = (u: User) { u |> (User(name = n, ..)) { n } }\n"
    );
    assert_eq!(
        converted("s = \"a \\(x + 1) b\"\n"),
        "s = \"a \\(x + 1) b\"\n"
    );
}

#[test]
fn convert_services() {
    assert_eq!(
        converted(
            "find_user = (id: String, db ! Db) -> User ^ NotFound ! Db {\n\tdb.find(id)\n}\n"
        ),
        "find_user = (id: String) <- (db: Db) -> User { db.find(id) }\n"
    );
    assert_eq!(
        converted(
            "fake = () -> User ^ NotFound {\n\t! Db = Db(find: (id) { fail.raise(NotFound(id)) })\n\tfind_user(\"1\")\n}\n"
        ),
        "fake = () -> User {\n\tfind_user(\"1\") <- (Db(find = (id) { fail.raise(NotFound(id)) }))\n}\n"
    );
    assert_eq!(
        converted(
            "count(A, E, N) = (list: List(A), where: (A) -> Bool ^ E ! N) -> Int ^ E ! N { 0 }\n"
        ),
        "count = <A, N>(list: List(A), where: (A) <- N -> Bool) -> Int { 0 }\n"
    );
    assert_eq!(
        converted("@external(\"i:fail\", \"raise\")\nraise(E): (error: E) -> Never ^ E\n"),
        "@external(\"i:fail\", \"raise\")\nraise: <E>(E) <- (Fail(E)) -> Never\n"
    );
    assert_eq!(
        converted("f = () {\n\t! A = a\n\tx = 1\n\tg(x)\n}\n"),
        "f = () {\n\t{\n\t\tx = 1\n\t\tg(x)\n\t} <- (a)\n}\n"
    );
    assert_eq!(
        converted("h(N) = (f: () -> Int ! N) -> Int ! N { f() }\n"),
        "h = <N>(f: () <- N -> Int) -> Int { f() }\n"
    );
    // Direct values merge into one provide; other values nest.
    assert_eq!(
        converted("f = () {\n\t! A = a\n\t! B = b\n\tg()\n}\n"),
        "f = () { g() <- (a, b) }\n"
    );
    assert_eq!(
        converted("f = () {\n\t! A = make()\n\t! B = b\n\tg()\n}\n"),
        "f = () { g() <- (b) <- (make()) }\n"
    );
    assert_eq!(
        converted("T = (key: String) -> Int ^ E ! Db | Log\n"),
        "T = (String) <- (Fail(E), Db, Log) -> Int\n"
    );
    assert_eq!(
        converted("m(A, B, E) = (f: (A) -> B ^ E) -> B ^ E { f(a) }\n"),
        "m = <A, B, E>(f: (A) <- (Fail(E)) -> B) -> B { f(a) }\n"
    );
}

#[test]
fn convert_labelled_calls() {
    // The parameters of a function of the file are known: positional, in
    // parameter order.
    assert_eq!(
        converted(
            "connect = (host: String, port: Int) { open(host, port) }\nc = connect(\"a.com\", port: 80)\nd = connect(port: 80, host: \"a.com\")\n"
        ),
        "connect = (host: String, port: Int) { open(host, port) }\nc = connect(\"a.com\", 80)\nd = connect(\"a.com\", 80)\n"
    );
    // The order changes and two arguments are not direct values: locals keep
    // the order of evaluation.
    assert_eq!(
        converted("f = (a: Int, b: Int) { a - b }\nx = f(b: g(), a: h())\n"),
        "f = (a: Int, b: Int) { a - b }\nx = {\n\t_syn3_0 = g()\n\t_syn3_1 = h()\n\tf(_syn3_1, _syn3_0)\n}\n"
    );
    // The piped value is the first argument.
    assert_eq!(
        converted("f = (a: Int, b: Int, c: Int) { a }\nx = 1 |> f(c: 3, b: 2)\n"),
        "f = (a: Int, b: Int, c: Int) { a }\nx = 1 |> f(2, 3)\n"
    );
    // A function that the text does not show keeps its labels.
    assert_eq!(converted("x = m.f(1, to: 2)\n"), "x = m.f(1, to = 2)\n");
    // The functions and constructors of an imported module.
    let (lib, _) = ilang_syntax::parse_module(
        "User { User(name: String, age: Int) }\nrange = (from: Int, to: Int) { [] }\n",
    );
    let mut signatures = convert::Signatures::default();
    convert::collect(&lib, "users", &mut signatures);
    let old = "(\n\tu = app/users\n\t(User) = app/users\n)\n\nx = u.range(to: 9, from: 1)\ny = User(\"ada\", 36)\nz = u.User(\"ada\", 36)\n";
    let (module, _) = ilang_syntax::parse_module(old);
    let (new, notes) = convert::convert(&module, Some(&ilang_syntax::lexer::lex(old)), &signatures);
    assert!(notes.is_empty(), "{notes:#?}");
    assert_eq!(
        syn3::print_module(&new),
        "(\n\tu = app/users\n\t(User) = app/users\n)\n\nx = u.range(1, 9)\n\ny = User(name = \"ada\", age = 36)\n\nz = u.User(name = \"ada\", age = 36)\n"
    );
}

#[test]
fn convert_lisp_surface() {
    let old = "(= append (-> (xs ys) _ {(| xs {(-> [] ys) (-> [x .. rest] [x .. (append rest ys)])})}))\n";
    let (module, diags) = ilang_syntax::lisp::parse_source(old);
    assert!(diags.is_empty(), "{diags:#?}");
    let (new, _) = convert::convert(&module, None, &convert::Signatures::default());
    let text = lisp::to_lisp(&new);
    assert_eq!(
        text,
        "(= append (-> (, xs ys) _ (|> xs (| (-> (, []) _ ys) (-> (, [x .. rest]) _ [x .. (append rest ys)])))))\n"
    );
    let (read, diags) = lisp::from_lisp(&text);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(read, new);
}

#[test]
fn conversion_keeps_ordinary_fail_services_out_of_the_legacy_error_channel() {
    // §4.3 aliases keep nominal identity. The transition syntax reserves
    // bare Fail(E) for ^ E; an ordinary ! Fail(E) remains a service.
    let text = converted("Use(E) = () -> Nil ! Fail(E)\n");
    let (lowered, diags) = syn3::parse_lowered(&text);
    assert!(diags.is_empty(), "{text}\n{diags:#?}");
    let shown = ilang_syntax::print::print_module(&lowered);
    assert!(shown.contains("! Syn3ServiceFail(E)"), "{shown}");
    assert!(!shown.contains("^ E"), "{shown}");
    let collision = converted("T { Syn3ServiceFail }\nUse(E) = () -> Nil ! Fail(E)\n");
    assert!(collision.contains("Syn3ServiceFailX(E)"), "{collision}");
}
