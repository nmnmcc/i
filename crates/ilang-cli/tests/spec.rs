//! The spec test suite: every case in `tests/spec/` is compiled at `--opt 0`
//! and `--opt 2`, run, and checked against its header. Both levels must give
//! the same outcome, with the same stop messages.
//!
//! A case is `tests/spec/<section>/<case>.i`, a module of a package named
//! `spec` (so its path is `spec/<case>`), or a directory
//! `tests/spec/<section>/<case>/` with an `i.toml`. The header is at the top
//! of the `.i` file, or of the `i.toml` with `#` in place of `//`:
//!
//! ```text
//! // spec: 6.10
//! // expect: pass | compile-error <code> | stop "<first line>" | warning <code>
//! # entry: name                (a package case selects an [entries] target)
//! # entry-test: name           (same, only when building tests)
//! // at: spec/case:3:5          (for a stop: each further line of the message)
//! // output: text               (each text that the entry gave to a host function)
//! ```
//!
//! The host gives the services and external functions of
//! `ilang_host::services::testing` under `test:` namespaces, and, as by
//! default, does not check their answers, so that the module stops for an
//! answer that is not correct (B.2). With `output` lines, the texts that the entry gave to
//! `test:io/v1` `print` and to the functions of `test:async/v1` are exactly
//! those, in order.
//!
//! - `pass`: the entry, when there is one, ends without a stop, and every
//!   `@test` passes.
//! - `warning <code>`: as `pass`, and the compiler gives that warning.
//! - `stop "…"`: the entry stops, and the first line of the stop message is
//!   the text. With `at` lines, the other lines of the message are exactly
//!   those.
//! - `compile-error <code>`: the compiler gives that error.
//!
//! `cargo test -p ilang-cli --test spec -- <filter>` runs the cases whose
//! path has the filter.

use ilang_cli::{BuildError, BuildOptions, Level, build_entry, run_test_with};
use ilang_host::services::{Log, testing};
use ilang_host::{HostOptions, Outcome, Program, TestResult};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[path = "common/single_file.rs"]
mod single_file;

#[path = "common/spec_header.rs"]
mod spec_header;

#[path = "common/spec_entry.rs"]
mod spec_entry;

#[derive(Debug, Clone, PartialEq)]
enum Expect {
    Pass,
    Warning(String),
    Stop(String, Vec<String>),
    CompileError(String),
}

struct Case {
    name: String,
    header_path: PathBuf,
    comment: &'static str,
    /// The package directory to build.
    project: PathBuf,
    expect: Expect,
    /// The texts that the entry gives to the host, when they are checked.
    output: Option<Vec<String>>,
    _temp: Option<tempfile::TempDir>,
}

fn header(text: &str, comment: &str) -> Result<(Expect, Option<Vec<String>>), String> {
    let mut expect = None;
    let mut at = Vec::new();
    let mut output: Option<Vec<String>> = None;
    let mut spec = false;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(comment) else {
            if line.trim().is_empty() {
                continue;
            }
            break;
        };
        let rest = rest.trim();
        if let Some(s) = rest.strip_prefix("spec:") {
            spec = !s.trim().is_empty();
        } else if let Some(e) = rest.strip_prefix("expect:") {
            let e = e.trim();
            expect = Some(if e == "pass" {
                Expect::Pass
            } else if let Some(c) = e.strip_prefix("compile-error ") {
                Expect::CompileError(c.trim().to_string())
            } else if let Some(c) = e.strip_prefix("warning ") {
                Expect::Warning(c.trim().to_string())
            } else if let Some(m) = e.strip_prefix("stop ") {
                let m = m.trim();
                let m = m
                    .strip_prefix('"')
                    .and_then(|m| m.strip_suffix('"'))
                    .ok_or("a stop message is in quotes")?;
                Expect::Stop(m.to_string(), Vec::new())
            } else {
                return Err(format!("unknown expectation `{e}`"));
            });
        } else if let Some(a) = rest.strip_prefix("at:") {
            at.push(a.trim().to_string());
        } else if let Some(o) = rest.strip_prefix("output:") {
            output
                .get_or_insert_with(Vec::new)
                .push(o.trim().to_string());
        }
    }
    if !spec {
        return Err("the header has no `spec:` line".into());
    }
    match expect {
        Some(Expect::Stop(m, _)) => Ok((Expect::Stop(m, at), output)),
        Some(e) => Ok((e, output)),
        None => Err("the header has no `expect:` line".into()),
    }
}

