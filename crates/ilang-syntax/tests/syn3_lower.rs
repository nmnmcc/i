//! The transition front of syntax three (docs/design/syntax-3.md §7.2,
//! stage 8): a syntax-three text lowers to the tree that today's text gives.

use ilang_syntax::print::print_module;
use ilang_syntax::syn3;

/// `new` (syntax three) lowers to the tree of `old` (today's syntax), as
/// today's printer shows both.
fn same(new: &str, old: &str) {
    let (module, diags) = syn3::parse_lowered(new);
    assert!(diags.is_empty(), "{new}\n{diags:#?}");
    let (expected, diags) = ilang_syntax::legacy::parser::parse_module_legacy(old);
    assert!(diags.is_empty(), "{old}\n{diags:#?}");
    assert_eq!(print_module(&module), print_module(&expected), "{new}");
}

/// The lowered tree of `new`, as today's printer shows it.
fn shown(new: &str) -> String {
    let (module, diags) = syn3::parse_lowered(new);
    assert!(diags.is_empty(), "{new}\n{diags:#?}");
    print_module(&module)
}

fn bad(new: &str, needle: &str) {
    let (_, diags) = syn3::parse_lowered(new);
    assert!(
        diags.iter().any(|d| d.message.contains(needle)),
        "{new}\nexpected an error with `{needle}`, got {diags:#?}"
    );
}

#[test]
fn type_definitions() {
    same(
        "(\n\tOption(..)\n\tShape\n\tCircle\n\tSquare\n)\n\nSome(A) = (A)\nNone = ()\nOption(A) = Some(A) | None\n\nCircle = (radius = Float)\nSquare = (Float)\nShape = Circle | Square\n\nUser = (name = String, age = Int)\n\nHandle = Opaque\n\nPair(A) = { (A, A) }\n\nName = String\n",
        "(\n\tOption(..)\n\tShape\n)\n\nOption(A) {\n\tSome(A)\n\tNone\n}\n\nShape {\n\tCircle(radius: Float)\n\tSquare(Float)\n}\n\nUser {\n\tUser(name: String, age: Int)\n}\n\nHandle {}\n\nPair(A) = (A, A)\n\nName = String\n",
    );
}

#[test]
fn constructor_named_like_its_sum() {
    same(
        "Inherit = ()\nDirectory = (String)\nDirectory = Inherit | Directory\n",
        "Directory {\n\tInherit\n\tDirectory(String)\n}\n",
    );
}

#[test]
fn matches_and_chains() {
    same(
        "f = (n: Int) -> String {\n\tn |> (0) { \"zero\" } | (1 | 2) { \"few\" } | (x ? x > 9) { \"many\" } | (_) { \"some\" }\n}\n",
        "f = (n: Int) -> String ^ _ ! _ {\n\tn |> {\n\t\t0 -> \"zero\"\n\t\t1 | 2 -> \"few\"\n\t\tx ? x > 9 -> \"many\"\n\t\t_ -> \"some\"\n\t}\n}\n",
    );
    same(
        "name = (Some(u)) { u } | (None) { \"\" }\n",
        "name = {\n\tSome(u) -> u\n\tNone -> \"\"\n}\n",
    );
}

#[test]
fn calls_data_and_patterns() {
    same(
        "u = User(name = \"ada\", age = 36)\nv = User(..u, age = 37)\nt = (1, 2)\nr = (a = 1, b = 2)\ne = ()\nx = t.0\ng = (User(name = n, ..)) { n }\nh = (User(name = name, age = age)) { name }\n",
        "u = User(name: \"ada\", age: 36)\nv = User(..u, age: 37)\nt = (1, 2)\nr = (a: 1, b: 2)\ne = ()\nx = t.0\ng = (User(n = name, ..)) { n }\nh = (User(name, age)) { name }\n",
    );
}

#[test]
fn services_and_needs() {
    same(
        "find_user = (id: String) <- (db: Db, log: Log) {\n\tlog.write(id)\n\tdb.find(id)\n}\n\nDb = (find = (String) <- (Fail(NotFound), Log) -> User)\n",
        "find_user = (id: String, db ! Db, log ! Log) {\n\tlog.write(id)\n\tdb.find(id)\n}\n\nDb {\n\tDb(find: (String) -> User ^ NotFound ! Log)\n}\n",
    );
}

#[test]
fn provides() {
    // `! _ = e` is not written in today's syntax: only the front makes it.
    let text = shown(
        "check = () {\n\t{\n\t\tu = find_user(\"1\")\n\t\tassert(u.name == \"ada\")\n\t} <- (fake_db, quiet_log)\n}\n\nn = () { 1 + f(2) <- (s) }\n",
    );
    assert_eq!(
        text,
        "check = () {\n  ! _ = fake_db\n  ! _ = quiet_log\n  u = find_user(\"1\")\n  assert(u.name == \"ada\")\n}\n\nn = () {\n  ! _ = s\n  1 + f(2)\n}\n"
    );
}

#[test]
fn type_parameters_and_row_variables() {
    let text = shown(
        "map = <A, B, N>(xs: List(A), f: (A) <- N -> B) -> List(B) { [] }\n\ncatch: <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E)\n",
    );
    assert!(text.contains("map(A, B, N, N%E) ="), "{text}");
    assert!(text.contains("f: (A) -> B ^ N%E ! N"), "{text}");
    assert!(
        text.contains("catch(A, E, N): (() -> A ^ E ! N) -> Result(A, E) ! N"),
        "{text}"
    );
}

#[test]
fn several_positions() {
    let text = shown("gcd = (a, 0) { a } | (a, b) { gcd(b, a % b) }\n");
    assert!(text.contains("(%arg1, %arg2)"), "{text}");
}

#[test]
fn forms_of_later_stages_are_errors() {
    bad(
        "area = (r: Float) { r } | (w: Float, h: Float) { w *. h }\n",
        "stage 9",
    );
    bad("keep = (f: Int) <- (..env: N) { f }\n", "stage E6");
    bad("f = (x) { g(x) <- env }\n", "stage E6");
}

/// The Lisp surface: `std/src/test.is`, converted and read back, is the
/// tree that the file gives today (quotes included).
#[test]
fn lisp_surface_round_trip() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../std/src/test.is");
    let source = std::fs::read_to_string(path).unwrap();
    // The repository file is in today's rows until the conversion commit,
    // and in the rows of syntax three after it.
    let Ok(document) = ilang_syntax::lisp::parse(&source) else {
        return;
    };
    let Ok(old) = ilang_syntax::lisp::parse_module(&document) else {
        return;
    };
    let (converted, _) = syn3::convert::convert(&old, None, &syn3::convert::Signatures::default());
    let text = syn3::lisp::to_lisp(&converted);
    let (lowered, diags) = syn3::parse_lisp_lowered(&text);
    assert!(diags.is_empty(), "{diags:#?}");
    let shown = |m: &ilang_syntax::ast::Module| ilang_syntax::lisp::print_module(m);
    let (a, b) = (shown(&old), shown(&lowered));
    for (x, y) in a.lines().zip(b.lines()) {
        assert_eq!(x, y);
    }
    assert_eq!(a.lines().count(), b.lines().count());
}
