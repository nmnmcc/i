//! The queries for editors (`ilang_check::query`) on small packages.

use ilang_check::Checked;
use ilang_check::hir::ModuleId;
use ilang_check::query::{CompletionKind, Query, SymbolKind, check_for_queries};
use ilang_package::PackageGraph;
use std::fs;

struct Fixture {
    _dir: tempfile::TempDir,
    graph: PackageGraph,
    checked: Checked,
}

impl Fixture {
    /// A package named `app` with these files.
    fn new(files: &[(&str, &str)]) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
        for (path, text) in files {
            let p = dir.path().join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
        let (graph, diags) = ilang_package::load(dir.path());
        assert!(diags.is_empty(), "{diags:?}");
        let checked = check_for_queries(&graph);
        Fixture {
            _dir: dir,
            graph,
            checked,
        }
    }

    fn query(&self) -> Query<'_> {
        Query::new(
            &self.graph,
            self.checked.program.as_ref().unwrap(),
            self.checked.types.as_ref(),
        )
    }

    fn module(&self, path: &str) -> ModuleId {
        let p = self.checked.program.as_ref().unwrap();
        ModuleId(p.modules.iter().position(|m| m.path == path).unwrap() as u32)
    }

    fn text(&self, path: &str) -> &str {
        let p = self.checked.program.as_ref().unwrap();
        &self
            .graph
            .sources
            .get(p.module(self.module(path)).source)
            .text
    }

    /// The offset of the `nth` occurrence of `needle` in the module,
    /// plus `delta`.
    fn at(&self, path: &str, needle: &str, nth: usize, delta: u32) -> u32 {
        let text = self.text(path);
        let (i, _) = text
            .match_indices(needle)
            .nth(nth)
            .unwrap_or_else(|| panic!("no {needle:?} #{nth}"));
        i as u32 + delta
    }

    fn hover(&self, path: &str, needle: &str, nth: usize) -> String {
        let at = self.at(path, needle, nth, 0);
        let h = self
            .query()
            .type_at(self.module(path), at)
            .unwrap_or_else(|| panic!("no hover at {needle:?}"));
        let text = self.text(path);
        assert!(
            text[h.span.start as usize..h.span.end as usize].starts_with(
                needle
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap()
            ),
            "span {:?}",
            h.span
        );
        h.code
    }

    /// The text at the definition of the name at the occurrence, with its
    /// module path.
    fn definition(&self, path: &str, needle: &str, nth: usize) -> (String, String) {
        let at = self.at(path, needle, nth, 0);
        let loc = self
            .query()
            .definition_at(self.module(path), at)
            .unwrap_or_else(|| panic!("no definition at {needle:?}"));
        let p = self.checked.program.as_ref().unwrap();
        let m = p.module(loc.module);
        let text = &self.graph.sources.get(loc.source).text;
        (
            m.path.clone(),
            text[loc.span.start as usize..loc.span.end as usize].to_string(),
        )
    }

    fn completions(&self, path: &str, needle: &str, delta: u32) -> Vec<(String, CompletionKind)> {
        let at = self.at(path, needle, 0, delta);
        self.query()
            .completions_at(self.module(path), at)
            .into_iter()
            .map(|c| (c.label, c.kind))
            .collect()
    }
}

const MAIN: &str = r#"(
	util = app/util
	(User, greet) = app/user
)

/// Adds two numbers.
add = (x, y) {
	z = x * 2
	z + y
}

id = (a) { a }

describe = (u: User) {
	label = u.name
	label <> greet(u)
}

first = (o) { o |> (Some(v)) { v } | (None) { 0 } }

Some(A) = (A)
None = ()
Option(A) = Some(A) | None

total = util.double(add(1, 2))
"#;

const UTIL: &str = r#" (double)

/// Twice the number.
double = (n: Int) -> Int { n * 2 }
"#;

const USER: &str = r#"(
	User(..)
	greet
)

User = (name = String, age = Int)

greet = (u: User) { "hi " <> u.name }
"#;

