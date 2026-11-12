//! The modules of a package: the `.i` files under `src/` and `test/`.

use crate::{Diagnostic, Loader, ModuleKind, ModuleSource, code, is_name};
use ilang_syntax::span::Span;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Every `.i` file under `dir/src`, and under `dir/test` when `tests` is set,
/// in the order of their paths. With `shown`, the files are shown as
/// `shown/` and their path in `dir`; otherwise relative to the directory of
/// the compiled package.
pub(crate) fn collect(
    loader: &mut Loader,
    package: &str,
    dir: &Path,
    tests: bool,
    shown: Option<&str>,
) -> (Vec<ModuleSource>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut out: Vec<ModuleSource> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let roots: &[(&str, ModuleKind)] = if tests {
        &[("src", ModuleKind::Src), ("test", ModuleKind::Test)]
    } else {
        &[("src", ModuleKind::Src)]
    };
    for &(sub, kind) in roots {
        let base = dir.join(sub);
        let mut files = Vec::new();
        walk(&base, &mut files);
        files.sort();
        for file in files {
            let display = match shown {
                Some(s) => format!("{s}/{}", crate::relative(dir, &file)),
                None => loader.display(&file),
            };
            let text = match std::fs::read(&file).map(String::from_utf8) {
                Ok(Ok(text)) => text,
                _ => {
                    diags.push(Diagnostic::error(
                        code::UNREADABLE,
                        None,
                        Span::default(),
                        format!("`{display}` could not be read as UTF-8 text"),
                    ));
                    continue;
                }
            };
            let source = loader.graph.sources.add(display.clone(), text);
            let rel = file.strip_prefix(&base).unwrap_or(&file).with_extension("");
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            if let Some(bad) = parts.iter().find(|p| !is_name(p)) {
                diags.push(Diagnostic::error(
                    code::MODULE_PATH_NAME,
                    Some(source),
                    Span::default(),
                    format!("every part of a module path must be a name, but `{bad}` is not"),
                ));
                continue;
            }
            let path = format!("{package}/{}", parts.join("/"));
            if let Some(&first) = seen.get(&path) {
                let other = loader.graph.sources.get(out[first].source).display.clone();
                diags.push(Diagnostic::error(
                    code::MODULE_TWICE,
                    Some(source),
                    Span::default(),
                    format!("the module path `{path}` is also the path of `{other}`"),
                ));
                continue;
            }
            seen.insert(path.clone(), out.len());
            out.push(ModuleSource { path, source, kind });
        }
    }
    (out, diags)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "i" || e == "is") {
            out.push(path);
        }
    }
}
