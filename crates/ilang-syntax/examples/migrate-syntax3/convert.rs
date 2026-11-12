//! `convert` and `--verify` of the syntax three migrator
//! (docs/design/syntax-3.md §7.2 stage 8, §7.3).
//!
//! ```text
//! migrate-syntax3 convert [--write] [--ctors DIR]... FILE...
//!     Reads each FILE (`.i`, `.is`) with the current parser and prints its
//!     syntax-three text, or with --write puts it in place of the file.
//!     Constructor labels are read from the files themselves and from every
//!     source under each --ctors DIR.
//! migrate-syntax3 --verify [DIR...]
//!     For every `.i` and `.is` file under the DIRs (default: lib std
//!     examples tests) that the current parser reads without an error:
//!     parse3(print3(convert(parse_old(f)))) == convert(parse_old(f)),
//!     the same for the text with the comments of the source, every comment
//!     is in that text, and the formatter leaves that text as it is.
//!     Prints the counts and the files that failed; exits 1 when one failed.
//! ```

use ilang_syntax::Diagnostic;
use ilang_syntax::lexer::lex;
use ilang_syntax::syn3::{self, ast::Module, convert};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn is_lisp(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "is")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.is_file() {
        out.push(dir.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | "node_modules" | ".git") {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "i" || e == "is") {
            out.push(path);
        }
    }
}

fn parse_old(path: &Path, source: &str) -> (ilang_syntax::ast::Module, Vec<Diagnostic>) {
    if is_lisp(path) {
        ilang_syntax::lisp::parse_source(source)
    } else {
        ilang_syntax::parse_module(source)
    }
}

struct Converted {
    module: Module,
    notes: Vec<convert::Note>,
    /// The text for the file, with the comments of the source.
    text: String,
    /// The number of comments in the source.
    comments: usize,
}

fn convert_file(
    path: &Path,
    source: &str,
    ctors: &convert::Signatures,
) -> Result<Converted, Vec<Diagnostic>> {
    let (old, diags) = parse_old(path, source);
    if !diags.is_empty() {
        return Err(diags);
    }
    if is_lisp(path) {
        let (module, notes) = convert::convert(&old, None, ctors);
        let text = syn3::lisp::to_lisp(&module);
        Ok(Converted {
            module,
            notes,
            text,
            comments: 0,
        })
    } else {
        let lexed = lex(source);
        let (module, notes) = convert::convert(&old, Some(&lexed), ctors);
        let text = syn3::print_with_comments(&module, source, &lexed.comments);
        Ok(Converted {
            module,
            notes,
            text,
            comments: lexed.comments.len(),
        })
    }
}

/// The checks of `--verify` on one converted file. Returns what failed.
fn check(path: &Path, c: &Converted) -> Option<String> {
    let reparse = |text: &str, what: &str| -> Option<String> {
        let (module, diags) = if is_lisp(path) {
            syn3::lisp::from_lisp(text)
        } else {
            syn3::parse_module(text)
        };
        if let Some(d) = diags.first() {
            let line = text[..d.span.start as usize].matches('\n').count() + 1;
            let shown = text.lines().nth(line - 1).unwrap_or("");
            return Some(format!(
                "{what}: the new parser rejects the text at line {line}: {}\n      {}",
                d.message,
                shown.trim()
            ));
        }
        (module != c.module)
            .then(|| format!("{what}: the tree read back is not the converted tree"))
    };
    if is_lisp(path) {
        return reparse(&c.text, "lisp");
    }
    reparse(&syn3::print_module(&c.module), "print")
        .or_else(|| reparse(&c.text, "print with comments"))
        .or_else(|| {
            let kept = ilang_syntax::lexer::lex3(&c.text).comments.len();
            (kept != c.comments).then(|| {
                format!(
                    "the source has {} comments, the new text {kept}",
                    c.comments
                )
            })
        })
        .or_else(|| match syn3::format(&c.text) {
            Ok(again) if again == c.text => None,
            Ok(again) => {
                let (a, b) = again
                    .lines()
                    .zip(c.text.lines())
                    .find(|(a, b)| a != b)
                    .unwrap_or(("", ""));
                Some(format!(
                    "the formatter changes its own text:\n      {b}\n      {a}"
                ))
            }
            Err(_) => Some("the formatter rejects the new text".into()),
        })
}

fn ctor_table(files: &[PathBuf]) -> convert::Signatures {
    let mut table = convert::Signatures::default();
    for path in files {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let (old, diags) = parse_old(path, &source);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if diags.is_empty() {
            convert::collect(&old, stem, &mut table);
        }
    }
    table
}

pub fn verify(dirs: &[String]) -> i32 {
    let dirs: Vec<String> = if dirs.is_empty() {
        ["lib", "std", "examples", "tests"]
            .map(String::from)
            .to_vec()
    } else {
        dirs.to_vec()
    };
    let mut files = Vec::new();
    for d in &dirs {
        sources(Path::new(d), &mut files);
    }
    let ctors = ctor_table(&files);
    let (mut converted, mut skipped) = (0usize, 0usize);
    let mut failed: Vec<(PathBuf, String)> = Vec::new();
    let mut kinds: BTreeMap<&'static str, (usize, usize, String)> = BTreeMap::new();
    for path in &files {
        let Ok(source) = std::fs::read_to_string(path) else {
            skipped += 1;
            continue;
        };
        let c = match convert_file(path, &source, &ctors) {
            Ok(c) => c,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let mut seen = Vec::new();
        for note in &c.notes {
            let entry = kinds
                .entry(note.kind)
                .or_insert_with(|| (0, 0, format!("{}: {}", path.display(), note.message)));
            entry.0 += 1;
            if !seen.contains(&note.kind) {
                seen.push(note.kind);
                entry.1 += 1;
            }
        }
        match check(path, &c) {
            None => converted += 1,
            Some(why) => failed.push((path.clone(), why)),
        }
    }
    for (path, why) in &failed {
        println!("FAILED {}\n    {why}", path.display());
    }
    println!("forms that need a hand or the resolver (kind: places, files, first):");
    for (kind, (places, files, first)) in &kinds {
        println!("  {kind}: {places} places in {files} files; first {first}");
    }
    println!(
        "files: {} read; {converted} converted and read back equal; {} failed; {skipped} skipped (the current parser rejects them)",
        files.len(),
        failed.len()
    );
    i32::from(!failed.is_empty())
}

pub fn convert_command(args: &[String]) -> i32 {
    let mut write = false;
    let mut ctor_dirs = Vec::new();
    let mut files = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--write" => write = true,
            "--ctors" => ctor_dirs.push(it.next().expect("--ctors needs a directory").clone()),
            _ => sources(Path::new(a), &mut files),
        }
    }
    let mut all = files.clone();
    for d in &ctor_dirs {
        sources(Path::new(d), &mut all);
    }
    let ctors = ctor_table(&all);
    let mut status = 0;
    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                status = 1;
                continue;
            }
        };
        match convert_file(path, &source, &ctors) {
            Ok(c) => {
                for note in &c.notes {
                    let line = source[..note.span.start as usize].matches('\n').count() + 1;
                    eprintln!("{}:{line}: {}: {}", path.display(), note.kind, note.message);
                }
                if write {
                    std::fs::write(path, &c.text).expect("the file can be written");
                } else {
                    print!("{}", c.text);
                }
            }
            Err(diags) => {
                for d in diags {
                    eprintln!("{}: the current parser rejects it: {d}", path.display());
                }
                status = 1;
            }
        }
    }
    status
}
