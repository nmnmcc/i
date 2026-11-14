//! Syntax three, stage 5 (`docs/design/syntax-3.md` §7.2): in today's syntax, group
//! every `a |> f OP b` as `{ a |> f } OP b` and every `a OP b |> f` as
//! `a OP { b |> f }`, where OP is at precedence levels 1 to 5. Afterwards `|>` can
//! move to the weakest level without changing any program.
//!
//! The edits are pure insertions of `{ ` and ` }` at expression spans; the text between
//! them is untouched. A group that is already written is a `Block` node and is not
//! touched again, so the tool is idempotent and can be re-run after merging other lanes.
//!
//! Usage (from the repository root):
//!   cargo run --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --bin migrate-syntax3-pipe -- [--check] [--quiet] [FILE...]
//! Without files it reads every tracked `.i`, `.md`, `.rs`, `.js` and `.mjs` file.
//! `--check` writes nothing and exits 1 when an edit is due.
//! Sources are `.i` files, `i` fences in Markdown, and string literals in Rust and
//! JavaScript that contain `|>` and parse as a module. Literals that contain `|>`
//! but do not parse are listed on stderr (`unparsed`) for review by hand.
#[path = "support/legacy/mod.rs"]
mod legacy;

use ilang_syntax::ast::*;
use legacy::parser::parse_module_old_pipe as parse_module;
use std::{fs, path::Path, process::Command};

// ---- Finding the groups ----

struct Finder<'a> {
    src: &'a str,
    /// (position, open) in `src`.
    marks: Vec<(usize, bool, usize)>,
}

fn is_pipe(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Binary {
            op: BinOp::Pipe,
            ..
        } | ExprKind::Case { .. }
    )
}

impl Finder<'_> {
    /// True when the text between the operands is the operator itself, which is
    /// not so for the `<>` that joins the parts of a string interpolation.
    fn written(&self, op: BinOp, lhs: &Expr, rhs: &Expr) -> bool {
        let (a, b) = (lhs.span.end as usize, rhs.span.start as usize);
        if a > b {
            return false;
        }
        let mut gap = &self.src[a..b];
        loop {
            gap = gap.trim_start();
            if let Some(rest) = gap.strip_prefix("//") {
                gap = rest.split_once('\n').map_or("", |(_, r)| r);
            } else {
                break;
            }
        }
        gap.starts_with(op.text())
    }

    fn group(&mut self, e: &Expr) {
        let len = (e.span.end - e.span.start) as usize;
        self.marks.push((e.span.start as usize, true, len));
        self.marks.push((e.span.end as usize, false, len));
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => self.expr(&b.value),
                Stmt::Env { value, .. } => self.expr(value),
                Stmt::Expr(e) => self.expr(e),
            }
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Named { pattern, .. } => self.pattern(pattern),
            PatternKind::Constructor { args, .. } => args.iter().for_each(|p| self.pattern(p)),
            PatternKind::Tuple(xs) => xs.iter().for_each(|p| self.pattern(p)),
            PatternKind::List { items, .. } => items.iter().for_each(|p| self.pattern(p)),
            PatternKind::BitArray(xs) => {
                for x in xs {
                    self.pattern(&x.value);
                    if let Some(a) = x.format.as_ref().and_then(|f| f.arg.as_ref()) {
                        self.expr(a);
                    }
                }
            }
            _ => {}
        }
    }

    fn clauses(&mut self, cs: &[Clause]) {
        for c in cs {
            c.patterns.iter().for_each(|p| self.pattern(p));
            if let Some(g) = &c.guard {
                self.expr(g);
            }
            self.expr(&c.body);
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Tuple(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::Record { base, fields } => {
                if let Some(b) = base {
                    self.expr(b);
                }
                fields.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::List { items, rest } => {
                items.iter().for_each(|x| self.expr(x));
                if let Some(r) = rest {
                    self.expr(r);
                }
            }
            ExprKind::BitArray(xs) => {
                for x in xs {
                    self.expr(&x.value);
                    if let Some(a) = x.format.as_ref().and_then(|f| f.arg.as_ref()) {
                        self.expr(a);
                    }
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    if let Param::Pattern(p) = p {
                        self.pattern(p);
                    }
                }
                self.block(&f.body);
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                self.clauses(clauses);
            }
            ExprKind::Clauses(cs) => self.clauses(cs),
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target)
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                self.expr(constructor);
                self.expr(base);
                fields.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { op, lhs, rhs } => {
                // Levels 1 to 5 of the syntax before the change: not `|>`, `+`, `*`.
                let weak = !matches!(
                    op,
                    BinOp::Pipe
                        | BinOp::Add
                        | BinOp::Sub
                        | BinOp::AddFloat
                        | BinOp::SubFloat
                        | BinOp::Mul
                        | BinOp::Div
                        | BinOp::Rem
                        | BinOp::MulFloat
                        | BinOp::DivFloat
                );
                if weak && self.written(*op, lhs, rhs) {
                    if is_pipe(lhs) {
                        self.group(lhs);
                    }
                    if is_pipe(rhs) {
                        self.group(rhs);
                    }
                }
                self.expr(lhs);
                self.expr(rhs);
            }
            _ => {}
        }
    }
}

