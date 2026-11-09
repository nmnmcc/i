//! The rules of `i.toml` (§2.6) and of locations and versions.

use ilang_package::code;
use ilang_package::manifest::{
    DependencySource, Location, Version, parse_location, parse_manifest, parse_version,
};

/// The codes of the problems in a manifest.
fn codes(text: &str) -> Vec<&'static str> {
    parse_manifest(text).1.iter().map(|d| d.code).collect()
}

#[test]
fn full_manifest() {
    let (m, diags) = parse_manifest(
        r#"
name = "app"
unknown = 1

[dependencies]
json = "github.com/alice/json@1.4.0"
text = "codeberg.org/bob/tools//text@0.3.2"
util = { path = "../util", other = true }

[dev-dependencies]
check = "github.com/carol/check@2.1.0"

[wasm]
exports = ["app/math.add", "app/a/b.c"]
other = 2

[other]
x = 1
"#,
    );
    assert!(diags.is_empty(), "{diags:?}");
    let m = m.unwrap();
    assert_eq!(m.name, "app");
    let deps: Vec<(&str, &DependencySource)> = m
        .dependencies
        .iter()
        .map(|d| (d.name.as_str(), &d.source))
        .collect();
    assert_eq!(deps.len(), 3);
    assert!(deps.contains(&(
        "json",
        &DependencySource::Location(Location {
            repository: "github.com/alice/json".into(),
            dir: None,
            version: Version {
                major: 1,
                minor: 4,
                patch: 0
            },
        })
    )));
    assert!(deps.contains(&(
        "text",
        &DependencySource::Location(Location {
            repository: "codeberg.org/bob/tools".into(),
            dir: Some("text".into()),
            version: Version {
                major: 0,
                minor: 3,
                patch: 2
            },
        })
    )));
    assert!(deps.contains(&("util", &DependencySource::Path("../util".into()))));
    assert_eq!(m.dev_dependencies.len(), 1);
    let exports: Vec<(&str, &str)> = m
        .wasm_exports
        .iter()
        .map(|e| (e.module.as_str(), e.function.as_str()))
        .collect();
    assert_eq!(exports, [("app/math", "add"), ("app/a/b", "c")]);
}

