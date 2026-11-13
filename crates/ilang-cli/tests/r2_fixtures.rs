//! Additional JavaScript execution of the hand-authored R2 spec bodies.
//! R2_FIXTURE_DUMP generates fixtures; ordinary runs check the transformation.
use ilang_cli::{BuildOptions, Level, build};
use ilang_syntax::{
    ast::{DItem, ExprKind, ItemKind, PatternKind},
    parse_module,
};
use std::{fs, path::PathBuf};

/// The byte range of a header group `( … )` from the span the tree gives it:
/// the whole group, or (syntax three's lowering) only its entries.
fn group(text: &str, span: ilang_syntax::span::Span) -> (usize, usize) {
    let (start, end) = (span.start as usize, span.end as usize);
    if text[start..].starts_with('(') && text[..end].ends_with(')') {
        return (start, end);
    }
    let open = text[..start].rfind('(').unwrap();
    let close = end + text[end..].find(')').unwrap() + 1;
    (open, close)
}

struct Case {
    name: String,
    module: String,
    expectation: String,
    output: Option<Vec<String>>,
}

fn package() -> (tempfile::TempDir, Vec<Case>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    let mut cases = Vec::new();
    for section in ["9.9", "9.11"] {
        let mut files: Vec<_> = fs::read_dir(root.join("tests/spec").join(section))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "i"))
            .collect();
        files.sort();
        for file in files {
            let text = fs::read_to_string(&file).unwrap();
            let stem = file.file_stem().unwrap().to_str().unwrap();
            let expectation = text
                .lines()
                .find_map(|line| line.strip_prefix("// expect: "))
                .unwrap();
            if expectation.starts_with("compile-error") {
                continue;
            }
            assert!(expectation == "pass" || expectation.starts_with("stop \""));
            let (module, diagnostics) = parse_module(&text);
            assert!(
                diagnostics.is_empty(),
                "{}: {diagnostics:?}",
                file.display()
            );
            let mut names: Vec<_> = module.exports.iter().map(|e| e.name.name.clone()).collect();
            names.push("r2_probe".into());
            for item in &module.items {
                if let ItemKind::TypeDef(def) = &item.kind {
                    names.push(def.name.name.clone());
                }
            }
            names.sort();
            names.dedup();
            let exports = format!("\n({})\n", names.join(", "));
            let mut edits = Vec::new();
            let mut entries = 0;
            for item in &module.items {
                if let ItemKind::Bind(binding) = &item.kind
                    && let PatternKind::Var(name) = &binding.pattern.kind
                    && name == "main"
                {
                    entries += 1;
                }
                for attribute in &item.attributes {
                    if attribute.module.is_none() && attribute.name.name == "host" {
                        let Some(DItem::Pos(value)) = attribute.args.as_deref().and_then(|args| args.first()) else {
                            panic!("host attribute has its namespace argument")
                        };
                        let ExprKind::String(namespace) = &value.kind else {
                            panic!("host namespace is a written string")
                        };
                        let span = value.span.0;
                        edits.push((
                            span.start as usize,
                            span.end as usize,
                            format!("{:?}", format!("r2:law:{section}:{stem}:{namespace}")),
                        ));
                    }
                }
            }
            assert_eq!(entries, 1, "one main in {}", file.display());
            let exports_group = module.exports.iter().map(|item| item.span.0).reduce(ilang_syntax::span::Span::to).map(|span| group(&text, span));
            let imports_group = module.imports.iter().map(|item| item.span.0).reduce(ilang_syntax::span::Span::to).map(|span| group(&text, span));
            if let Some((start, end)) = exports_group {
                edits.push((start, end, exports));
            } else {
                let at = imports_group.map_or(0, |(_, end)| end);
                edits.push((at, at, exports));
            }
            if let Some((_, end)) = imports_group {
                let at = end - 1;
                edits.push((at, at, "\n r2_fail = i/fail\n r2_core = i/core\n".into()));
            } else {
                edits.push((0, 0, "(r2_fail = i/fail, r2_core = i/core)\n".into()));
            }
            edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
            let mut adapted = text.clone();
            for (start, end, replacement) in edits {
                adapted.replace_range(start..end, &replacement);
            }
            adapted.push_str("r2_probe = () {\n\tresult = r2_fail.catch(() { main() })\n\tresult |> (Ok(_)) { Nil } | (Error(e)) { panic(r2_core.inspect(e)) }\n}\n");
            let (parsed, diagnostics) = parse_module(&adapted);
            assert!(
                diagnostics.is_empty(),
                "{}: {diagnostics:?}",
                file.display()
            );
            assert!(
                !parsed
                    .items
                    .iter()
                    .any(|item| item.attributes.iter().any(|a| a.name.name == "main"))
            );
            let module = format!("s{}_{}", section.replace('.', "_"), stem);
            fs::write(
                directory.path().join("src").join(format!("{module}.i")),
                adapted,
            )
            .unwrap();
            let output: Vec<_> = text
                .lines()
                .filter_map(|line| line.strip_prefix("// output: ").map(str::to_owned))
                .collect();
            cases.push(Case {
                name: format!("{section}/{stem}"),
                module,
                expectation: expectation.into(),
                output: if output.is_empty() {
                    None
                } else {
                    Some(output)
                },
            });
        }
    }
    let exports = cases
        .iter()
        .map(|case| format!("\"app/{}.r2_probe\"", case.module))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        directory.path().join("i.toml"),
        format!("name = \"app\"\n[wasm]\nexports = [{exports}]\n"),
    )
    .unwrap();
    (directory, cases)
}

#[test]
fn conformance_fixture_sources_parse() {
    let _ = package();
}

#[test]
fn generate_conformance_fixtures() {
    let Some(out) = std::env::var_os("R2_FIXTURE_DUMP") else {
        return;
    };
    let out = PathBuf::from(out);
    fs::create_dir_all(&out).unwrap();
    let (directory, cases) = package();
    for (opt, level) in [(Level::O0, 0), (Level::O2, 2)] {
        let built = build(
            directory.path(),
            BuildOptions {
                opt,
                tests: false,
                offline: true,
                target: Default::default(),
            },
        )
        .unwrap();
        fs::write(out.join(format!("laws-o{level}.wasm")), &built.wasm).unwrap();
        fs::write(
            out.join(format!("laws-o{level}.bindings.json")),
            built.binding_plan.to_json().unwrap(),
        )
        .unwrap();
    }
    let mut metadata = String::from(
        "// Generated from tests/spec/9.9 and 9.11 by R2_FIXTURE_DUMP.\nexport default [\n",
    );
    for case in &cases {
        metadata.push_str(&format!(
            "  {{ name: {:?}, entry: {:?}, expectation: {:?}, output: {} }},\n",
            case.name,
            format!("app/{}.r2_probe", case.module),
            case.expectation,
            case.output
                .as_ref()
                .map_or_else(|| "null".into(), |output| format!("{output:?}"))
        ));
    }
    metadata.push_str("];\n");
    fs::write(out.join("cases.mjs"), metadata).unwrap();
}
