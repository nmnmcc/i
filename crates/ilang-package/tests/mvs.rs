//! Minimal version selection (§2.7), without git.

use ilang_package::manifest::{Version, parse_version};
use ilang_package::mvs::{PackageKey, Requirement, select};
use std::collections::HashMap;

fn req(s: &str) -> Requirement<()> {
    let (loc, v) = s.split_once('@').unwrap();
    let version = parse_version(v).unwrap();
    Requirement {
        key: PackageKey::new(loc, version),
        version,
        from: (),
    }
}

fn v(s: &str) -> Version {
    parse_version(s).unwrap()
}

/// A location, a major version, and the version used.
type Chosen = (String, u64, String);

/// A world of `i.toml` files: `"a@1.0.0" => ["b@1.2.0", ...]`.
fn run(roots: &[&str], world: &[(&str, &[&str])]) -> (Vec<Chosen>, Vec<String>, Vec<String>) {
    let world: HashMap<String, Vec<String>> = world
        .iter()
        .map(|(k, ds)| (k.to_string(), ds.iter().map(|d| d.to_string()).collect()))
        .collect();
    let mut asked = Vec::new();
    let (sel, errors) = select(roots.iter().map(|r| req(r)), |r| {
        let name = format!("{}@{}", r.key.location, r.version);
        asked.push(name.clone());
        match world.get(&name) {
            Some(ds) => Ok(ds.iter().map(|d| req(d)).collect()),
            None => Err(name),
        }
    });
    let versions = sel
        .versions
        .iter()
        .map(|(k, v)| (k.location.clone(), k.major, v.to_string()))
        .collect();
    (versions, asked, errors)
}

fn s(location: &str, major: u64, version: &str) -> Chosen {
    (location.to_string(), major, version.to_string())
}

#[test]
fn diamond_takes_the_highest_reached() {
    let (versions, asked, errors) = run(
        &["a@1.0.0", "b@1.0.0"],
        &[
            ("a@1.0.0", &["c@1.1.0"]),
            ("b@1.0.0", &["c@1.2.0"]),
            ("c@1.1.0", &[]),
            ("c@1.2.0", &[]),
            // Never reached, so never used.
            ("c@1.3.0", &[]),
        ],
    );
    assert!(errors.is_empty());
    assert_eq!(
        versions,
        [s("a", 1, "1.0.0"), s("b", 1, "1.0.0"), s("c", 1, "1.2.0")]
    );
    // Every reached version has its `i.toml` read once.
    let mut asked = asked;
    asked.sort();
    assert_eq!(asked, ["a@1.0.0", "b@1.0.0", "c@1.1.0", "c@1.2.0"]);
}

#[test]
fn old_versions_still_count() {
    // `a@1.0.0` is not used, as `b` raises `a` to 1.1.0, but its
    // dependencies are reached: `d@1.5.0` and `e`.
    let (versions, _, errors) = run(
        &["a@1.0.0", "b@1.0.0"],
        &[
            ("a@1.0.0", &["d@1.5.0", "e@0.1.0"]),
            ("a@1.1.0", &["d@1.2.0"]),
            ("b@1.0.0", &["a@1.1.0"]),
            ("d@1.2.0", &[]),
            ("d@1.5.0", &[]),
            ("e@0.1.0", &[]),
        ],
    );
    assert!(errors.is_empty());
    assert_eq!(
        versions,
        [
            s("a", 1, "1.1.0"),
            s("b", 1, "1.0.0"),
            s("d", 1, "1.5.0"),
            s("e", 0, "0.1.0")
        ]
    );
}

#[test]
fn majors_are_different_packages() {
    let (versions, _, errors) = run(
        &["j@1.4.0", "j@2.0.1"],
        &[
            ("j@1.4.0", &[]),
            ("j@2.0.1", &["j@1.6.0"]),
            ("j@1.6.0", &[]),
        ],
    );
    assert!(errors.is_empty());
    assert_eq!(versions, [s("j", 1, "1.6.0"), s("j", 2, "2.0.1")]);
}

#[test]
fn circles_end_and_errors_are_kept() {
    let (versions, _, errors) = run(
        &["a@1.0.0"],
        &[
            ("a@1.0.0", &["b@1.0.0"]),
            ("b@1.0.0", &["a@1.0.0", "x@3.0.0"]),
        ],
    );
    assert_eq!(errors, ["x@3.0.0"]);
    // A version that cannot be read is still reached.
    assert_eq!(
        versions,
        [s("a", 1, "1.0.0"), s("b", 1, "1.0.0"), s("x", 3, "3.0.0")]
    );
}

#[test]
fn origin_is_the_first_requirement() {
    let roots = vec![
        Requirement {
            key: PackageKey::new("a", v("1.0.0")),
            version: v("1.0.0"),
            from: "root",
        },
        Requirement {
            key: PackageKey::new("b", v("1.0.0")),
            version: v("1.0.0"),
            from: "root",
        },
    ];
    let (sel, _) = select::<_, ()>(roots, |r| {
        Ok(if r.key.location == "b" {
            vec![Requirement {
                key: PackageKey::new("a", v("1.2.0")),
                version: v("1.2.0"),
                from: "b",
            }]
        } else {
            vec![]
        })
    });
    assert_eq!(sel.origin(&PackageKey::new("a", v("1.0.0"))), Some(&"b"));
    assert_eq!(sel.origin(&PackageKey::new("b", v("1.0.0"))), Some(&"root"));
}