fn fixture() -> Fixture {
    let f = Fixture::new(&[
        ("src/main.i", MAIN),
        ("src/util.i", UTIL),
        ("src/user.i", USER),
    ]);
    let errors: Vec<_> = f.checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    f
}

#[test]
fn type_of_top_level_names() {
    let f = fixture();
    assert_eq!(
        f.hover("app/main", "add = ", 0),
        "add: (x: Int, y: Int) -> Int"
    );
    assert_eq!(
        f.hover("app/main", "add(1", 0),
        "add: (x: Int, y: Int) -> Int"
    );
    assert_eq!(f.hover("app/main", "id = ", 0), "id: (a: A) -> A");
    assert_eq!(f.hover("app/main", "total", 0), "total: Int");
    // A name of another module, after `m.`.
    assert_eq!(f.hover("app/main", "double", 0), "double: (n: Int) -> Int");
    // Imported by name.
    assert_eq!(
        f.hover("app/main", "greet(u)", 0),
        "greet: (u: User) -> String"
    );
}

#[test]
fn type_of_local_names() {
    let f = fixture();
    // A parameter, where it is bound and where it is used.
    assert_eq!(f.hover("app/main", "x, y", 0), "x: Int");
    assert_eq!(f.hover("app/main", "x * 2", 0), "x: Int");
    // A binding in a block.
    assert_eq!(f.hover("app/main", "z = ", 0), "z: Int");
    assert_eq!(f.hover("app/main", "z + y", 0), "z: Int");
    // A parameter of any type.
    assert_eq!(f.hover("app/main", "a) {", 0), "a: A");
    assert_eq!(f.hover("app/main", "label = ", 0), "label: String");
    assert_eq!(f.hover("app/main", "v) -> v", 0), "v: A");
    assert_eq!(f.hover("app/main", "o) {", 0), "o: Option(A)");
}

#[test]
fn type_of_other_things() {
    let f = fixture();
    assert_eq!(f.hover("app/main", "Some(v)", 0), "Some: (A) -> Option(A)");
    assert_eq!(f.hover("app/main", "None ->", 0), "None: Option(A)");
    assert_eq!(f.hover("app/main", "name", 0), "name: String");
    assert_eq!(
        f.hover("app/main", "User)", 0),
        "User = (name = String, age = Int)"
    );
    assert_eq!(f.hover("app/main", "2\n", 0), "Int");
    assert_eq!(f.hover("app/main", "util.", 0), "module app/util");
    let h = f
        .query()
        .type_at(f.module("app/main"), f.at("app/main", "add = ", 0, 0))
        .unwrap();
    assert_eq!(h.doc.as_deref(), Some("Adds two numbers."));
}

#[test]
fn definitions() {
    let f = fixture();
    let main = "app/main".to_string();
    // A local name goes to its binder.
    assert_eq!(
        f.definition("app/main", "z + y", 0),
        (main.clone(), "z".into())
    );
    assert_eq!(
        f.definition("app/main", "y\n}", 0),
        (main.clone(), "y".into())
    );
    assert_eq!(
        f.definition("app/main", "v\n", 0),
        (main.clone(), "v".into())
    );
    // A top-level name of this module and of others.
    assert_eq!(
        f.definition("app/main", "add(1", 0),
        (main.clone(), "add".into())
    );
    assert_eq!(
        f.definition("app/main", "double", 0),
        ("app/util".into(), "double".into())
    );
    assert_eq!(
        f.definition("app/main", "greet(u)", 0),
        ("app/user".into(), "greet".into())
    );
    // Constructors, types, and fields.
    assert_eq!(
        f.definition("app/main", "Some(v)", 0),
        (main.clone(), "Some".into())
    );
    assert_eq!(
        f.definition("app/main", "User)", 0),
        ("app/user".into(), "User".into())
    );
    assert_eq!(
        f.definition("app/main", "name", 0),
        ("app/user".into(), "name".into())
    );
    // A module, and the names in `import`.
    assert_eq!(
        f.definition("app/main", "util.", 0),
        ("app/util".into(), "".into())
    );
    assert_eq!(
        f.definition("app/main", "greet)", 0),
        ("app/user".into(), "greet".into())
    );
    // A built-in name, in the library.
    let f = Fixture::new(&[("src/main.i", "main = () { panic(\"no\") }\n")]);
    assert_eq!(
        f.definition("app/main", "panic", 0),
        ("i/prelude".into(), "panic".into())
    );
}

