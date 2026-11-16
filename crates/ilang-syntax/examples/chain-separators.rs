//! Migration helper: report the byte offset of every value-level chain
//! separator in a whole-file source, as the current parser reads it.
//!
//! The tree says where the chains are: only `ExprKind::Chain` nodes are
//! value-level chains, so `|` in types (`TypeKind::Union`), in pattern
//! alternatives (`PItem::choices`) and the `|>` pipe are never seen here. The
//! token stream then picks the separator itself: between two branches the only
//! token is the separator `|` (or `&`, once the parser reads `&` chains), and
//! in the Lisp surface the head before the first branch. Anything else in that
//! gap is reported as a refusal.
//!
//! Protocol, on stdin. Two kinds of request, one per line:
//!
//! * `<normal|lisp> <path>` — a whole file; the path is the rest of the line.
//! * `<normal|lisp> #<len>` — a source text, whose exactly `<len>` bytes follow
//!   the line; used for sources embedded in Rust string literals, JSON and
//!   Markdown fences, which are not files.
//!
//! On stdout, per request:
//!
//! ```text
//! FILE <path>         (or SNIPPET <index> for a text request)
//! SEP <offset>        one line per separator, ascending
//! DIAG <code> <hex>   one line per parse diagnostic (message hex-encoded)
//! AST <u64>           a fingerprint of the parsed tree
//! END
//! ```
//!
//! `FAIL <path> <message>` replaces the group when the request itself is
//! wrong. Exit status 0 unless a request line is malformed.
//!
//! A text that does not parse still gets its `DIAG` lines and the separators
//! the recovered tree still shows; the caller decides whether to trust them.
//! A `FAIL` means the source could not be read at all (a file that is not
//! UTF-8 or does not exist), and then no separator is reported.

use std::io::{self, BufRead, Read, Write};

use ilang_syntax::ast::{Expr, ExprKind, Module};
use ilang_syntax::span::Span;
use ilang_syntax::token::TokenKind as T;
use ilang_syntax::walk;

#[derive(Clone, Copy, PartialEq)]
enum Surface {
    Normal,
    Lisp,
}

struct Chains {
    /// `(whole chain span, branch spans)` in source order.
    chains: Vec<(Span, Vec<Span>)>,
}

impl walk::Visitor for Chains {
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if enter && let ExprKind::Chain(fs) = &e.kind {
            self.chains
                .push((e.span.0, fs.iter().map(|f| f.span.0).collect()));
        }
    }
}

/// The separators of one chain: the `|` token in each gap between branches
/// (normal surface) or the `|` head before the first branch (Lisp surface).
fn separators(
    tokens: &[ilang_syntax::token::Token],
    surface: Surface,
    chain: Span,
    branches: &[Span],
) -> Result<Vec<u32>, String> {
    let mut gaps: Vec<(u32, u32)> = Vec::new();
    match surface {
        Surface::Normal => {
            for pair in branches.windows(2) {
                if pair[0].end < pair[1].start {
                    gaps.push((pair[0].end, pair[1].start));
                }
            }
        }
        Surface::Lisp => {
            // The head sits between the chain's own `(` and its first branch.
            // `(-> F)` is a one-branch chain with no head; its gap is empty.
            if let Some(first) = branches.first()
                && chain.start + 1 < first.start
            {
                gaps.push((chain.start + 1, first.start));
            }
        }
    }
    let mut found = Vec::new();
    for (from, to) in gaps {
        if to < from {
            return Err(format!(
                "branches out of order in the chain at {from}..{to}"
            ));
        }
        let inside: Vec<&ilang_syntax::token::Token> = tokens
            .iter()
            .filter(|t| t.span.start >= from && t.span.end <= to)
            .collect();
        match inside.as_slice() {
            [only] if matches!(only.kind, T::Pipe | T::Amp) => found.push(only.span.start),
            [only] => {
                return Err(format!(
                    "expected a `|` or `&` separator at {from}..{to}, found {:?}",
                    only.kind
                ));
            }
            other => {
                return Err(format!(
                    "expected one `|` or `&` separator at {from}..{to}, found {} tokens",
                    other.len()
                ));
            }
        }
    }
    Ok(found)
}

