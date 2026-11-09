//! `i fmt [--check] [PATH…]`: formats source files with
//! `ilang_syntax::format`.
//!
//! With no paths, every `.i` file under the `src/` and `test/` directories of
//! the package is formatted. A path may be a file or a directory, which is
//! searched for `.i` files. A file with syntax errors is left as it is, and
//! its errors are printed. With `--check`, no file is written; the files that
//! would change are listed, and the exit code is 1 when there are any.

use ilang_syntax::format::format;
use ilang_syntax::span::LineIndex;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(project: &Path, check: bool, paths: &[PathBuf]) -> anyhow::Result<ExitCode> {
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
        match format(&source) {
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
