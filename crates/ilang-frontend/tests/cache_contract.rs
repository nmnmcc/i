//! Semantic cache regressions exercise legal user names that resemble compiler
//! metadata. Warm checks must agree with a fresh check, including argument order.

use std::{fs, path::Path};

use ilang_core::ast::{Statement, TypedExpr};
use ilang_frontend::{CheckOptions, CheckedProject, Project, check_project};
use tempfile::TempDir;

fn write(root: &Path, file: &str, text: &str) {
    fs::write(root.join(file), text).unwrap();
}

fn project(caller: &str, dependency: &str) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    write(
        root.path(),
        "i.toml",
        "name = \"app\"\nversion = \"1.0.0\"\n",
    );
    write(root.path(), "src/app.i", caller);
    write(root.path(), "src/dep.i", dependency);
    root
}

fn call_arguments(project: &CheckedProject) -> Vec<String> {
    let main = &project.module("app").unwrap().definitions.functions[0];
    let Statement::Expression(TypedExpr::Call { arguments, .. }) = &main.body[0] else {
        panic!("fixture main must be a call")
    };
    arguments
        .iter()
        .map(|argument| {
            let TypedExpr::Int { value, .. } = &argument.value else {
                panic!("fixture arguments must be integers")
            };
            value.to_string()
        })
        .collect()
}

#[test]
fn parameter_labels_named_start_and_end_invalidate_argument_order() {
    for (first, second) in [("start", "end"), ("location", "documentation")] {
        let root = project(
            &format!("import dep\npub fn main() {{ dep.subtract({first}: 10, {second}: 3) }}\n"),
            &format!("pub fn subtract({first} x: Int, {second} y: Int) -> Int {{ x - y }}\n"),
        );
        let initial = check_project(root.path(), &CheckOptions::default()).unwrap();
        assert_eq!(call_arguments(&initial), ["10", "3"]);
        write(
            root.path(),
            "src/dep.i",
            &format!("pub fn subtract({second} x: Int, {first} y: Int) -> Int {{ x - y }}\n",),
        );
        let warm = check_project(root.path(), &CheckOptions::default()).unwrap();
        let fresh = check_project(
            root.path(),
            &CheckOptions {
                use_cache: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            warm.stats.checked, 2,
            "changed labels must invalidate the caller"
        );
        assert_eq!(call_arguments(&warm), ["3", "10"]);
        assert_eq!(call_arguments(&warm), call_arguments(&fresh));
    }
}

#[test]
fn renamed_public_function_cannot_survive_in_memory_or_disk_caches() {
    for in_memory in [false, true] {
        let root = project(
            "import dep\npub fn main() { dep.location() }\n",
            "pub fn location() -> Int { 1 }\n",
        );
        let mut session = Project::load(root.path()).unwrap();
        session.check(&CheckOptions::default()).unwrap();
        write(
            root.path(),
            "src/dep.i",
            "pub fn documentation() -> Int { 1 }\n",
        );
        let warm = if in_memory {
            session.check(&CheckOptions::default())
        } else {
            check_project(root.path(), &CheckOptions::default())
        };
        let fresh = check_project(
            root.path(),
            &CheckOptions {
                use_cache: false,
                ..Default::default()
            },
        );
        for result in [warm, fresh] {
            let diagnostic = result.unwrap_err().to_string();
            assert!(diagnostic.contains("location"), "{diagnostic}");
            assert!(diagnostic.contains("app.i"), "{diagnostic}");
        }
    }
}

#[test]
fn all_metadata_like_export_names_preserve_their_types() {
    for name in [
        "location",
        "definition_location",
        "src_path",
        "line_numbers",
        "documentation",
        "warnings",
        "references",
        "contains_echo",
        "arguments_start_position",
        "operator_start",
    ] {
        let root = project(
            &format!("import dep\npub fn main() {{ dep.{name}() + 1 }}\n"),
            &format!("pub fn {name}() -> Int {{ 1 }}\n"),
        );
        check_project(root.path(), &CheckOptions::default()).unwrap();
        write(
            root.path(),
            "src/dep.i",
            &format!("pub fn {name}() -> String {{ \"changed\" }}\n"),
        );
        let diagnostic = check_project(root.path(), &CheckOptions::default())
            .unwrap_err()
            .to_string();
        assert!(diagnostic.contains("String"), "{name}: {diagnostic}");
        assert!(diagnostic.contains("Int"), "{name}: {diagnostic}");
    }
}

#[test]
fn metadata_like_record_labels_remain_in_the_interface() {
    let root = project(
        "import dep\npub fn main() { dep.Box(location: 1).location }\n",
        "pub type Box { Box(location: Int) }\n",
    );
    check_project(root.path(), &CheckOptions::default()).unwrap();
    write(
        root.path(),
        "src/dep.i",
        "pub type Box { Box(documentation: Int) }\n",
    );
    assert!(check_project(root.path(), &CheckOptions::default()).is_err());
    assert!(
        check_project(
            root.path(),
            &CheckOptions {
                use_cache: false,
                ..Default::default()
            }
        )
        .is_err()
    );
}
