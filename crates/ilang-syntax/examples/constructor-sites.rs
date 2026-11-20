//! Migration helper: report every bracketless constructor use in a source,
//! as the current parser reads it, so the migration can rewrite `True` to a
//! call `True()`.
//!
//! The tree says where the uses are. In a value position an uppercase
//! `ExprKind::Name` is a constructor use unless it is directly the callee of
//! a call (`True(x)` already has brackets). In a pattern a
//! `PatternKind::Ctor` without arguments is one. Types are never visited: a
//! type definition, a declaration, a function's return and service types and
//! a typed binder's type are type positions. A label (`Flags(True = x)`) is
//! a key, not a use, and is not reported. In a Lisp template every uppercase
//! name that is not the head of a round list is reported as a pattern site,
//! and inside an unquote as a value site.
//!
//! The helper also lists the zero-field types of the source, `T = ()` (also
//! with a head, `X(A) = ()`), whose constructors are the ones the migration
//! rewrites. `CTOR` lines list every type defined with data (`T = (items)`)
//! with its field count, zero included, so a caller can also tell a payload
//! constructor from a zero-field one.
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
//! FILE <path>                         (or SNIPPET <index> for a text request)
//! ZERO <name>                         one per type defined as `()`
//! CTOR <name> <fields>                one per type defined with data, `T = (…)`
//! SITE value <start> <end> <hex>      a constructor used as a value
//! SITE pattern <start> <end> <hex>    a constructor matched without brackets
//! DIAG <code> <hex>                   one line per parse diagnostic
//! END
//! ```
//!
//! Offsets are byte offsets into the UTF-8 source, end exclusive, and cover
//! the written name without a module part; `<hex>` is that name's UTF-8 bytes
//! hex-encoded, as is a diagnostic message. `SITE` lines are ascending.
//!
//! `FAIL <path> <message>` replaces the group when the request itself is
//! wrong. Exit status 0 unless a request line is malformed.
//!
//! A text that does not parse still gets its `DIAG` lines and the sites the
//! recovered tree still shows; the caller decides whether to trust them.

use std::io::{self, BufRead, Read, Write};

use ilang_syntax::ast::{
    Binding, Block, DItem, DPItem, Expr, ExprKind, Format, Function, InterpPart, ItemKind,
    Module, Pattern, PatternKind, Stmt, TypeRhs,
};
use ilang_syntax::lisp::{Datum, DatumKind};
use ilang_syntax::span::Span;

#[derive(Clone, Copy, PartialEq)]
enum Surface {
    Normal,
    Lisp,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Value,
    Pattern,
}

impl Kind {
    fn text(self) -> &'static str {
        match self {
            Kind::Value => "value",
            Kind::Pattern => "pattern",
        }
    }
}

#[derive(Default)]
struct Sites {
    sites: Vec<(u32, u32, Kind, String)>,
}

