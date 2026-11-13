//! `i fmt [--check] [PATH…]`: formats source files with
//! `ilang_syntax::format`.
//!
//! With no paths, every `.i` and `.is` file under the `src/` and `test/` directories of
//! the package is formatted. A path may be a file or a directory, which is
//! searched for `.i` and `.is` files. A file with syntax errors is left as it is, and
//! its errors are printed. With `--check`, no file is written; the files that
//! would change are listed, and the exit code is 1 when there are any.

use ilang_syntax::format::format;
use ilang_syntax::span::LineIndex;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(
    project: &Path,
    check: bool,
    to: Option<&str>,
    paths: &[PathBuf],
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
        let is_lisp = file.extension().is_some_and(|e| e == "is");
        let destination = to.map(|target| file.with_extension(target));
        let converting = destination.as_ref().is_some_and(|dest| dest != file);
        let result = if converting && is_lisp {
            convert_from_lisp(&source)
        } else if converting {
            convert_to_lisp(&source)
        } else if is_lisp {
            ilang_syntax::lisp::format(&source).map(|out| (out, Vec::new()))
        } else {
            format(&source).map(|out| (out, Vec::new()))
        };
        match result {
            Ok((out, _)) if !converting && out == source => {}
            Ok((out, lost)) => {
                let lines = LineIndex::new(&source);
                for span in lost {
                    let (line, col) = lines.line_col(span.start);
                    eprintln!(
                        "{}:{line}:{col}: comment could not be preserved",
                        file.display()
                    );
                }
                if check {
                    println!("{}", file.display());
                    failed = true;
                } else if let Some(dest) = destination.as_ref().filter(|_| converting) {
                    let mut output = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(dest)
                        .map_err(|e| anyhow::anyhow!("cannot create {}: {e}", dest.display()))?;
                    if let Err(e) = output.write_all(out.as_bytes()) {
                        drop(output);
                        std::fs::remove_file(dest)?;
                        return Err(anyhow::anyhow!("cannot write {}: {e}", dest.display()));
                    }
                    drop(output);
                    std::fs::remove_file(file)
                        .map_err(|e| anyhow::anyhow!("cannot remove {}: {e}", file.display()))?;
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

/// The `.i` and `.is` files under `dir`, if it is there.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "i" || e == "is") {
            out.push(path);
        }
    }
    Ok(())
}

/// The other surface of a file, read and written as the tree of syntax
/// three. The errors of the lowering are reported as `i check` reports them.
fn convert_to_lisp(
    source: &str,
) -> Result<(String, Vec<ilang_syntax::span::Span>), Vec<ilang_syntax::Diagnostic>> {
    let (_, errors) = ilang_syntax::parse_module(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (out, lost) = ilang_syntax::syn3::surface::normal_to_lisp(source)?;
    Ok((ilang_syntax::lisp::format(&out)?, lost))
}

fn convert_from_lisp(
    source: &str,
) -> Result<(String, Vec<ilang_syntax::span::Span>), Vec<ilang_syntax::Diagnostic>> {
    let (_, errors) = ilang_syntax::lisp::parse_source(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (out, lost) = ilang_syntax::syn3::surface::lisp_to_normal(source)?;
    Ok((format(&out)?, lost))
}