#[test]
fn definitions_with_a_syntax_error() {
    let f = Fixture::new(&[(
        "src/main.i",
        /* old-syntax-check: E0101 */
        "f = (n) { n + 1 }\ng = () { f(1) }\nh = (x) { x + }\n",
    )]);
    assert!(f.checked.diags.iter().any(|d| d.is_error()));
    assert_eq!(
        f.definition("app/main", "f(1", 0),
        ("app/main".into(), "f".into())
    );
}

fn labels(cs: &[(String, CompletionKind)]) -> Vec<&str> {
    cs.iter().map(|c| c.0.as_str()).collect()
}

#[test]
fn completions_in_scope() {
    let f = fixture();
    let cs = f.completions("app/main", "z + y", 0);
    let ls = labels(&cs);
    for name in [
        "z", "x", "y", "add", "id", "describe", "util", "greet", "User", "Some",
    ] {
        assert!(ls.contains(&name), "{name} in {ls:?}");
    }
    // Built-in names and types.
    for name in ["panic", "assert", "True", "Ok", "Int", "List"] {
        assert!(ls.contains(&name), "{name} in {ls:?}");
    }
    assert!(!ls.contains(&"case"));
    // Names of other functions are not in scope.
    assert!(!ls.contains(&"label") && !ls.contains(&"a"), "{ls:?}");
    assert!(cs.contains(&("z".into(), CompletionKind::Local)));
    assert!(cs.contains(&("add".into(), CompletionKind::Function)));
    assert!(cs.contains(&("total".into(), CompletionKind::Value)));
    // A binding is not in scope in its own value, unless it is direct.
    let cs = f.completions("app/main", "u.name", 0);
    assert!(!labels(&cs).contains(&"label"));
    assert!(labels(&cs).contains(&"u"));
}

#[test]
fn completions_of_members() {
    let f = fixture();
    // After `util.`: the exported names of the module.
    let cs = f.completions("app/main", "util.double", 5);
    assert_eq!(cs, vec![("double".to_string(), CompletionKind::Function)]);
    // With some letters already written.
    let cs = f.completions("app/main", "util.double", 7);
    assert_eq!(labels(&cs), vec!["double"]);
    // After `u.`: the fields of its type.
    let cs = f.completions("app/main", "u.name", 2);
    assert_eq!(labels(&cs), vec!["name", "age"]);
}

#[test]
fn completions_of_fields_from_inference() {
    let f = Fixture::new(&[(
        "src/main.i",
        "Point = (x = Int, y = Int)\n\
         origin = Point(x = 0, y = 0)\n\
         f = () {\n\
         \tp = origin\n\
         \tp.x + origin.y\n\
         }\n\
         g = (o: Point) {\n\
         \to |> (Point(..) ? True) { o.x } | (_) { panic(\"unreachable\") }\n\
         }\n\
         ",
    )]);
    assert!(!f.checked.diags.iter().any(|d| d.is_error()));
    // A local name whose type is inferred.
    let cs = f.completions("app/main", "p.x", 2);
    assert_eq!(labels(&cs), vec!["x", "y"]);
    // A top-level name.
    let cs = f.completions("app/main", "origin.y", 7);
    assert_eq!(labels(&cs), vec!["x", "y"]);
    // A name known to be made by a constructor (§7.5).
    let cs = f.completions("app/main", "o.x", 2);
    assert_eq!(labels(&cs), vec!["x", "y"]);
}