#[test]
fn name_rules() {
    assert_eq!(codes(""), [code::MANIFEST_NAME]);
    assert_eq!(codes("name = 1"), [code::MANIFEST_NAME]);
    assert_eq!(codes(r#"name = "i""#), [code::MANIFEST_NAME]);
    assert_eq!(codes(r#"name = "App""#), [code::MANIFEST_NAME]);
    assert_eq!(codes(r#"name = "my-app""#), [code::MANIFEST_NAME]);
    assert_eq!(codes(r#"name = "__x""#), [code::MANIFEST_NAME]);
    assert_eq!(codes(r#"name = "case""#), [code::MANIFEST_NAME]);
    assert!(codes(r#"name = "_my_app2""#).is_empty());
    assert!(parse_manifest("").0.is_none());
}

#[test]
fn toml_syntax() {
    assert_eq!(codes("name = "), [code::MANIFEST_SYNTAX]);
    let (_, d) = parse_manifest("name = \"a\"\nname = \"b\"\n");
    assert_eq!(d[0].code, code::MANIFEST_SYNTAX);
    assert_eq!(d[0].span.start, 11);
}

#[test]
fn dependency_names() {
    let deps = |body: &str| codes(&format!("name = \"app\"\n[dependencies]\n{body}"));
    assert_eq!(deps(r#"i = { path = "x" }"#), [code::DEPENDENCY_NAME]);
    assert_eq!(deps(r#"app = { path = "x" }"#), [code::DEPENDENCY_NAME]);
    assert_eq!(deps(r#"Json = { path = "x" }"#), [code::DEPENDENCY_NAME]);
    assert_eq!(deps(r#"if = { path = "x" }"#), [code::DEPENDENCY_NAME]);
    assert!(deps(r#"json = { path = "x" }"#).is_empty());
    // Distinct across both tables.
    assert_eq!(
        codes(
            "name = \"app\"\n[dependencies]\njson = { path = \"a\" }\n[dev-dependencies]\njson = { path = \"b\" }\n"
        ),
        [code::DEPENDENCY_TWICE]
    );
    let (m, _) = parse_manifest(
        "name = \"app\"\n[dependencies]\njson = { path = \"a\" }\n[dev-dependencies]\njson = { path = \"b\" }\n",
    );
    assert!(m.unwrap().dev_dependencies.is_empty());
}

#[test]
fn dependency_values() {
    let deps = |body: &str| codes(&format!("name = \"app\"\n[dependencies]\n{body}"));
    assert_eq!(deps("a = 1"), [code::DEPENDENCY_VALUE]);
    assert_eq!(deps("a = { other = \"x\" }"), [code::DEPENDENCY_VALUE]);
    assert_eq!(deps("a = { path = 1 }"), [code::DEPENDENCY_VALUE]);
    assert_eq!(deps("a = \"github.com/a/b\""), [code::BAD_LOCATION]);
    assert_eq!(deps("a = \"github.com/a/b@1.2\""), [code::BAD_VERSION]);
    assert_eq!(
        codes("name = \"app\"\ndependencies = 1"),
        [code::MANIFEST_SHAPE]
    );
    assert_eq!(codes("name = \"app\"\nwasm = 1"), [code::MANIFEST_SHAPE]);
}

#[test]
fn wasm_exports() {
    let w = |body: &str| codes(&format!("name = \"app\"\n[wasm]\nexports = {body}"));
    assert!(w(r#"["app/math.add"]"#).is_empty());
    assert_eq!(w(r#""app/math.add""#), [code::WASM_EXPORT]);
    assert_eq!(w(r#"[1]"#), [code::WASM_EXPORT]);
    assert_eq!(w(r#"["app.add"]"#), [code::WASM_EXPORT]);
    assert_eq!(w(r#"["app/math"]"#), [code::WASM_EXPORT]);
    assert_eq!(w(r#"["app/Math.add"]"#), [code::WASM_EXPORT]);
    assert_eq!(w(r#"["app/math.Add"]"#), [code::WASM_EXPORT]);
}

#[test]
fn path_dependencies_only_by_path() {
    let (m, _) = parse_manifest(
        "name = \"lib\"\n[dependencies]\na = { path = \"x\" }\nb = \"github.com/a/b@1.0.0\"\n[dev-dependencies]\nc = { path = \"y\" }\n",
    );
    let m = m.unwrap();
    assert!(m.check_path_dependencies(false).is_empty());
    let d: Vec<_> = m
        .check_path_dependencies(true)
        .iter()
        .map(|d| d.code)
        .collect();
    assert_eq!(d, [code::PATH_DEPENDENCY, code::PATH_DEPENDENCY]);
}

#[test]
fn locations() {
    let ok = |s: &str| parse_location(s).unwrap();
    let l = ok("github.com/alice/json@1.4.0");
    assert_eq!(l.repository, "github.com/alice/json");
    assert_eq!(l.dir, None);
    let l = ok("codeberg.org/bob/tools//text/sub@10.20.30");
    assert_eq!(l.repository, "codeberg.org/bob/tools");
    assert_eq!(l.dir.as_deref(), Some("text/sub"));
    assert_eq!(
        l.version,
        Version {
            major: 10,
            minor: 20,
            patch: 30
        }
    );
    let bad = |s: &str| parse_location(s).unwrap_err().0;
    assert_eq!(bad("github.com/alice/json"), code::BAD_LOCATION);
    assert_eq!(
        bad("https://github.com/alice/json@1.0.0"),
        code::BAD_LOCATION
    );
    assert_eq!(bad("json@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com//json@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com/a/b//@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com/a/b//x//y@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com/a b/c@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com/a/../c@1.0.0"), code::BAD_LOCATION);
    assert_eq!(bad("github.com/a/b@v1.0.0"), code::BAD_VERSION);
}

#[test]
fn versions() {
    assert_eq!(
        parse_version("0.0.0"),
        Some(Version {
            major: 0,
            minor: 0,
            patch: 0
        })
    );
    for bad in [
        "1",
        "1.2",
        "1.2.3.4",
        "1.2.x",
        "01.2.3",
        "1.2.-3",
        "1..3",
        "",
        "1.2.3-beta",
        "+1.2.3",
    ] {
        assert_eq!(parse_version(bad), None, "{bad}");
    }
}
