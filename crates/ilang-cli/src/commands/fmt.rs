//! `i fmt [--check] [PATH…]`: formats source files with
//! `ilang_syntax::format`.
//!
//! With no paths, every `.i` file under the `src/` and `test/` directories of
//! the package is formatted. A path may be a file or a directory, which is
//! searched for `.i` files. A file with syntax errors is left as it is, and
//! its errors are printed. With `--check`, no file is written; the files that
//! would change are listed, and the exit code is 1 when there are any.

use ilang_syntax::span::LineIndex;
use ilang_syntax::{format::format_migration, migrate};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub struct Options {
    pub from_old: bool,
    pub keep_layout: bool,
    pub report: bool,
    pub choices_split: bool,
}

pub fn run(
    project: &Path,
    check: bool,
    paths: &[PathBuf],
    options: Options,
) -> anyhow::Result<ExitCode> {
    let mut files = Vec::new();
    if paths.is_empty() {
        let roots = [project.join("src"), project.join("test")];
        if !roots.iter().any(|r| r.is_dir()) {
            anyhow::bail!("{} has no `src/` or `test/` directory", project.display());
        }
        for root in roots {
            collect(&root, &mut files)?;
        }
    } else {
        for path in paths {
            if path.is_dir() {
                collect(path, &mut files)?;
            } else if path.is_file() {
                files.push(path.clone());
            } else {
                anyhow::bail!("{} is not a file or a directory", path.display());
            }
        }
    }
    files.sort();
    files.dedup();
    let mut failed = false;
    for file in &files {
        let source = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", file.display()))?;
        let module_name = module_path(file);
        let result = if options.keep_layout {
            let (module, _) = ilang_syntax::parse_module_old(&source);
            let mut env = migrate::Env::new(&module);
            env.module = module_name.clone();
            env.choices_split = options.choices_split;
            let plan = migrate::plan(&module, &env);
            migrate::splice(&source, &plan).map(|out| (out.text, plan))
        } else {
            format_migration(
                &source,
                options.from_old,
                &module_name,
                options.choices_split,
            )
        };
        if options.report {
            let parse = if options.from_old || options.keep_layout {
                ilang_syntax::parse_module_old
            } else {
                ilang_syntax::parse_module
            };
            let (module, _) = parse(&source);
            let mut env = migrate::Env::new(&module);
            env.module = module_name;
            env.choices_split = options.choices_split;
            let plan = migrate::plan(&module, &env);
            let output = result.as_ref().map(|(out, _)| out.as_str());
            for line in migrate::report(
                &module,
                &plan,
                &source,
                output.ok(),
                &file.display().to_string(),
            ) {
                eprintln!("{line}");
            }
        }
        match result.map(|(out, _)| out) {
            Ok(out) if out == source => {}
            Ok(out) => {
                if check {
                    println!("{}", file.display());
                    failed = true;
                } else {
                    std::fs::write(file, out)
                        .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", file.display()))?;
                }
            }
            Err(diags) => {
                failed = true;
                let lines = LineIndex::new(&source);
                for d in diags {
                    let (line, col) = lines.line_col(d.span.start);
                    eprintln!(
                        "{}:{line}:{col}: error[{}]: {}",
                        file.display(),
                        d.code.as_str(),
                        d.message
                    );
                }
            }
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// The `.i` files under `dir`, if it is there.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "i") {
            out.push(path);
        }
    }
    Ok(())
}

/// Use the package's own module path, as the WAT owner comments do.
fn module_path(file: &Path) -> String {
    let absolute = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    for parent in absolute.ancestors().skip(1) {
        let manifest = parent.join("i.toml");
        if let Ok(text) = std::fs::read_to_string(manifest) {
            let (manifest, _) = ilang_package::manifest::parse_manifest(&text);
            if let Some(manifest) = manifest {
                for root in ["src", "test"] {
                    if let Ok(path) = absolute.strip_prefix(parent.join(root)) {
                        return format!("{}/{}", manifest.name, path.with_extension("").display());
                    }
                }
            }
        }
    }
    if absolute
        .to_string_lossy()
        .contains("/crates/ilang-check/tests/types/")
    {
        return "app/main".into();
    }
    if let Some(parent) = absolute.parent()
        && parent.file_name().is_some_and(|n| n == "i")
    {
        return format!("i/{}", absolute.file_stem().unwrap().to_string_lossy());
    }
    if absolute.file_stem().is_some_and(|n| n == "prelude") {
        return "prelude".into();
    }
    format!(
        "spec/{}",
        absolute.file_stem().unwrap_or_default().to_string_lossy()
    )
}
