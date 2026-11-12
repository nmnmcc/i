//! The transition front of syntax three (docs/design/syntax-3.md §7.2,
//! stage 8): a syntax-three text lowers to the tree that today's text gives.

use ilang_syntax::print::print_module;
use ilang_syntax::syn3;

/// `new` (syntax three) lowers to the tree of `old` (today's syntax), as
/// today's printer shows both.
fn same(new: &str, old: &str) {
    let (module, diags) = syn3::parse_lowered(new);
    assert!(diags.is_empty(), "{new}\n{diags:#?}");
    let (expected, diags) = ilang_syntax::parse_module(old);
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
    same(
        "check = () {\n\t{\n\t\tu = find_user(\"1\")\n\t\tassert(u.name == \"ada\")\n\t} <- (fake_db, quiet_log)\n}\n\nn = () { 1 + f(2) <- (s) }\n",
        "check = () {\n\t! _ = fake_db\n\t! _ = quiet_log\n\tu = find_user(\"1\")\n\tassert(u.name == \"ada\")\n}\n\nn = () {\n\t! _ = s\n\t1 + f(2)\n}\n",
    );
}

#[test]
fn type_parameters_and_row_variables() {
    let text = shown(
        "map = <A, B, N>(xs: List(A), f: (A) <- N -> B) -> List(B) { [] }\n\ncatch: <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E)\n",
    );
    assert!(text.contains("map(A, B, N, N%E) ="), "{text}");
    assert!(text.contains("f: (A) -> B ^ N%E ! N"), "{text}");
    assert!(text.contains("catch(A, E, N): (() -> A ^ E ! N) -> Result(A, E) ! N"), "{text}");
}

#[test]
fn several_positions() {
    let text = shown("gcd = (a, 0) { a } | (a, b) { gcd(b, a % b) }\n");
    assert!(text.contains("(%arg1, %arg2)"), "{text}");
}

#[test]
fn forms_of_later_stages_are_errors() {
    bad("area = (r: Float) { r } | (w: Float, h: Float) { w *. h }\n", "stage 9");
    bad("keep = (f: Int) <- (..env: N) { f }\n", "stage E6");
    bad("f = (x) { g(x) <- env }\n", "stage E6");
}
