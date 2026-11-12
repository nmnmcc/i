//! The syntax-three migrator (docs/design/syntax-3.md §7.2 stage 8, §7.3).
//!
//! ```text
//! migrate-syntax3 convert [--write] [--ctors DIR]... FILE...
//!     Reads each FILE (`.i`, `.is`) with the current parser and prints its
//!     syntax-three text, or with --write puts it in place of the file.
//!     Constructor labels are read from the files themselves and from every
//!     source under each --ctors DIR.
//!     A file already in syntax three (only the new parser reads it, or both
//!     read it to the same tree) is left as it is, so the command can run
//!     again on a tree where part of the files are converted. A file that
//!     both parsers read to different trees is read as old, unless SYN3_KEEP
//!     (`:`-separated path suffixes) names it. A file that neither reads, or
//!     whose converted text does not read back, is a failure (exit 1). Files
//!     named in SYN3_FROM_OLD are read as old.
//! migrate-syntax3 snippets [--ctors DIR]...
//!     The same for length-prefixed texts on stdin (see `snippets_command`).
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
        ilang_syntax::lisp::parse_source_legacy(source)
    } else {
        ilang_syntax::legacy::parser::parse_module_legacy(source)
    }
}

fn parse_new(path: &Path, source: &str) -> (Module, Vec<Diagnostic>) {
    if is_lisp(path) {
        syn3::lisp::from_lisp(source)
    } else {
        syn3::parse_module(source)
    }
}

/// Which syntax a text is in, so that the conversion can run again on a tree
/// where part of the files are converted.
enum Reading {
    /// Only the syntax before syntax three reads it: convert it.
    Old,
    /// Only syntax three reads it, or both read it to the same tree: it is
    /// converted already.
    New,
    /// Both read it, to different trees: the tool cannot tell.
    Ambiguous,
    /// Neither reads it; the diagnostics of the old parser.
    Neither(Vec<Diagnostic>),
}