fn cases(root: &Path) -> Vec<Result<Case, String>> {
    let mut out = Vec::new();
    let mut sections: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("tests/spec")
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    sections.sort();
    for section in sections {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&section)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                let text = std::fs::read_to_string(path.join("i.toml")).unwrap_or_default();
                out.push(header(&text, "#").map_err(|e| format!("{name}: {e}")).map(
                    |(expect, output)| Case {
                        name,
                        header_path: path.join("i.toml"),
                        comment: "#",
                        project: path.clone(),
                        expect,
                        output,
                        _temp: None,
                    },
                ));
            } else if path.extension().is_some_and(|e| e == "i") {
                let text = std::fs::read_to_string(&path).unwrap();
                let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                out.push(header(&text, "//").map_err(|e| format!("{name}: {e}")).map(
                    |(mut expect, output)| {
                        if let Expect::Stop(_, positions) = &mut expect {
                            let old = format!("spec/{stem}:");
                            for position in positions {
                                if let Some(suffix) = position.strip_prefix(&old) {
                                    *position = format!("spec/main:{suffix}");
                                }
                            }
                        }
                        let dir = tempfile::tempdir().unwrap();
                        std::fs::write(dir.path().join("i.toml"), "name = \"spec\"\n").unwrap();
                        single_file::write(dir.path(), &stem, &text);
                        Case {
                            name,
                            header_path: path.clone(),
                            comment: "//",
                            project: dir.path().to_path_buf(),
                            expect,
                            output,
                            _temp: Some(dir),
                        }
                    },
                ));
            }
        }
    }
    out
}

/// What a build and its runs gave, in a form that two levels can compare.
#[derive(Debug, PartialEq)]
enum Outcome2 {
    CompileError {
        codes: Vec<String>,
        text: String,
    },
    Ran {
        warnings: Vec<String>,
        /// `None` when there is no entry; else `Ok` or the stop message.
        main: Option<Result<(), String>>,
        /// What the entry gave to the host.
        output: Vec<String>,
        tests: Vec<(String, Result<(), String>)>,
    },
}

fn stop_text(stop: &ilang_host::Stop) -> String {
    match &stop.message {
        Some(m) => m.clone(),
        None => format!("trap without a stop message: {}", stop.trap),
    }
}

fn run(project: &Path, opt: Level) -> Result<Outcome2, String> {
    let entry = spec_entry::named_entry(project, true);
    let built = match build_entry(
        project,
        BuildOptions {
            tests: true,
            opt,
            offline: false,
            target: Default::default(),
        },
        entry.as_deref(),
    ) {
        Ok(b) => b,
        Err(BuildError::Program { lines, codes, .. }) => {
            return Ok(Outcome2::CompileError {
                codes: codes.iter().map(|c| c.to_string()).collect(),
                text: lines.join("\n"),
            });
        }
        Err(e) => return Err(e.to_string()),
    };
    let program = Program::from_bytes(&built.wasm).map_err(|e| format!("{e:#}"))?;
    let options = |log: &Log| HostOptions::new(testing(log.clone()));
    let log = Log::default();
    let main = match &program.abi().entry {
        None => None,
        Some(_) => Some(match program.run_entry(options(&log)) {
            Ok(Outcome::Ok(_)) => Ok(()),
            Ok(Outcome::Stop(stop)) => Err(stop_text(&stop)),
            Ok(Outcome::Err(e)) => Err(format!("error {:?}", e.0)),
            Err(e) => return Err(format!("{e:#}")),
        }),
    };
    let mut tests = Vec::new();
    for t in &program.abi().tests {
        let r = match run_test_with(&program, &t.name, options(&Log::default())) {
            TestResult::Passed => Ok(()),
            TestResult::Stopped(stop) => Err(stop_text(&stop)),
            TestResult::Failed(e) => return Err(format!("{}: {e:#}", t.name)),
        };
        tests.push((t.name.clone(), r));
    }
    let output = log.lock().unwrap().clone();
    Ok(Outcome2::Ran {
        warnings: built.warnings,
        main,
        output,
        tests,
    })
}