/// The insertions `(position, text)` that group one module, or `None` when the
/// source is not a module of today's syntax.
fn insertions(src: &str) -> Option<Vec<(usize, &'static str)>> {
    let (module, diags) = parse_module(src);
    if !diags.is_empty() {
        return None;
    }
    let mut f = Finder {
        src,
        marks: Vec::new(),
    };
    for item in &module.items {
        match &item.kind {
            ItemKind::Binding(b) => f.expr(&b.value),
            ItemKind::MacroItem { args, .. } => args.iter().for_each(|a| f.expr(&a.value)),
            _ => {}
        }
        for a in &item.attributes {
            if let Some((args, _)) = &a.call {
                args.iter().for_each(|a| f.expr(&a.value));
            }
        }
    }
    // At one position, closers come first; openers go outermost first and closers
    // innermost first, so nested groups stay nested.
    let mut marks = f.marks;
    marks.sort_by_key(|&(pos, open, len)| (pos, open, if open { usize::MAX - len } else { len }));
    marks.dedup();
    Some(
        marks
            .into_iter()
            .map(|(pos, open, _)| (pos, if open { "{ " } else { " }" }))
            .collect(),
    )
}

// ---- Finding the sources ----

/// A decoded literal and the raw offset of each decoded byte (one past the end
/// included).
struct Literal {
    start: usize,
    value: String,
    raw: Vec<usize>,
}