#[test]
fn symbols_of_a_module() {
    let f = fixture();
    let syms = f.query().symbols(f.module("app/main"));
    let names: Vec<(&str, SymbolKind)> = syms.iter().map(|s| (s.name.as_str(), s.kind)).collect();
    assert_eq!(
        names,
        vec![
            ("add", SymbolKind::Function),
            ("id", SymbolKind::Function),
            ("describe", SymbolKind::Function),
            ("first", SymbolKind::Function),
            ("Option", SymbolKind::Type),
            ("total", SymbolKind::Value),
        ]
    );
    let option = &syms[4];
    let ctors: Vec<&str> = option.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(ctors, vec!["Some", "None"]);
    assert_eq!(syms[0].detail.as_deref(), Some("(x: Int, y: Int) -> Int"));
    // The span is the whole item; the name span is the name.
    let text = f.text("app/main");
    let s = &syms[1];
    assert_eq!(
        &text[s.span.start as usize..s.span.end as usize],
        "id = (a) { a }"
    );
    assert_eq!(
        &text[s.name_span.start as usize..s.name_span.end as usize],
        "id"
    );
}

#[test]
fn types_in_a_program_with_errors() {
    let f = Fixture::new(&[(
        "src/main.i",
        "f = (n: Int) {\n  m = n + 1\n  m\n}\ng = () { nope }\n",
    )]);
    assert!(f.checked.diags.iter().any(|d| d.is_error()));
    assert_eq!(f.hover("app/main", "m = ", 0), "m: Int");
    assert_eq!(f.hover("app/main", "f = ", 0), "f: (n: Int) -> Int");
}