fn is_upper(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

impl Sites {
    fn push(&mut self, kind: Kind, span: Span, name: &str) {
        self.sites.push((span.start, span.end, kind, name.to_string()));
    }

    fn module(&mut self, module: &Module) {
        for item in &module.items {
            match &item.kind {
                ItemKind::Bind(b) => self.binding(b),
                ItemKind::Macro { args, .. } => self.ditems(args),
                // Type positions.
                ItemKind::Decl { .. } | ItemKind::TypeDef(_) => {}
            }
        }
    }

    fn binding(&mut self, b: &Binding) {
        self.pattern(&b.pattern);
        self.expr(&b.value, false);
    }

    fn block(&mut self, b: &Block) {
        for stmt in &b.stmts {
            match stmt {
                Stmt::Bind(b) => self.binding(b),
                Stmt::Expr(e) => self.expr(e, false),
            }
        }
    }

    fn function(&mut self, f: &Function) {
        for p in &f.params {
            // A parameter key is a label, not a use.
            for c in &p.choices {
                self.pattern(c);
            }
        }
        if let Some(g) = &f.guard {
            self.expr(g, false);
        }
        // `services` and `ret` are types.
        self.block(&f.body);
    }

    fn ditems(&mut self, items: &[DItem]) {
        for i in items {
            match i {
                // A label is a key, not a use.
                DItem::Pos(e) | DItem::Label(_, e) | DItem::Spread(e) => self.expr(e, false),
            }
        }
    }

    fn format(&mut self, f: &Option<Format>) {
        if let Some(arg) = f.as_ref().and_then(|f| f.arg.as_ref()) {
            self.expr(arg, false);
        }
    }

    fn expr(&mut self, e: &Expr, is_callee: bool) {
        match &e.kind {
            ExprKind::Name { name, .. } => {
                if !is_callee && is_upper(&name.name) {
                    self.push(Kind::Value, name.span.0, &name.name);
                }
            }
            ExprKind::Quote(q) => self.datum(&q.0, Kind::Pattern),
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) => {}
            ExprKind::Interp(parts) => {
                for p in parts {
                    if let InterpPart::Expr(e) = p {
                        self.expr(e, false);
                    }
                }
            }
            ExprKind::Data(items) => self.ditems(items),
            ExprKind::List { items, rest } => {
                for x in items {
                    self.expr(x, false);
                }
                if let Some(x) = rest {
                    self.expr(x, false);
                }
            }
            ExprKind::BitArray(xs) => {
                for x in xs {
                    self.expr(&x.value, false);
                    self.format(&x.format);
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Chain(fs) => {
                for f in fs {
                    self.function(f);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee, true);
                self.ditems(args);
            }
            ExprKind::Provide { target, services } => {
                self.expr(target, false);
                self.expr(services, false);
            }
            ExprKind::Field { target, .. }
            | ExprKind::Index { target, .. }
            | ExprKind::Unary { expr: target, .. } => self.expr(target, false),
            ExprKind::Binary { lhs, rhs, .. } | ExprKind::Pipe { lhs, rhs } => {
                self.expr(lhs, false);
                self.expr(rhs, false);
            }
        }
    }

    fn dpitems(&mut self, items: &[DPItem]) {
        for i in items {
            match i {
                // A key is a label, not a use.
                DPItem::Pos(p) | DPItem::Key(_, p) => self.pattern(p),
                DPItem::Rest(_) => {}
            }
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Quote(q) => {
                self.datum(&q.datum.0, Kind::Pattern);
                for (_, capture) in &q.captures {
                    self.pattern(capture);
                }
            }
            PatternKind::Ctor { name, args, .. } => match args {
                None => self.push(Kind::Pattern, name.span.0, &name.name),
                Some(args) => self.dpitems(args),
            },
            PatternKind::Data(items) => self.dpitems(items),
            PatternKind::List { items, .. } | PatternKind::And(items) => {
                for p in items {
                    self.pattern(p);
                }
            }
            PatternKind::BitArray(xs) => {
                for x in xs {
                    self.pattern(&x.value);
                    self.format(&x.format);
                }
            }
            // `Typed`'s type is a type position; its binder is a name.
            PatternKind::Typed { .. }
            | PatternKind::Var(_)
            | PatternKind::Discard(_)
            | PatternKind::Int { .. }
            | PatternKind::Float { .. }
            | PatternKind::String(_)
            | PatternKind::Prefix { .. } => {}
        }
    }

    /// A template datum: uppercase names are sites of `kind` (`Pattern` in a
    /// template, `Value` inside an unquote), except the head of a round list,
    /// which is a callee.
    fn datum(&mut self, d: &Datum, kind: Kind) {
        match &d.kind {
            DatumKind::UpName(name) => self.push(kind, d.span, name),
            DatumKind::Round(xs) => {
                let mut rest = xs.iter();
                if let Some(first) = xs.first()
                    && matches!(first.kind, DatumKind::UpName(_))
                {
                    rest.next();
                }
                for x in rest {
                    self.datum(x, kind);
                }
            }
            DatumKind::Square(xs) | DatumKind::Curly(xs) | DatumKind::Bits(xs) => {
                for x in xs {
                    self.datum(x, kind);
                }
            }
            DatumKind::Quote(x) | DatumKind::Splice(x) => self.datum(x, kind),
            DatumKind::Unquote(x) => self.datum(x, Kind::Value),
            DatumKind::Name(_)
            | DatumKind::Discard(_)
            | DatumKind::Int(_)
            | DatumKind::Float(_)
            | DatumKind::String(_)
            | DatumKind::Symbol(_) => {}
        }
    }
}

/// The types of the module written as `T = ()`.
fn zero_types(module: &Module) -> Vec<String> {
    module
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::TypeDef(t) if matches!(&t.rhs, TypeRhs::Data(items) if items.is_empty()) => {
                Some(t.name.name.clone())
            }
            _ => None,
        })
        .collect()
}

/// Every type of the module written as `T = (items)`, with `items.len()`.
fn data_types(module: &Module) -> Vec<(String, usize)> {
    module
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::TypeDef(t) => match &t.rhs {
                TypeRhs::Data(items) => Some((t.name.name.clone(), items.len())),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

struct Report {
    zero: Vec<String>,
    ctors: Vec<(String, usize)>,
    sites: Vec<(u32, u32, Kind, String)>,
    diags: Vec<(String, String)>,
}

fn run(surface: Surface, source: &str) -> Report {
    let (module, diags) = match surface {
        Surface::Normal => ilang_syntax::parse::parse_module(source),
        Surface::Lisp => ilang_syntax::rows::from_lisp(source),
    };
    let mut sites = Sites::default();
    sites.module(&module);
    let mut sites = sites.sites;
    sites.sort();
    sites.dedup();
    Report {
        zero: zero_types(&module),
        ctors: data_types(&module),
        sites,
        diags: diags
            .into_iter()
            .map(|d| (d.code.as_str().to_string(), d.message))
            .collect(),
    }
}

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

fn report(output: &mut impl Write, name: &str, source: &str, surface: Surface) {
    let Report {
        zero,
        ctors,
        sites,
        diags,
    } = run(surface, source);
    writeln!(output, "{name}").unwrap();
    for name in zero {
        writeln!(output, "ZERO {name}").unwrap();
    }
    for (name, fields) in ctors {
        writeln!(output, "CTOR {name} {fields}").unwrap();
    }
    for (start, end, kind, name) in sites {
        writeln!(output, "SITE {} {start} {end} {}", kind.text(), hex(&name)).unwrap();
    }
    for (code, message) in diags {
        writeln!(output, "DIAG {code} {}", hex(&message)).unwrap();
    }
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