/// Rust string literals, skipping comments, character literals and lifetimes.
fn rust_literals(text: &str) -> Vec<Literal> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if text[i..].starts_with("//") {
            i = text[i..].find('\n').map_or(b.len(), |n| i + n);
        } else if text[i..].starts_with("/*") {
            let mut depth = 1;
            i += 2;
            while depth > 0 && i < b.len() {
                if text[i..].starts_with("/*") {
                    depth += 1;
                    i += 2;
                } else if text[i..].starts_with("*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if b[i] == b'r'
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
            && raw_open(text, i).is_some()
        {
            let (hashes, open) = raw_open(text, i).unwrap();
            let close = format!("\"{}", "#".repeat(hashes));
            let end = open + text[open..].find(&close).expect("unterminated raw string");
            out.push(Literal {
                start: i,
                value: text[open..end].to_string(),
                raw: (open..=end).collect(),
            });
            i = end + close.len();
        } else if b[i] == b'"' {
            let (lit, next) = escaped(text, i, '"', false);
            out.push(lit);
            i = next;
        } else if b[i] == b'\'' {
            i += char_len(text, i);
        } else {
            i += 1;
        }
    }
    out
}

fn raw_open(text: &str, i: usize) -> Option<(usize, usize)> {
    let rest = &text[i + 1..];
    let hashes = rest.bytes().take_while(|&c| c == b'#').count();
    rest[hashes..]
        .starts_with('"')
        .then_some((hashes, i + 1 + hashes + 1))
}

/// Length of a character literal at `i`, or of the lone `'` of a lifetime.
fn char_len(text: &str, i: usize) -> usize {
    let rest = &text[i + 1..];
    let mut chars = rest.char_indices();
    match chars.next() {
        Some((_, '\\')) => {
            if let Some(n) = rest.find('\'').filter(|&n| n > 1) {
                // `'\''` has its quote at 2.
                let n = if rest[1..].starts_with('\'') {
                    rest[2..].find('\'').map_or(n, |m| m + 2)
                } else {
                    n
                };
                return n + 2;
            }
            1
        }
        Some((_, c)) if rest[c.len_utf8()..].starts_with('\'') => 1 + c.len_utf8() + 1,
        _ => 1,
    }
}

/// A quoted literal at `i` with Rust or JavaScript escapes.
fn escaped(text: &str, i: usize, quote: char, js: bool) -> (Literal, usize) {
    let mut value = String::new();
    let mut raw = Vec::new();
    let mut j = i + 1;
    let push = |value: &mut String, raw: &mut Vec<usize>, c: char, at: usize| {
        value.push(c);
        raw.extend(std::iter::repeat_n(at, c.len_utf8()));
    };
    while let Some(c) = text[j..].chars().next() {
        if c == quote {
            raw.push(j);
            j += 1;
            break;
        }
        if c != '\\' {
            push(&mut value, &mut raw, c, j);
            j += c.len_utf8();
            continue;
        }
        let at = j;
        j += 1;
        let e = text[j..].chars().next().unwrap();
        j += e.len_utf8();
        match e {
            '\n' | '\r' => {
                if !js {
                    while text[j..].starts_with(|c: char| c.is_whitespace()) {
                        j += text[j..].chars().next().unwrap().len_utf8();
                    }
                }
            }
            'n' => push(&mut value, &mut raw, '\n', at),
            'r' => push(&mut value, &mut raw, '\r', at),
            't' => push(&mut value, &mut raw, '\t', at),
            '0' => push(&mut value, &mut raw, '\0', at),
            'x' => {
                let n = u32::from_str_radix(&text[j..j + 2], 16).unwrap();
                j += 2;
                push(&mut value, &mut raw, char::from_u32(n).unwrap(), at);
            }
            'u' => {
                let braced = text[j..].starts_with('{');
                let (digits, next) = if braced {
                    let end = j + text[j..].find('}').unwrap();
                    (text[j + 1..end].replace('_', ""), end + 1)
                } else {
                    (text[j..j + 4].to_string(), j + 4)
                };
                j = next;
                let n = u32::from_str_radix(&digits, 16).unwrap();
                push(&mut value, &mut raw, char::from_u32(n).unwrap_or('?'), at);
            }
            c => push(&mut value, &mut raw, c, at),
        }
    }
    (
        Literal {
            start: i,
            value,
            raw,
        },
        j,
    )
}

/// JavaScript strings and templates without `${`, skipping comments.
fn js_literals(text: &str) -> Vec<Literal> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if text[i..].starts_with("//") {
            i = text[i..].find('\n').map_or(b.len(), |n| i + n);
        } else if text[i..].starts_with("/*") {
            i = text[i..].find("*/").map_or(b.len(), |n| i + n + 2);
        } else if matches!(b[i], b'"' | b'\'' | b'`') {
            let (lit, next) = escaped(text, i, b[i] as char, true);
            if !lit.value.contains("${") {
                out.push(lit);
            }
            i = next;
        } else {
            i += text[i..].chars().next().unwrap().len_utf8();
        }
    }
    out
}

/// The `i` fences of a Markdown file: (offset of the first line, source).
fn fences(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0;
    let mut open: Option<usize> = None;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        match open {
            None => {
                if let Some(rest) = bare.strip_prefix("```i")
                    && (rest.is_empty() || rest.starts_with([' ', '\t']))
                {
                    open = Some(offset + line.len());
                }
            }
            Some(start) => {
                if bare.trim_end_matches([' ', '\t']) == "```" {
                    out.push((start, &text[start..offset]));
                    open = None;
                }
            }
        }
        offset += line.len();
    }
    out
}

// ---- Applying ----

struct Report {
    groups: usize,
    unparsed: Vec<String>,
}

