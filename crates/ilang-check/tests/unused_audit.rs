//! Repository warning inventory. Run with UNUSED_AUDIT=<output directory>.
//! Like wat_dump, this writes artifacts only when explicitly requested.
use ilang_check::code;
use ilang_package::{Diagnostic, ModuleKind};
use ilang_syntax::{ast, span::LineIndex};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[test]
fn unused_audit() {
    let Ok(out) = std::env::var("UNUSED_AUDIT") else {
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let listed = std::process::Command::new("git")
        .args(["ls-files", "-z", "*.i"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(listed.status.success());
    let files = String::from_utf8(listed.stdout).unwrap();
    let mut projects = BTreeSet::new();
    let mut singles = Vec::new();
    let mut tests = 0;
    let mut test_counts = BTreeMap::<String, usize>::new();
    let mut zero = Vec::new();
    for relative in files.split('\0').filter(|s| !s.is_empty()) {
        let path = root.join(relative);
        let text = std::fs::read_to_string(&path).unwrap();
        let (module, _) = ilang_syntax::parse_module(&text);
        for item in module.items {
            if matches!(&item.kind, ast::ItemKind::MacroItem { name, .. }
                if matches!(name.name.as_str(), "test" | "skip" | "only" | "fails"))
            {
                tests += 1;
                let group = if relative.starts_with("tests/review/") {
                    "tests/review"
                } else {
                    relative.split('/').next().unwrap()
                };
                *test_counts.entry(group.to_string()).or_default() += 1;
            } else if relative.contains("/test/")
                && let ast::ItemKind::Binding(b) = &item.kind
                && let ast::ExprKind::Fn(f) = &b.value.kind
                && f.params.iter().all(|p| matches!(p, ast::Param::Env { .. }))
            {
                let (line, _) = LineIndex::new(&text).line_col(b.pattern.span.start);
                zero.push(format!(
                    "{relative}:{line}\t{}",
                    &text[b.pattern.span.start as usize..b.pattern.span.end as usize]
                ));
            }
        }
        let project = path
            .ancestors()
            .skip(1)
            .take_while(|p| p.starts_with(&root))
            .find(|p| {
                p.join("i.toml").is_file()
                    && path
                        .strip_prefix(p)
                        .unwrap()
                        .components()
                        .next()
                        .is_some_and(|c| c.as_os_str() == "src" || c.as_os_str() == "test")
            });
        if let Some(project) = project {
            projects.insert(project.to_path_buf());
        } else if !relative.starts_with("lib/") {
            singles.push(path);
        }
    }
    let mut warnings = BTreeSet::new();
    let mut statuses = Vec::new();
    std::fs::write(out.join("unmarked-zero.tsv"), zero.join("\n") + "\n").unwrap();
    std::fs::write(out.join("counts.txt"), format!("test macro declarations: {tests}\nunmarked test-module functions without value parameters: {}\nprojects: {}\nstandalone files: {}\n", zero.len(), projects.len(), singles.len())).unwrap();
    std::fs::write(
        out.join("test-counts.tsv"),
        test_counts
            .into_iter()
            .map(|(path, count)| format!("{path}\t{count}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    for project in &projects {
        bounded(&root, project, None, &out, &mut warnings, &mut statuses);
    }
    for path in &singles {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"spec\"\n").unwrap();
        std::fs::create_dir(dir.path().join("test")).unwrap();
        std::fs::copy(path, dir.path().join("test/case.i")).unwrap();
        bounded(
            &root,
            dir.path(),
            Some(path),
            &out,
            &mut warnings,
            &mut statuses,
        );
    }
    std::fs::write(
        out.join("warnings.tsv"),
        warnings.into_iter().collect::<Vec<_>>().join("\n") + "\n",
    )
    .unwrap();
    std::fs::write(out.join("checks.tsv"), statuses.join("\n") + "\n").unwrap();
}

/// User plus system CPU time of a running child, from `/proc/<pid>/stat` (Linux; 100 ticks a
/// second). `None` where that is not available.
fn child_cpu_time(pid: u32) -> Option<Duration> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name is in parentheses and may hold spaces: count fields after the last ')'.
    let rest = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After the name: state is field 3, utime 14 and stime 15 (1-based), so indices 11 and 12.
    let ticks: u64 = fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
    Some(Duration::from_millis(ticks * 10))
}

fn bounded(
    root: &Path,
    project: &Path,
    single: Option<&Path>,
    out: &Path,
    warnings: &mut BTreeSet<String>,
    statuses: &mut Vec<String>,
) {
    let result = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "unused_audit_case"])
        .env("UNUSED_AUDIT_PROJECT", project)
        .env("UNUSED_AUDIT_SINGLE", single.unwrap_or(Path::new("")))
        .env("UNUSED_AUDIT_RESULT", result.path())
        .env("RUST_MIN_STACK", "33554432")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // What this bound is about: a case that loops or blows up is cut off and reported as a
    // timeout. It is a bound on the child's CPU time, not on wall time, so a busy machine does
    // not turn slow-but-fine cases into timeouts. Wall time only backs it up for a child that
    // sleeps forever, or where CPU time cannot be read.
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        let over = match child_cpu_time(child.id()) {
            Some(cpu) => {
                cpu >= Duration::from_secs(10) || start.elapsed() >= Duration::from_secs(600)
            }
            None => start.elapsed() >= Duration::from_secs(60),
        };
        if over {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if status.is_some_and(|s| s.success()) {
        warnings.extend(
            std::fs::read_to_string(result.path().join("warnings"))
                .unwrap()
                .lines()
                .map(str::to_string),
        );
        statuses.extend(
            std::fs::read_to_string(result.path().join("status"))
                .unwrap()
                .lines()
                .map(str::to_string),
        );
    } else {
        let label = single
            .unwrap_or(project)
            .strip_prefix(root)
            .unwrap()
            .display();
        statuses.push(format!(
            "{label}\t{}",
            if status.is_none() {
                "timeout (10 CPU seconds)"
            } else {
                "crash"
            }
        ));
    }
    std::fs::write(out.join("checks.tsv"), statuses.join("\n") + "\n").unwrap();
    std::fs::write(
        out.join("warnings.tsv"),
        warnings.iter().cloned().collect::<Vec<_>>().join("\n") + "\n",
    )
    .unwrap();
}

#[test]
fn unused_audit_case() {
    let Ok(project) = std::env::var("UNUSED_AUDIT_PROJECT") else {
        return;
    };
    let single = std::env::var("UNUSED_AUDIT_SINGLE").unwrap();
    let result = PathBuf::from(std::env::var("UNUSED_AUDIT_RESULT").unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let mut warnings = BTreeSet::new();
    let mut statuses = Vec::new();
    audit(
        &root,
        Path::new(&project),
        (!single.is_empty()).then(|| Path::new(&single)),
        &mut warnings,
        &mut statuses,
    );
    std::fs::write(
        result.join("warnings"),
        warnings.into_iter().collect::<Vec<_>>().join("\n"),
    )
    .unwrap();
    std::fs::write(result.join("status"), statuses.join("\n")).unwrap();
}

fn audit(
    root: &Path,
    project: &Path,
    single: Option<&Path>,
    warnings: &mut BTreeSet<String>,
    statuses: &mut Vec<String>,
) {
    let (graph, mut diags) = ilang_package::load(project);
    let checked = ilang_check::check(&graph);
    diags.extend(checked.diags);
    let label = single
        .unwrap_or(project)
        .strip_prefix(root)
        .unwrap()
        .display()
        .to_string();
    statuses.push(format!(
        "{label}\t{}",
        if diags.iter().any(Diagnostic::is_error) {
            "errors"
        } else {
            "checked"
        }
    ));
    let Some(p) = checked.program else { return };
    for d in diags.iter().filter(|d| !d.is_error()) {
        let Some(source) = d.source else { continue };
        let Some(m) = p.modules.iter().find(|m| m.source == source) else {
            continue;
        };
        let file = graph.sources.get(source);
        let path = if let Some(path) = single.filter(|_| Some(m.package) == p.root) {
            path.to_path_buf()
        } else if let Some(dir) = &graph.packages[m.package].dir {
            let relative = m.path.split_once('/').unwrap().1;
            dir.join(if m.kind == ModuleKind::Test {
                "test"
            } else {
                "src"
            })
            .join(format!("{relative}.i"))
        } else {
            root.join(if m.path == "i/prelude" {
                "lib/prelude.i".into()
            } else {
                format!("lib/{}.i", m.path)
            })
        };
        let path = path.strip_prefix(root).unwrap_or(&path);
        let (line, col) = LineIndex::new(&file.text).line_col(d.span.start);
        let class = if d.code != code::UNUSED_BINDING {
            "existing"
        } else if d.message.contains("did you forget") {
            "test-shaped"
        } else {
            "unused"
        };
        warnings.insert(format!(
            "{class}\t{}:{line}:{col}\t{}\t{}",
            path.display(),
            d.code,
            d.message
        ));
    }
}