#[test]
fn types_of_parameters_that_share_a_type_variable() {
    let f = Fixture::new(&[(
        "src/main.i",
        "h3 = (f, x) { f(x) + 1 }\n\
         apply = (f, v) { f(v) }\n\
         h1 = (f) { f(1) }\n\
         id = (a) { a }\n",
    )]);
    assert!(!f.checked.diags.iter().any(|d| d.is_error()));
    // The parameter type of `f` is the type of `x`, which the scheme names.
    assert_eq!(
        f.hover("app/main", "h3", 0),
        "h3: (f: (A) -> Int ^ E ! N, x: A) -> Int ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "f, x", 0), "f: (A) -> Int ^ E ! N");
    assert_eq!(f.hover("app/main", "f(x)", 0), "f: (A) -> Int ^ E ! N");
    assert_eq!(f.hover("app/main", "x)", 0), "x: A");
    assert_eq!(f.hover("app/main", "f, v", 0), "f: (A) -> B ^ E ! N");
    assert_eq!(f.hover("app/main", "v)", 0), "v: A");
    // Not shared: as before.
    assert_eq!(f.hover("app/main", "f) {", 0), "f: (1) -> A ^ E ! N");
    assert_eq!(f.hover("app/main", "a) {", 0), "a: A");
}

#[test]
fn local_types_use_the_names_of_the_scheme() {
    let f = Fixture::new(&[(
        "src/main.i",
        "k = (f, x) {\n  y = f(x)\n  y\n}\n\
         compose = (f, g) {\n  (x) { g(f(x)) }\n}\n\
         pairup = (a, b) { (b, a) }\n\
         twice = (f) {\n  (z) { f(f(z)) }\n}\n\
         both = (f, x) {\n  w = (q) { (q, x) }\n  (f(x), w)\n}\n",
    )]);
    assert!(!f.checked.diags.iter().any(|d| d.is_error()));
    // The value of `f` is `y`, and the value of `k`.
    assert_eq!(
        f.hover("app/main", "k = ", 0),
        "k: (f: (A) -> B ^ E ! N, x: A) -> B ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "y = ", 0), "y: B");
    assert_eq!(f.hover("app/main", "y\n}", 0), "y: B");
    assert_eq!(
        f.hover("app/main", "f, x) {\n  y", 0),
        "f: (A) -> B ^ E ! N"
    );
    // The second parameter of `compose` is named as in its scheme.
    let compose = f.hover("app/main", "compose", 0);
    assert_eq!(
        compose,
        "compose: (f: (A) -> B ^ E ! N, g: (B) -> C ^ E2 ! N2) -> \
         (x: A) -> C ^ E | E2 ! N | N2"
    );
    assert_eq!(f.hover("app/main", "f, g", 0), "f: (A) -> B ^ E ! N");
    assert_eq!(f.hover("app/main", "g) {", 0), "g: (B) -> C ^ E2 ! N2");
    assert_eq!(f.hover("app/main", "x) { g", 0), "x: A");
    assert_eq!(f.hover("app/main", "g(f(x))", 0), "g: (B) -> C ^ E2 ! N2");
    // The names of the parameters are not the names of their types.
    assert_eq!(
        f.hover("app/main", "pairup", 0),
        "pairup: (a: A, b: B) -> (B, A)"
    );
    assert_eq!(f.hover("app/main", "a, b)", 0), "a: A");
    assert_eq!(f.hover("app/main", "b) {", 0), "b: B");
    assert_eq!(f.hover("app/main", "b, a)", 0), "b: B");
    assert_eq!(
        f.hover("app/main", "twice", 0),
        "twice: (f: (A | B) -> B & C ^ E ! N) -> (z: A) -> C ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "z) {", 0), "z: A");
    assert_eq!(
        f.hover("app/main", "f(f(z))", 0),
        "f: (A | B) -> B & C ^ E ! N"
    );
    // `w` is generalized in the block: its own variable is not one of the
    // scheme, and has a name that is not used there.
    assert_eq!(
        f.hover("app/main", "both = ", 0),
        "both: (f: (A) -> B ^ E ! N, x: A) -> (B, (q: C) -> (C, A)) ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "w = ", 0), "w: (q: D) -> (D, A)");
    assert_eq!(f.hover("app/main", "q) {", 0), "q: D");
    assert_eq!(f.hover("app/main", "x) {\n  w", 0), "x: A");
}

#[test]
fn local_types_name_one_sided_variables_as_the_scheme() {
    // A variable on one side only of the scheme has no effect, and is
    // shown as a name of its own there: a local name of that type shows
    // the same name.
    let f = Fixture::new(&[(
        "src/main.i",
        "one = (x) {\n  y = x\n  1\n}\n\
         drop = (x, g) {\n  y = x\n  g(1)\n}\n\
         drop2 = (x, g, h) {\n  y = (x, h)\n  g(1)\n}\n\
         effonly = (f, x) {\n  r = f(x)\n  Nil\n}\n",
    )]);
    assert!(!f.checked.diags.iter().any(|d| d.is_error()));
    assert_eq!(f.hover("app/main", "one", 0), "one: (x: A) -> 1");
    assert_eq!(f.hover("app/main", "x) {\n  y = x\n  1", 0), "x: A");
    assert_eq!(f.hover("app/main", "y = x\n  1", 0), "y: A");
    assert_eq!(f.hover("app/main", "x\n  1", 0), "x: A");
    assert_eq!(
        f.hover("app/main", "drop =", 0),
        "drop: (x: A, g: (1) -> B ^ E ! N) -> B ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "x, g) {\n  y = x\n  g", 0), "x: A");
    assert_eq!(f.hover("app/main", "y = x\n  g", 0), "y: A");
    assert_eq!(
        f.hover("app/main", "g) {\n  y = x", 0),
        "g: (1) -> B ^ E ! N"
    );
    assert_eq!(
        f.hover("app/main", "drop2", 0),
        "drop2: (x: A, g: (1) -> B ^ E ! N, h: C) -> B ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "x, g, h", 0), "x: A");
    assert_eq!(f.hover("app/main", "h) {", 0), "h: C");
    assert_eq!(f.hover("app/main", "y = (x, h)", 0), "y: (A, C)");
    assert_eq!(f.hover("app/main", "x, h)", 0), "x: A");
    assert_eq!(f.hover("app/main", "h)\n", 0), "h: C");
    assert_eq!(
        f.hover("app/main", "effonly", 0),
        "effonly: (f: (A) -> B ^ E ! N, x: A) -> Nil ^ E ! N"
    );
    assert_eq!(
        f.hover("app/main", "f, x) {\n  r", 0),
        "f: (A) -> B ^ E ! N"
    );
    assert_eq!(f.hover("app/main", "x) {\n  r", 0), "x: A");
    assert_eq!(f.hover("app/main", "f(x)\n  Nil", 0), "f: (A) -> B ^ E ! N");
    assert_eq!(f.hover("app/main", "r = ", 0), "r: B");
}

#[test]
fn completions_of_the_name_being_bound() {
    let f = Fixture::new(&[(
        "src/main.i",
        "g = (alpha) {\n  xs = [1, 2]\n  y = alpha\n  loop = (n) { loop(n) }\n  pair = (() { pair }, 1)\n  (xs, y, loop, pair)\n}\n",
    )]);
    // A direct value: its own names only inside its function bodies.
    let cs = f.completions("app/main", "1, 2]", 0);
    assert!(!labels(&cs).contains(&"xs"), "{cs:?}");
    let cs = f.completions("app/main", "alpha\n", 1);
    assert!(!labels(&cs).contains(&"y"), "{cs:?}");
    assert!(labels(&cs).contains(&"alpha"), "{cs:?}");
    let cs = f.completions("app/main", "loop(n)", 0);
    assert!(labels(&cs).contains(&"loop"), "{cs:?}");
    let cs = f.completions("app/main", "pair }", 0);
    assert!(labels(&cs).contains(&"pair"), "{cs:?}");
    let cs = f.completions("app/main", ", 1)", 2);
    assert!(!labels(&cs).contains(&"pair"), "{cs:?}");
}

#[test]
fn completions_after_any_expression() {
    let f = Fixture::new(&[(
        "src/main.i",
        "Point = (x = Int, y = Int)\n\
         make = () -> Point { Point(x = 1, y = 2) }\n\
         f = (t: Point) {\n\
         \ta = make().x\n\
         \tb = { t }.y\n\
         \ta + b\n\
         }\n\
         ",
    )]);
    assert!(!f.checked.diags.iter().any(|d| d.is_error()));
    let cs = f.completions("app/main", "make().x", 7);
    assert_eq!(labels(&cs), vec!["x", "y"]);
    let cs = f.completions("app/main", "{ t }.y", 6);
    assert_eq!(labels(&cs), vec!["x", "y"]);
    // With some letters written.
    let cs = f.completions("app/main", "make().x", 8);
    assert_eq!(labels(&cs), vec!["x", "y"]);
}

#[test]
fn new_match_scopes_cover_subjects_and_hide_synthetic_names() {
    // A1 §3.12: the made-up binding covers the subject too, and its names
    // enter scope only in the body. Synthetic parameters never appear.
    let source = "main = () {\n\tlocal = 4\n\t(x) { local + x } |> (f) { f(local) }\n}\nmany = (0) { 1 } | (_) { 2 }\nuse_many = () { apply(1, (0) { 1 } | (_) { 2 }) }\napply = (x, f) { f(x) }";
    let fixture = Fixture::new(&[("src/main.i", source)]);
    let subject = fixture.completions("app/main", "local + x", 0);
    assert!(subject.iter().any(|(n, _)| n == "local"), "{subject:?}");
    assert!(subject.iter().any(|(n, _)| n == "x"), "{subject:?}");
    assert!(!subject.iter().any(|(n, _)| n == "f"), "{subject:?}");
    let body = fixture.completions("app/main", "f(local)", 0);
    assert!(body.iter().any(|(n, _)| n == "f"), "{body:?}");
    for needle in ["{ 0 ->", ",{ 0 ->"] {
        let at = fixture.at("app/main", needle, 0, 0);
        let query = fixture.query();
        let module = fixture.module("app/main");
        assert!(
            !query
                .completions_at(module, at)
                .iter()
                .any(|c| c.label.starts_with('%'))
        );
        if let Some(hover) = query.type_at(module, at) {
            assert!(!hover.code.contains("%arg"), "{}", hover.code);
        }
        assert!(query.definition_at(module, at).is_none());
    }
    assert!(
        fixture
            .hover("app/main", "many", 0)
            .contains("(A) -> 1 | 2")
    );
}

#[test]
fn private_value_and_written_alias_hovers_limit_literals_only_in_human_text() {
    // Decision 11: the hover is limited; Typed::schemes and package text stay full.
    let lits = (0..14).map(|i| format!("\"m{i:02}\"")).collect::<Vec<_>>();
    let source = format!("Modes = {}\nxs = [{}]\n", lits.join(" | "), lits.join(", "));
    let f = Fixture::new(&[("src/main.i", &source)]);
    assert!(
        !f.checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        f.checked.diags
    );
    for name in ["Modes", "xs ="] {
        let text = f.hover("app/main", name, 0);
        assert!(text.contains("… (4 more)"), "{text}");
        assert!(!text.contains("m13"), "{text}");
    }
    let types = f.checked.types.as_ref().unwrap();
    assert!(
        types
            .schemes
            .values()
            .any(|t| t.contains("m13") && !t.contains('…'))
    );
}

// ---- The queries for references, rename, navigation and code actions ----

const SHAPES_Q: &str = "(\n\tShape(..)\n\tarea\n)\n\nCircle = (radius = Int)\nDot = ()\nShape = Circle | Dot\n\narea = (s: Shape) -> Int {\n\ts |> (Circle(radius = radius)) { radius * 3 } | (Dot) { 0 }\n}\n";
const MAIN_Q: &str = "(\n\t(Shape, Circle, area) = app/shapes\n)\n\nmain = () {\n\tc = Circle(radius = 2)\n\tarea(c) + area(Circle(radius = 1))\n}\n";

fn shapes_fixture() -> Fixture {
    Fixture::new(&[("src/shapes.i", SHAPES_Q), ("src/main.i", MAIN_Q)])
}

fn texts_of(f: &Fixture, refs: &[ilang_check::query::Reference]) -> Vec<(String, String)> {
    let p = f.checked.program.as_ref().unwrap();
    refs.iter()
        .map(|r| {
            let text = &f.graph.sources.get(r.location.source).text;
            (
                p.module(r.location.module).path.clone(),
                text[r.location.span.start as usize..r.location.span.end as usize].to_string(),
            )
        })
        .collect()
}

#[test]
fn query_references_follow_identity_across_modules() {
    let f = shapes_fixture();
    let q = f.query();
    let m = f.module("app/main");
    let at = f.at("app/main", "area(c)", 0, 1);
    let (owner, _, target) = q.target_at_offset(m, at).unwrap();
    let refs = q.references(&target, owner, &q.own_modules());
    let found = texts_of(&f, &refs);
    assert_eq!(
        found.iter().filter(|(p, _)| p == "app/main").count(),
        3,
        "{found:?}"
    );
    assert_eq!(
        found.iter().filter(|(p, _)| p == "app/shapes").count(),
        2,
        "{found:?}"
    );
    assert!(found.iter().all(|(_, t)| t == "area"), "{found:?}");
    assert_eq!(refs.iter().filter(|r| r.definition).count(), 1);
    // A field: the label of the definition, the labels of the calls, and
    // the pattern part, which also binds a local.
    let at = f.at("app/main", "radius: 2", 0, 0);
    let (owner, _, target) = q.target_at_offset(m, at).unwrap();
    let refs = q.references(&target, owner, &q.own_modules());
    assert_eq!(refs.len(), 4, "{:?}", texts_of(&f, &refs));
    assert_eq!(refs.iter().filter(|r| r.label_binder).count(), 1);
}

#[test]
fn query_rename_checks_kind_and_scope() {
    let f = shapes_fixture();
    let q = f.query();
    let m = f.module("app/main");
    let at = f.at("app/main", "c = ", 0, 0);
    assert!(q.rename(m, at, "Big").is_err());
    let e = q.rename(m, at, "area").unwrap_err();
    assert!(e.0.contains("area"), "{e}");
    let edits = q.rename(m, at, "disk").unwrap();
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|e| e.text == "disk"));
    // A constructor, in both modules.
    let at = f.at("app/main", "Circle(radius: 2)", 0, 0);
    let edits = q.rename(m, at, "Disk").unwrap();
    assert_eq!(edits.len(), 5, "{edits:?}");
    // Refused for the library.
    let text = "main = () { panic(\"x\") }\n";
    let g = Fixture::new(&[("src/main.i", text)]);
    let e = g
        .query()
        .prepare_rename(g.module("app/main"), g.at("app/main", "panic", 0, 1))
        .unwrap_err();
    assert!(e.0.contains("library"), "{e}");
}

#[test]
fn query_missing_clauses_of_a_match() {
    let text = "Circle = (Int)\nDot = ()\nLine = ()\nShape = Circle | Dot | Line\n\nf = (s: Shape) { s |> (Dot) { 0 } | (Circle(1)) { 1 } }\n";
    let f = Fixture::new(&[("src/main.i", text)]);
    let q = f.query();
    let m = f.module("app/main");
    let span = ilang_syntax::span::Span::new(
        f.at("app/main", "s |>", 0, 0),
        f.at("app/main", "s |>", 0, 1),
    );
    let c = q.missing_clauses(m, span).unwrap();
    assert_eq!(c.patterns, vec!["Circle(..)", "Line"]);
    assert_eq!(
        c.edit.text,
        "    Circle(..) -> panic(\"todo\")\n    Line -> panic(\"todo\")\n"
    );
}

#[test]
fn query_type_definition_signature_and_hints() {
    let f = shapes_fixture();
    let q = f.query();
    let m = f.module("app/main");
    let types = q.type_definition_at(m, f.at("app/main", "area(c)", 0, 5));
    assert_eq!(types.len(), 1);
    assert_eq!(
        f.graph.sources.get(types[0].source).text
            [types[0].span.start as usize..types[0].span.end as usize]
            .to_string(),
        "Shape"
    );
    let s = q
        .signature_help(m, f.at("app/main", "area(c)", 0, 5))
        .unwrap();
    assert_eq!(s.label, "area(s: Shape) -> Int");
    assert_eq!(s.active, Some(0));
    let hints = q.inlay_hints(m, ilang_syntax::span::Span::new(0, MAIN_Q.len() as u32));
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert_eq!(hints[0].label, ": Shape");
}

#[test]
fn query_completion_context() {
    use ilang_check::query::CompletionContext;
    let f = shapes_fixture();
    let q = f.query();
    let shapes = f.module("app/shapes");
    assert_eq!(
        q.completion_context(shapes, f.at("app/shapes", "Shape) ->", 0, 1)),
        CompletionContext::Type
    );
    match q.completion_context(shapes, f.at("app/shapes", "Dot -> 0", 0, 0)) {
        CompletionContext::Pattern { types } => assert_eq!(types.len(), 1),
        other => panic!("{other:?}"),
    }
    let main = f.module("app/main");
    assert!(matches!(
        q.completion_context(main, f.at("app/main", "app/shapes", 0, 4)),
        CompletionContext::Import { names_of: None, .. }
    ));
    assert!(matches!(
        q.completion_context(main, f.at("app/main", "radius: 2", 0, 0)),
        CompletionContext::Expression { call: Some(_) }
    ));
}

#[test]
fn query_unused_imports_and_fixes() {
    let text = "(\n  app/shapes\n)\n\nmain = () { area(Dot) }\n";
    let f = Fixture::new(&[("src/shapes.i", SHAPES_Q), ("src/main.i", text)]);
    let q = f.query();
    let m = f.module("app/main");
    let unused = q.unused_imports(m);
    assert_eq!(unused.len(), 1);
    assert_eq!(unused[0].1, "app/shapes");
    let fixes = q.missing_import_fixes(
        m,
        ilang_syntax::span::Span::new(
            f.at("app/main", "area(", 0, 0),
            f.at("app/main", "area(", 0, 4),
        ),
    );
    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].edits[0].text, "(area) = ");
}