/// The migrated text of one file.
fn migrate_file(path: &str, text: &str, report: &mut Report) -> String {
    let mut inserts: Vec<(usize, &'static str)> = Vec::new();
    let line = |at: usize| text[..at].bytes().filter(|&c| c == b'\n').count() + 1;
    if path.ends_with(".i") {
        match insertions(text) {
            Some(v) => inserts.extend(v),
            None if text.contains("|>") => report.unparsed.push(format!("{path}: whole file")),
            None => {}
        }
    } else if path.ends_with(".md") {
        for (start, source) in fences(text) {
            if !source.contains("|>") {
                continue;
            }
            match insertions(source) {
                Some(v) => inserts.extend(v.into_iter().map(|(p, t)| (start + p, t))),
                None => report
                    .unparsed
                    .push(format!("{path}:{}: fence", line(start))),
            }
        }
    } else {
        let literals = if path.ends_with(".rs") {
            rust_literals(text)
        } else {
            js_literals(text)
        };
        for lit in literals {
            if !lit.value.contains("|>") {
                continue;
            }
            match insertions(&lit.value) {
                Some(v) => inserts.extend(v.into_iter().map(|(p, t)| (lit.raw[p], t))),
                None => report
                    .unparsed
                    .push(format!("{path}:{}: literal", line(lit.start))),
            }
        }
    }
    report.groups += inserts.len() / 2;
    // Insertions at one raw position keep their order.
    let mut out = String::with_capacity(text.len() + inserts.len() * 2);
    let mut last = 0;
    for (pos, t) in inserts {
        out.push_str(&text[last..pos]);
        out.push_str(t);
        last = pos;
    }
    out.push_str(&text[last..]);
    out
}

fn files() -> Vec<String> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.i",
            "*.md",
            "*.rs",
            "*.js",
            "*.mjs",
        ])
        .output()
        .expect("git ls-files");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Files that must keep their text: the controlling note shows both spellings, the
/// legacy parser and this tool hold the old form on purpose.
fn skipped(path: &str) -> bool {
    path == "docs/design/syntax-3.md"
        || path.starts_with("crates/ilang-syntax/examples/legacy/")
        || path.starts_with("scripts/")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let quiet = args.iter().any(|a| a == "--quiet");
    let mut paths: Vec<String> = args.into_iter().filter(|a| !a.starts_with("--")).collect();
    if paths.is_empty() {
        paths = files();
    }
    let mut total = Report {
        groups: 0,
        unparsed: Vec::new(),
    };
    let mut changed = 0;
    for path in paths {
        if skipped(&path) || !Path::new(&path).is_file() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let before = total.groups;
        let out = migrate_file(&path, &text, &mut total);
        if out != text {
            changed += 1;
            if !quiet {
                println!("{path}: {} groups", total.groups - before);
            }
            if !check {
                fs::write(&path, out).expect("write");
            }
        }
    }
    for u in &total.unparsed {
        eprintln!("unparsed {u}");
    }
    println!("{} groups in {changed} files", total.groups);
    if check && changed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> String {
        let mut r = Report {
            groups: 0,
            unparsed: Vec::new(),
        };
        migrate_file("t.i", src, &mut r)
    }

    #[test]
    fn groups_both_sides_and_is_idempotent() {
        // Sources are written without the `r = ` binding, so that the repository's
        // source check does not read them as i literals (they are the old grouping).
        for (before, after) in [
            ("a |> f == b", "{ a |> f } == b"),
            ("a == b |> f", "a == { b |> f }"),
            ("a |> f |> g <> b", "{ a |> f |> g } <> b"),
            ("a |> f < b |> g", "{ a |> f } < { b |> g }"),
            ("a + b |> f", "a + b |> f"),
            ("x |> { 0 -> 1 } == 1", "{ x |> { 0 -> 1 } } == 1"),
            ("{ a |> f } == b", "{ a |> f } == b"),
            ("a || b |> f && c |> g", "a || { b |> f } && { c |> g }"),
        ] {
            let once = run(&format!("r = {before}"));
            assert_eq!(once, format!("r = {after}"), "{before}");
            assert_eq!(run(&once), once, "{before}");
        }
        let text = format!("r = \"a${{x |> f}}\"");
        assert_eq!(run(&text), text);
    }

    #[test]
    fn edits_inside_rust_strings() {
        let text = "const S: &str = \"r = a |> f == b\\n\";\n";
        let mut r = Report {
            groups: 0,
            unparsed: Vec::new(),
        };
        let out = migrate_file("t.rs", text, &mut r);
        assert_eq!(out, "const S: &str = \"r = { a |> f } == b\\n\";\n");
    }
}