fn fingerprint(module: &Module) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{module:?}").hash(&mut hasher);
    hasher.finish()
}

fn run(surface: Surface, source: &str) -> (Vec<u32>, Vec<(String, String)>, u64) {
    let (module, diags, lexed) = match surface {
        Surface::Normal => {
            let (module, diags) = ilang_syntax::syn3::parse_module(source);
            let lexed = ilang_syntax::lex(source);
            (module, diags, lexed)
        }
        Surface::Lisp => {
            let (module, diags) = ilang_syntax::syn3::lisp::from_lisp(source);
            let lexed = ilang_syntax::lexer::lex_lisp(source);
            (module, diags, lexed)
        }
    };
    let mut collector = Chains { chains: Vec::new() };
    let mut module = module;
    walk::module(&mut module, &mut collector);
    let mut offsets = Vec::new();
    let mut problems = Vec::new();
    for (chain, branches) in collector.chains {
        match separators(&lexed.tokens, surface, chain, &branches) {
            Ok(mut found) => offsets.append(&mut found),
            Err(message) => problems.push((String::from("E0000"), message)),
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    let diags = diags
        .into_iter()
        .map(|d| (d.code.as_str().to_string(), d.message))
        .chain(problems)
        .collect();
    (offsets, diags, fingerprint(&module))
}

fn report(output: &mut impl Write, name: &str, source: &str, surface: Surface) {
    let (offsets, diags, ast) = run(surface, source);
    writeln!(output, "{name}").unwrap();
    for offset in offsets {
        writeln!(output, "SEP {offset}").unwrap();
    }
    for (code, message) in diags {
        let hex: String = message.bytes().map(|b| format!("{b:02x}")).collect();
        writeln!(output, "DIAG {code} {hex}").unwrap();
    }
    writeln!(output, "AST {ast}").unwrap();
    writeln!(output, "END").unwrap();
}

fn main() {
    let mut input = io::stdin().lock();
    let mut output = io::BufWriter::new(io::stdout().lock());
    let mut line = Vec::new();
    let mut bad = false;
    let mut snippet = 0usize;
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
            break;
        }
        let request = String::from_utf8_lossy(&line)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if request.is_empty() {
            continue;
        }
        let Some((surface, target)) = request.split_once(' ') else {
            writeln!(output, "FAIL {request} a request is `<normal|lisp> <path>`").unwrap();
            bad = true;
            continue;
        };
        let surface = match surface {
            "normal" => Surface::Normal,
            "lisp" => Surface::Lisp,
            other => {
                writeln!(output, "FAIL {target} unknown surface {other}").unwrap();
                bad = true;
                continue;
            }
        };
        if let Some(length) = target.strip_prefix('#') {
            let length: usize = match length.parse() {
                Ok(length) => length,
                Err(_) => {
                    writeln!(output, "FAIL {target} a length is `#<bytes>`").unwrap();
                    bad = true;
                    continue;
                }
            };
            let mut source = vec![0u8; length];
            if let Err(e) = input.read_exact(&mut source) {
                writeln!(output, "FAIL #{length} {e}").unwrap();
                bad = true;
                break;
            }
            let name = format!("SNIPPET {snippet}");
            snippet += 1;
            match std::str::from_utf8(&source) {
                Ok(source) => report(&mut output, &name, source, surface),
                Err(e) => {
                    writeln!(output, "FAIL {name} not UTF-8: {e}").unwrap();
                }
            }
            continue;
        }
        match std::fs::read_to_string(target) {
            Ok(source) => report(&mut output, &format!("FILE {target}"), &source, surface),
            Err(e) => {
                writeln!(output, "FAIL {target} {e}").unwrap();
            }
        }
    }
    output.flush().unwrap();
    if bad {
        std::process::exit(2);
    }
}