fn reading(path: &Path, source: &str, ctors: &convert::Signatures) -> Reading {
    let (old, old_diags) = parse_old(path, source);
    let (new, new_diags) = parse_new(path, source);
    match (old_diags.is_empty(), new_diags.is_empty()) {
        (true, false) => Reading::Old,
        (false, true) => Reading::New,
        (false, false) => Reading::Neither(old_diags),
        (true, true) => {
            let lexed = (!is_lisp(path)).then(|| lex(source));
            let (converted, _) = convert::convert(&old, lexed.as_ref(), ctors);
            // `x<-1` (`x < -1` before) is in no file of today's syntax
            // (§7.1 row 9), so a `<-` marks a converted text.
            let arrow = !is_lisp(path)
                && ilang_syntax::lexer::lex3(source)
                    .tokens
                    .iter()
                    .any(|t| t.kind == ilang_syntax::token::TokenKind::LArrow);
            if converted == new || arrow {
                Reading::New
            } else {
                Reading::Ambiguous
            }
        }
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
        // Laid out by the Lisp convention (`i fmt`).
        let text = ilang_syntax::lisp::format(&syn3::lisp::to_lisp(&module))
            .expect("the converted Lisp text reads back");
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
        (module != c.module).then(|| format!("{what}: the tree read back is not the converted tree"))
    };
    if is_lisp(path) {
        return reparse(&c.text, "lisp");
    }
    reparse(&syn3::print_module(&c.module), "print")
        .or_else(|| reparse(&c.text, "print with comments"))
        .or_else(|| {
            let kept = ilang_syntax::lexer::lex3(&c.text).comments.len();
            (kept != c.comments).then(|| {
                format!("the source has {} comments, the new text {kept}", c.comments)
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
                Some(format!("the formatter changes its own text:\n      {b}\n      {a}"))
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
            continue;
        }
        // A file converted already: its lowered tree has the same labels.
        let (new, diags) = parse_new(path, &source);
        if diags.is_empty() {
            let lowered = if is_lisp(path) {
                syn3::lower::lower_lisp(&new)
            } else {
                syn3::lower::lower(&new)
            };
            if lowered.1.is_empty() {
                convert::collect(&lowered.0, stem, &mut table);
            }
        }
    }
    table
}

fn verify(dirs: &[String]) -> i32 {
    let dirs: Vec<String> = if dirs.is_empty() {
        ["lib", "std", "examples", "tests"].map(String::from).to_vec()
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

fn convert_command(args: &[String]) -> i32 {
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
    let force: Vec<PathBuf> = std::env::var("SYN3_FROM_OLD")
        .map(|v| v.split(':').filter(|p| !p.is_empty()).map(PathBuf::from).collect())
        .unwrap_or_default();
    let keep: Vec<PathBuf> = std::env::var("SYN3_KEEP")
        .map(|v| v.split(':').filter(|p| !p.is_empty()).map(PathBuf::from).collect())
        .unwrap_or_default();
    let (mut converted, mut already, mut failed) = (0usize, 0usize, 0usize);
    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                status = 1;
                continue;
            }
        };
        let forced = force.iter().any(|f| path.ends_with(f));
        match if forced { Reading::Old } else { reading(path, &source, &ctors) } {
            Reading::New => {
                already += 1;
                // Formatted, so that the layout is the formatter's everywhere.
                let formatted = if is_lisp(path) {
                    ilang_syntax::lisp::format(&source)
                } else {
                    syn3::format(&source)
                };
                // A text with carriage returns keeps them (fixtures of the
                // line rule): it is left as it is.
                let text = match formatted {
                    Ok(text) if !source.contains('\r') => text,
                    _ => source.clone(),
                };
                if !write {
                    print!("{text}");
                } else if text != source {
                    std::fs::write(path, &text).expect("the file can be written");
                }
                continue;
            }
            Reading::Ambiguous if keep.iter().any(|f| path.ends_with(f)) => {
                already += 1;
                if !write {
                    print!("{source}");
                }
                continue;
            }
            // Both read it to different trees: the tree being converted is
            // in the syntax before syntax three (the conversion is re-run on
            // a newer tree of today's syntax), so the old reading wins.
            Reading::Ambiguous => {
                eprintln!(
                    "{}:1: ambiguous: both syntaxes read it, to different trees; read as the old syntax (SYN3_KEEP names files to leave)",
                    path.display()
                );
            }
            Reading::Neither(diags) => {
                for d in diags {
                    let line = source[..(d.span.start as usize).min(source.len())]
                        .matches('\n')
                        .count()
                        + 1;
                    eprintln!(
                        "{}:{line}: FAILED: neither syntax reads it: {}",
                        path.display(),
                        d.message
                    );
                }
                failed += 1;
                status = 1;
                continue;
            }
            Reading::Old => {}
        }
        match convert_file(path, &source, &ctors) {
            Ok(c) => {
                for note in &c.notes {
                    let line = source[..note.span.start as usize].matches('\n').count() + 1;
                    eprintln!("{}:{line}: {}: {}", path.display(), note.kind, note.message);
                }
                if let Some(why) = check(path, &c) {
                    eprintln!("{}: FAILED: {why}", path.display());
                    failed += 1;
                    status = 1;
                    continue;
                }
                converted += 1;
                if write {
                    std::fs::write(path, &c.text).expect("the file can be written");
                } else {
                    print!("{}", c.text);
                }
            }
            Err(diags) => {
                for d in diags {
                    eprintln!("{}: FAILED: the current parser rejects it: {d}", path.display());
                }
                failed += 1;
                status = 1;
            }
        }
    }
    eprintln!(
        "migrate-syntax3: {} files: {converted} converted, {already} in syntax three already, {failed} failed",
        files.len()
    );
    status
}

/// `snippets [--ctors DIR]...`: reads length-prefixed normal-surface texts on
/// stdin (the protocol of `parse-snippets`) and answers each with one line,
/// `old <hex>` (the converted text), `new`, `failed <hex>` or `neither <hex>`
/// (the first diagnostic of the old parser).
fn snippets_command(args: &[String]) -> i32 {
    use std::io::{BufRead, Read, Write};
    let mut all = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--ctors" {
            sources(Path::new(it.next().expect("--ctors needs a directory")), &mut all);
        }
    }
    let ctors = ctor_table(&all);
    let hex = |s: &str| s.bytes().map(|b| format!("{b:02x}")).collect::<String>();
    let mut input = std::io::stdin().lock();
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    let mut line = String::new();
    let path = Path::new("snippet.i");
    while input.read_line(&mut line).unwrap() != 0 {
        let len: usize = line.trim().parse().unwrap();
        let mut source = vec![0; len];
        input.read_exact(&mut source).unwrap();
        let source = String::from_utf8(source).unwrap();
        let answer = match reading(path, &source, &ctors) {
            Reading::New => "new".to_string(),
            Reading::Neither(d) => format!("neither {}", hex(&d[0].message)),
            Reading::Old | Reading::Ambiguous => match convert_file(path, &source, &ctors) {
                Ok(c) => match check(path, &c) {
                    None => format!("old {}", hex(&c.text)),
                    Some(why) => format!("failed {}", hex(&why)),
                },
                Err(d) => format!("neither {}", hex(&d[0].message)),
            },
        };
        writeln!(output, "{answer}").unwrap();
        line.clear();
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = match args.first().map(String::as_str) {
        Some("convert") => convert_command(&args[1..]),
        Some("--verify") => verify(&args[1..]),
        Some("snippets") => snippets_command(&args[1..]),
        _ => {
            eprintln!(
                "usage: migrate-syntax3 convert [--write] [--ctors DIR]... FILE...\n       migrate-syntax3 --verify [DIR...]"
            );
            2
        }
    };
    std::process::exit(status);
}