fn check(case: &Case, o: &Outcome2, expect: &Expect) -> Result<(), String> {
    if let (Some(want), Outcome2::Ran { output, .. }) = (&case.output, o)
        && want != output
    {
        return Err(format!("expected the output {want:?}, got {output:?}"));
    }
    match (expect, o) {
        (Expect::CompileError(code), Outcome2::CompileError { codes, text }) => {
            if codes.iter().any(|c| c == code) {
                Ok(())
            } else {
                Err(format!("expected the error {code}, got:\n{text}"))
            }
        }
        (Expect::CompileError(code), Outcome2::Ran { .. }) => {
            Err(format!("expected the error {code}, but it compiled"))
        }
        (_, Outcome2::CompileError { text, .. }) => Err(format!("compile error:\n{text}")),
        (
            Expect::Pass | Expect::Warning(_),
            Outcome2::Ran {
                warnings,
                main,
                tests,
                ..
            },
        ) => {
            if let Expect::Warning(code) = expect
                && !warnings
                    .iter()
                    .any(|w| w.contains(&format!("warning[{code}]")))
            {
                return Err(format!("expected the warning {code}, got {warnings:?}"));
            }
            if let Some(Err(m)) = main {
                return Err(format!("the entry stopped:\n{m}"));
            }
            if main.is_none() && tests.is_empty() {
                return Err("the case has no entry and no @test".into());
            }
            for (name, r) in tests {
                if let Err(m) = r {
                    return Err(format!("{name} failed:\n{m}"));
                }
            }
            Ok(())
        }
        (Expect::Stop(first, at), Outcome2::Ran { main, .. }) => match main {
            None => Err("a stop case needs an entry".into()),
            Some(Ok(())) => Err(format!("expected the stop `{first}`, but the entry ended")),
            Some(Err(m)) => {
                let mut lines = m.split('\n');
                let got = lines.next().unwrap_or("");
                if got != first {
                    return Err(format!("expected the stop `{first}`, got:\n{m}"));
                }
                let rest: Vec<&str> = lines.collect();
                if !at.is_empty() && rest != *at {
                    return Err(format!("expected the positions {at:?}, got:\n{m}"));
                }
                Ok(())
            }
        },
    }
}

fn main() {
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/spec");
    let all = cases(&root);
    let mut failures: Vec<String> = Vec::new();
    let mut ready = Vec::new();
    for c in all {
        match c {
            Ok(c) => {
                if filter.as_ref().is_none_or(|f| c.name.contains(f.as_str())) {
                    ready.push(c);
                }
            }
            Err(e) => failures.push(e),
        }
    }
    let total = ready.len();
    let results = Mutex::new(Vec::new());
    let next = Mutex::new(0usize);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        let i = *n;
                        *n += 1;
                        i
                    };
                    let Some(case) = ready.get(i) else { break };
                    let r = run_case(case);
                    results.lock().unwrap().push((case.name.clone(), r));
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    let mut passed = 0;
    for (name, r) in results {
        match r {
            Ok(()) => {
                passed += 1;
                println!("ok   {name}");
            }
            Err(e) => {
                println!("FAIL {name}");
                failures.push(format!("{name}: {e}"));
            }
        }
    }
    println!("\nspec: {passed} of {total} cases passed at --opt 0 and --opt 2");
    if !failures.is_empty() {
        println!("\nfailures:");
        for f in &failures {
            println!("\n{f}");
        }
        std::process::exit(1);
    }
}

fn run_case(case: &Case) -> Result<(), String> {
    let o0 = run(&case.project, Level::O0).map_err(|e| format!("--opt 0: {e}"))?;
    let o2 = run(&case.project, Level::O2).map_err(|e| format!("--opt 2: {e}"))?;
    if o0 != o2 {
        return Err(format!("--opt 0 and --opt 2 differ:\n{o0:#?}\n{o2:#?}"));
    }
    let mut expect = case.expect.clone();
    let mut update = None;
    if std::env::var("UPDATE").as_deref() == Ok("1")
        && let Expect::Stop(_, old_at) = &expect
        && let Outcome2::Ran {
            main: Some(Err(message)),
            ..
        } = &o0
    {
        let mut lines = message.split('\n');
        let first = lines.next().unwrap_or("");
        // An absent at list deliberately does not check the call frames.
        let at: Vec<String> = if old_at.is_empty() {
            Vec::new()
        } else {
            lines.map(str::to_string).collect()
        };
        let text = std::fs::read_to_string(&case.header_path).map_err(|e| e.to_string())?;
        let updated = spec_header::rewrite(&text, case.comment, first, &at)?;
        if updated != text {
            update = Some(updated);
        }
        expect = Expect::Stop(first.to_string(), at);
    }
    check(case, &o0, &expect).map_err(|e| format!("--opt 0: {e}"))?;
    check(case, &o2, &expect).map_err(|e| format!("--opt 2: {e}"))?;
    if let Some(updated) = update {
        std::fs::write(&case.header_path, updated).map_err(|e| e.to_string())?;
    }
    Ok(())
}
