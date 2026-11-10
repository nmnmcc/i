//! A recursive-descent parser for i (§2.2 to §8.1).
//!
//! The line rule (§1.7) is one check: a token with `brk` never continues a
//! construct. Inside `( )`, `[ ]`, and `<< >>` such a token counts as a `,`;
//! inside `{ }` and at the top level it begins the next statement, clause,
//! constructor, or item.

use crate::ast::*;
use crate::lexer::{DocComment, lex};
use crate::span::Span;
use crate::token::{Token, TokenKind as T};
use crate::{Code, Diagnostic};
use std::collections::HashMap;

type PResult<X> = Result<X, Diagnostic>;

/// What a `{` that begins an expression begins (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Brace {
    Block,
    Clauses,
    /// `{ name: … }`, kept for records (E0114); read as clauses.
    Record,
}

/// Counts of one parse, for tests: the `{` that were decided, and the
/// decisions made. Each `{` is decided once.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseStats {
    pub decisions: usize,
    pub braces: usize,
}

/// Parses a module in the syntax of §2.2. The module is complete as
/// far as the source allows; the diagnostics say what was wrong, sorted by
/// position.
pub fn parse_module(source: &str) -> (Module, Vec<Diagnostic>) {
    let (module, diags, _) = run(source, true);
    (module, diags)
}

#[doc(hidden)]
pub fn parse_stats(source: &str) -> ParseStats {
    run(source, true).2
}

/// Parses without reusing brace decisions, for the cache equivalence test.
#[doc(hidden)]
pub fn parse_module_uncached(source: &str) -> (Module, Vec<Diagnostic>) {
    let (module, diags, _) = run(source, false);
    (module, diags)
}

fn run(source: &str, cache: bool) -> (Module, Vec<Diagnostic>, ParseStats) {
    let lexed = lex(source);
    let mut parser = Parser {
        toks: &lexed.tokens,
        close: &lexed.close,
        docs: &lexed.docs,
        doc_pos: 0,
        pos: 0,
        diags: Vec::new(),
        guard: false,
        decided: HashMap::new(),
        decisions: 0,
        tuple_type: false,
        cache,
    };
    let module = parser.module();
    let stats = ParseStats {
        decisions: parser.decisions,
        braces: parser.decided.len(),
    };
    let mut diags = lexed.diagnostics.clone();
    diags.append(&mut parser.diags);
    diags.sort_by_key(|d| d.span.start);
    // Recovery may go on at the token that failed, and fail there again.
    diags.dedup();
    if !diags.is_empty()
        && let Some(hint) = crate::old_syntax_hint(source)
    {
        for d in &mut diags {
            d.message.push_str("; ");
            d.message.push_str(hint);
        }
    }
    (module, diags, stats)
}

const RECORD_RESERVED: &str =
    "`{ name: e }` is kept for records; a tuple type in a clause pattern is put in `{ }`";

const TUPLE_NOTE: &str = "; a tuple type in a clause pattern is put in `{ }`";

struct Parser<'a> {
    toks: &'a [Token],
    close: &'a [Option<u32>],
    docs: &'a [DocComment],
    doc_pos: usize,
    pos: usize,
    diags: Vec<Diagnostic>,
    /// Set in a guard: a `->` ends the clause head, so it never makes a `( )`
    /// a function (§6.1). Cleared inside brackets.
    guard: bool,
    /// The decision for each `{` that begins an expression, by the index of
    /// its token. A decision depends only on the tokens, so it
    /// holds after a speculative parse goes back.
    decided: HashMap<usize, Brace>,
    decisions: usize,
    /// Set when a pattern has `x: ( … )`, for the note on tuple types in a
    /// clause (§7.1).
    tuple_type: bool,
    cache: bool,
}

fn binop(kind: &T) -> Option<BinOp> {
    use BinOp::*;
    Some(match kind {
        T::PipePipe => Or,
        T::AmpAmp => And,
        T::EqEq => Eq,
        T::NotEq => NotEq,
        T::Lt => Lt,
        T::LtEq => LtEq,
        T::Gt => Gt,
        T::GtEq => GtEq,
        T::LtDot => LtFloat,
        T::LtEqDot => LtEqFloat,
        T::GtDot => GtFloat,
        T::GtEqDot => GtEqFloat,
        T::LtGt => Concat,
        T::PipeGt => Pipe,
        T::Plus => Add,
        T::Minus => Sub,
        T::PlusDot => AddFloat,
        T::MinusDot => SubFloat,
        T::Star => Mul,
        T::Slash => Div,
        T::Percent => Rem,
        T::StarDot => MulFloat,
        T::SlashDot => DivFloat,
        _ => return None,
    })
}

/// `Name` of §6.1: `x`, `C`, `m.C`, or `m.x`, which is a field here.
fn is_name(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name { .. } => true,
        ExprKind::Field { target, .. } => {
            matches!(target.kind, ExprKind::Name { module: None, .. })
        }
        _ => false,
    }
}

/// Whether the leftmost operand of `e`, through operators, calls, `.f`,
/// and `.k`, is a clause block.
fn starts_with_clauses(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Clauses(_) => true,
        ExprKind::Binary { lhs: e, .. }
        | ExprKind::Call { callee: e, .. }
        | ExprKind::Field { target: e, .. }
        | ExprKind::TupleIndex { target: e, .. } => starts_with_clauses(e),
        _ => false,
    }
}

/// The parameters of a type definition head when it is a list of plain
/// `UpName`s with no
/// module and no arguments (§4.3).
fn head_params(types: &[TypeExpr]) -> Option<Vec<Ident>> {
    let upper = types
        .iter()
        .all(|t| matches!(&t.kind, TypeKind::Named { module: None, args, .. } if args.is_empty()));
    if !upper {
        return None;
    }
    Some(
        types
            .iter()
            .map(|t| match &t.kind {
                TypeKind::Named { name, .. } => name.clone(),
                _ => unreachable!(),
            })
            .collect(),
    )
}

impl Parser<'_> {
    // ---- Tokens ----

    fn tok(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn kind(&self) -> &T {
        &self.tok().kind
    }

    fn nth(&self, n: usize) -> &T {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].kind
    }

    fn at(&self, kind: &T) -> bool {
        self.kind() == kind
    }

    /// The current token is `kind` and does not begin a new line part.
    fn at_joined(&self, kind: &T) -> bool {
        self.at(kind) && !self.tok().brk
    }

    fn bump(&mut self) -> Span {
        let span = self.tok().span;
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        span
    }

    fn eat(&mut self, kind: &T) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    fn start(&self) -> u32 {
        self.tok().span.start
    }

    fn span_from(&self, start: u32) -> Span {
        let end = self.toks[self.pos.saturating_sub(1)].span.end;
        Span::new(start, end.max(start))
    }

    fn error<X>(&self, expected: &str) -> PResult<X> {
        let found = if self.tok().brk {
            format!("a line break before {}", self.kind())
        } else {
            self.kind().to_string()
        };
        let msg = format!("expected {expected}, found {found}");
        Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg))
    }

    fn expect(&mut self, kind: &T) -> PResult<Span> {
        if self.at(kind) {
            Ok(self.bump())
        } else {
            self.error(&format!("`{}`", kind.text()))
        }
    }

    fn name(&mut self) -> PResult<Ident> {
        match self.kind() {
            T::Name(n) => {
                let name = n.clone();
                Ok(Ident {
                    name,
                    span: self.bump(),
                })
            }
            _ => self.error("a name"),
        }
    }

    fn up_name(&mut self) -> PResult<Ident> {
        match self.kind() {
            T::UpName(n) => {
                let name = n.clone();
                Ok(Ident {
                    name,
                    span: self.bump(),
                })
            }
            _ => self.error("an UpName"),
        }
    }

    /// `name .` followed by an UpName: a module qualifier.
    fn at_qualified(&self) -> bool {
        matches!(self.kind(), T::Name(_))
            && *self.nth(1) == T::Dot
            && matches!(self.nth(2), T::UpName(_))
    }

    fn qualifier(&mut self) -> PResult<Option<Ident>> {
        if !self.at_qualified() {
            return Ok(None);
        }
        let module = self.name()?;
        self.bump();
        Ok(Some(module))
    }

    /// Runs `f` inside brackets, where the guard restriction does not hold.
    fn nested<X>(&mut self, f: impl FnOnce(&mut Self) -> PResult<X>) -> PResult<X> {
        let saved = self.guard;
        self.guard = false;
        let result = f(self);
        self.guard = saved;
        result
    }

    /// The parts of a bracketed list, after the opening bracket, with `,` or a
    /// break between them and maybe a `,` at the end. Consumes the closing
    /// bracket.
    fn seq<X>(
        &mut self,
        close: T,
        empty: bool,
        mut part: impl FnMut(&mut Self) -> PResult<X>,
    ) -> PResult<Vec<X>> {
        self.nested(|p| {
            let mut parts = Vec::new();
            loop {
                if p.at(&close) {
                    if parts.is_empty() && !empty {
                        return p.error("a part");
                    }
                    break;
                }
                parts.push(part(p)?);
                if p.at(&close) {
                    break;
                }
                if !p.eat(&T::Comma) && !p.tok().brk {
                    return p.error(&format!("`,` or `{}`", close.text()));
                }
            }
            p.bump();
            Ok(parts)
        })
    }

    /// After an error in a part that began at `start`, skips to the next break
    /// at the same level, or to the closing token at `end`. At the top level
    /// (`end` is the end of the file), only a token that may begin an item is
    /// a place to go on.
    fn recover(&mut self, start: usize, end: usize) {
        let failed = self.pos;
        let top = end + 1 == self.toks.len();
        let resumes =
            |t: &Token| t.brk && !(top && matches!(t.kind, T::LBrace | T::Bang | T::DotDot));
        let mut i = start;
        while i < end && !(i > start && i >= failed && resumes(&self.toks[i])) {
            i = match self.close[i] {
                Some(c) if (c as usize) < end => c as usize + 1,
                _ => i + 1,
            };
        }
        self.pos = i.min(end);
    }

    fn take_docs(&mut self) -> Vec<String> {
        let start = self.start();
        let prev_end = match self.pos {
            0 => 0,
            i => self.toks[i - 1].span.end,
        };
        let mut out = Vec::new();
        while let Some(doc) = self.docs.get(self.doc_pos) {
            if doc.span.start >= start {
                break;
            }
            if !doc.module && doc.span.start >= prev_end {
                out.push(doc.text.clone());
            }
            self.doc_pos += 1;
        }
        out
    }

    // ---- Module (§2.2 to §2.5) ----

    fn module(&mut self) -> Module {
        let mut module = Module {
            doc: self
                .docs
                .iter()
                .filter(|d| d.module)
                .map(|d| d.text.clone())
                .collect(),
            ..Module::default()
        };
        let eof = self.toks.len() - 1;
        let (mut imports, mut exports) = (false, false);
        loop {
            let start = self.pos;
            let from = self.start();
            let is_import = if self.at_header_group() {
                let close = self.close[self.pos].unwrap() as usize;
                let is_import = self.slash_inside(self.pos, close);
                let msg = if close == self.pos + 1 {
                    Some("a file with nothing to import or export leaves the group out")
                } else if is_import && exports {
                    Some("the imports come before the exports")
                } else if is_import && imports || !is_import && exports {
                    Some("a file has at most one imports group and one exports group")
                } else {
                    None
                };
                if let Some(msg) = msg {
                    let span = Span::new(from, self.toks[close].span.end);
                    self.diags
                        .push(Diagnostic::new(Code::HeaderGroup, span, msg));
                    self.pos = (close + 1).min(eof);
                    continue;
                }
                is_import
            } else {
                break;
            };
            if is_import {
                imports = true;
                match self.import() {
                    Ok(items) => {
                        module.imports = items;
                        module.imports_span = Some(self.span_from(from));
                    }
                    Err(d) => {
                        self.diags.push(d);
                        self.recover(start, eof);
                    }
                }
            } else {
                exports = true;
                match self.export() {
                    Ok(items) => {
                        module.exports = items;
                        module.exports_span = Some(self.span_from(from));
                    }
                    Err(d) => {
                        self.diags.push(d);
                        self.recover(start, eof);
                    }
                }
            }
        }
        while !self.at(&T::Eof) {
            let start = self.pos;
            match self.item() {
                Ok(item) => module.items.push(item),
                Err(d) => {
                    self.diags.push(d);
                    self.recover(start, eof);
                }
            }
        }
        module
    }

    /// A `(` whose `)` is not followed by `=`: the imports or the exports
    /// at the start of a file, and an error after it (§2.2).
    fn at_header_group(&self) -> bool {
        self.at(&T::LParen)
            && self.close[self.pos].is_some_and(|c| self.toks[c as usize + 1].kind != T::Eq)
    }

    /// Whether a `/` is directly inside the brackets at `open` and `close`.
    fn slash_inside(&self, open: usize, close: usize) -> bool {
        let mut i = open + 1;
        while i < close {
            if self.toks[i].kind == T::Slash {
                return true;
            }
            i = match self.close[i] {
                Some(c) => c as usize + 1,
                None => i + 1,
            };
        }
        false
    }

    /// The imports group.
    fn import(&mut self) -> PResult<Vec<ImportItem>> {
        self.expect(&T::LParen)?;
        self.seq(T::RParen, true, |p| {
            let start = p.start();
            let alias = if matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Eq {
                let alias = p.name()?;
                p.bump();
                Some(alias)
            } else {
                None
            };
            let mut names = Vec::new();
            if p.eat(&T::LParen) {
                names = p.seq(T::RParen, false, |p| match p.kind() {
                    T::UpName(_) => p.up_name(),
                    _ => p.name(),
                })?;
                p.expect(&T::Eq)?;
            }
            let mut path = vec![p.name()?];
            while p.eat(&T::Slash) {
                path.push(p.name()?);
            }
            let span = p.span_from(start);
            if path.len() < 2 {
                let msg = "a module path has at least two parts, as in `i/fail`";
                return Err(Diagnostic::new(Code::ShortModulePath, span, msg));
            }
            Ok(ImportItem {
                alias,
                names,
                path,
                span,
            })
        })
    }

    /// The exports group.
    fn export(&mut self) -> PResult<Vec<ExportItem>> {
        self.expect(&T::LParen)?;
        self.seq(T::RParen, true, |p| {
            let start = p.start();
            if matches!(p.kind(), T::Name(_)) {
                let name = p.name()?;
                let span = name.span;
                return Ok(ExportItem {
                    name,
                    constructors: false,
                    span,
                });
            }
            let name = p.up_name()?;
            let constructors = p.at_joined(&T::LParen);
            if constructors {
                p.bump();
                p.expect(&T::DotDot)?;
                p.expect(&T::RParen)?;
            }
            Ok(ExportItem {
                name,
                constructors,
                span: p.span_from(start),
            })
        })
    }

    fn item(&mut self) -> PResult<Item> {
        let mut doc = self.take_docs();
        let start = self.start();
        let mut attributes = Vec::new();
        while self.at(&T::At) {
            attributes.push(self.attribute()?);
            // §1.2: a `///` after an attribute is still before the item.
            doc.extend(self.take_docs());
        }
        if self.at_header_group() {
            let close = self.close[self.pos].unwrap() as usize;
            let span = Span::new(self.start(), self.toks[close].span.end);
            let msg = if attributes.is_empty() {
                "a group in `( )` that is not followed by `=` is the imports or the exports, which come first in a file"
            } else {
                "a group in `( )` with an attribute before it is not a binding, because no `=` follows it; the imports and the exports have no attribute before them"
            };
            self.pos = (close + 1).min(self.toks.len() - 1);
            return Err(Diagnostic::new(Code::HeaderGroup, span, msg));
        }
        let kind = if matches!(self.kind(), T::UpName(_)) {
            ItemKind::TypeDef(self.type_def()?)
        } else if self.at_type_params() {
            let bstart = self.start();
            let (name, type_params, ty) = self.declared_head()?;
            match ty {
                Some(ty) if !self.at(&T::Eq) => ItemKind::Declaration {
                    name,
                    type_params,
                    ty,
                },
                ty => {
                    let pattern = Self::declared_pattern(name, ty);
                    self.expect(&T::Eq)?;
                    let value = self.expr()?;
                    ItemKind::Binding(Binding {
                        pattern,
                        value,
                        span: self.span_from(bstart),
                        type_params,
                    })
                }
            }
        } else if self.at_macro_item() {
            let module = if *self.nth(1) == T::Dot {
                let module = self.name()?;
                self.bump();
                Some(module)
            } else {
                None
            };
            let name = self.name()?;
            self.bump();
            let args = self.args()?;
            ItemKind::MacroItem { module, name, args }
        } else {
            let bstart = self.start();
            let Some(pattern) = self.binding_head()? else {
                // `name: Type` with no `=` is a declaration; the checker
                // reports it when `@external` is missing (§2.5).
                if matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Colon {
                    let name = self.name()?;
                    self.bump();
                    let ty = self.ty()?;
                    return Ok(Item {
                        doc,
                        attributes,
                        kind: ItemKind::Declaration {
                            name,
                            type_params: Vec::new(),
                            ty,
                        },
                        span: self.span_from(start),
                    });
                }
                return self.error("an item: a type definition, or a binding `pattern = value`");
            };
            let value = self.expr()?;
            ItemKind::Binding(Binding {
                pattern,
                value,
                span: self.span_from(bstart),
                type_params: Vec::new(),
            })
        };
        Ok(Item {
            doc,
            attributes,
            kind,
            span: self.span_from(start),
        })
    }

    /// `name ( UpName {, UpName} )` with the `(` on the line of the name,
    /// followed by `=` or `:`: a binding or a declaration with type
    /// parameters (§2.5, §6.1 test 1).
    fn at_type_params(&self) -> bool {
        if !matches!(self.kind(), T::Name(_)) {
            return false;
        }
        let open = self.pos + 1;
        if open >= self.toks.len() || self.toks[open].kind != T::LParen || self.toks[open].brk {
            return false;
        }
        let Some(close) = self.close[open] else {
            return false;
        };
        let close = close as usize;
        let mut j = open + 1;
        loop {
            if !matches!(self.toks[j].kind, T::UpName(_)) {
                return false;
            }
            j += 1;
            if j == close {
                break;
            }
            if self.toks[j].kind == T::Comma {
                j += 1;
                if j == close {
                    break;
                }
            } else if !self.toks[j].brk {
                return false;
            }
        }
        matches!(self.toks[close + 1].kind, T::Eq | T::Colon)
    }

    /// `name(A, …)` and then `: Type` if there is one.
    #[allow(clippy::type_complexity)]
    fn declared_head(&mut self) -> PResult<(Ident, Vec<Ident>, Option<TypeExpr>)> {
        let name = self.name()?;
        self.bump();
        let type_params = self.seq(T::RParen, false, Self::up_name)?;
        let ty = if self.eat(&T::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        Ok((name, type_params, ty))
    }

    /// The pattern of a binding with type parameters: the name, with its type.
    fn declared_pattern(name: Ident, ty: Option<TypeExpr>) -> Pattern {
        match ty {
            None => Pattern {
                kind: PatternKind::Var(name.name),
                span: name.span,
            },
            Some(ty) => Pattern {
                span: name.span.to(ty.span),
                kind: PatternKind::Typed {
                    binder: Binder {
                        name: name.name,
                        discard: false,
                        span: name.span,
                    },
                    ty,
                },
            },
        }
    }

    /// `[name .] name (` … `)` with no `=` or `:` after the `)`: a
    /// `MacroItem` (§2.2).
    fn at_macro_item(&self) -> bool {
        if !matches!(self.kind(), T::Name(_)) {
            return false;
        }
        let mut i = self.pos + 1;
        if self.toks[i].kind == T::Dot && matches!(self.nth(2), T::Name(_)) {
            i += 2;
        }
        if i >= self.toks.len() || self.toks[i].kind != T::LParen || self.toks[i].brk {
            return false;
        }
        self.close[i].is_some_and(|c| !matches!(self.toks[c as usize + 1].kind, T::Eq | T::Colon))
    }

    fn attribute(&mut self) -> PResult<Attribute> {
        let start = self.bump().start;
        let module = if matches!(self.kind(), T::Name(_))
            && *self.nth(1) == T::Dot
            && matches!(self.nth(2), T::Name(_))
        {
            let module = self.name()?;
            self.bump();
            Some(module)
        } else {
            None
        };
        let name = self.name()?;
        let mut args = Vec::new();
        let mut call = None;
        if self.at_joined(&T::LParen) {
            let open = self.bump().start;
            let list = self.args()?;
            let span = self.span_from(open);
            let strings: Option<Vec<(String, Span)>> = list
                .iter()
                .map(|a| match (&a.label, &a.value.kind) {
                    (None, ExprKind::String(s)) => Some((s.clone(), a.value.span)),
                    _ => None,
                })
                .collect();
            match strings {
                Some(strings) if !strings.is_empty() => args = strings,
                _ => call = Some((list, span)),
            }
        }
        Ok(Attribute {
            module,
            name,
            args,
            call,
            span: self.span_from(start),
        })
    }

    /// §4.3.
    fn type_def(&mut self) -> PResult<TypeDef> {
        let name = self.up_name()?;
        let mut params = Vec::new();
        let mut head = Vec::new();
        if self.at_joined(&T::LParen) {
            self.bump();
            let types = self.seq(T::RParen, false, Self::ty)?;
            match head_params(&types) {
                Some(names) => params = names,
                None => head = types,
            }
        }
        if self.eat(&T::Eq) {
            let body = TypeBody::Alias(self.ty()?);
            return Ok(TypeDef {
                name,
                params,
                head,
                body,
            });
        }
        if !self.at_joined(&T::LBrace) {
            return self.error("`{` or `=`");
        }
        self.bump();
        let constructors = self.nested(|p| {
            let mut constructors = Vec::new();
            while !p.eat(&T::RBrace) {
                let start = p.start();
                let name = p.up_name()?;
                let mut fields = Vec::new();
                if p.at_joined(&T::LParen) {
                    p.bump();
                    fields = p.seq(T::RParen, false, |p| {
                        let start = p.start();
                        let label = if matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Colon {
                            let label = p.name()?;
                            p.bump();
                            Some(label)
                        } else {
                            None
                        };
                        let ty = p.ty()?;
                        Ok(Field {
                            label,
                            ty,
                            span: p.span_from(start),
                        })
                    })?;
                }
                constructors.push(Constructor {
                    name,
                    fields,
                    span: p.span_from(start),
                });
            }
            Ok(constructors)
        })?;
        let body = TypeBody::Custom(constructors);
        Ok(TypeDef {
            name,
            params,
            head,
            body,
        })
    }

    // ---- Statements (§6.10) ----

    /// The longest chain `p0 = p1 = … =` at the current position, as one
    /// pattern (§6.1). Only a name may have `=` after it inside a pattern.
    /// Returns `None`, and moves nothing, when there is no pattern and `=`.
    fn binding_head(&mut self) -> PResult<Option<Pattern>> {
        let mut pats = Vec::new();
        loop {
            let (save, ndiags) = (self.pos, self.diags.len());
            match self.pattern_atom(false) {
                Ok(p) if self.at(&T::Eq) => {
                    self.bump();
                    let is_name = matches!(p.kind, PatternKind::Var(_));
                    pats.push(p);
                    if !is_name {
                        break;
                    }
                }
                _ => {
                    self.pos = save;
                    self.diags.truncate(ndiags);
                    break;
                }
            }
        }
        let Some(mut pattern) = pats.pop() else {
            return Ok(None);
        };
        while let Some(outer) = pats.pop() {
            let PatternKind::Var(name) = outer.kind else {
                unreachable!("only names are chained")
            };
            pattern = Pattern {
                span: outer.span.to(pattern.span),
                kind: PatternKind::Named {
                    name: Ident {
                        name,
                        span: outer.span,
                    },
                    pattern: Box::new(pattern),
                },
            };
        }
        Ok(Some(pattern))
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let start = self.start();
        if self.eat(&T::Bang) {
            let ty = self.named_type()?;
            self.expect(&T::Eq)?;
            let value = self.expr()?;
            let span = self.span_from(start);
            return Ok(Stmt::Env { ty, value, span });
        }
        if self.at_type_params() {
            let (name, type_params, ty) = self.declared_head()?;
            let pattern = Self::declared_pattern(name, ty);
            self.expect(&T::Eq)?;
            let value = self.expr()?;
            let span = self.span_from(start);
            return Ok(Stmt::Bind(Binding {
                pattern,
                value,
                span,
                type_params,
            }));
        }
        if let Some(pattern) = self.binding_head()? {
            let value = self.expr()?;
            let span = self.span_from(start);
            return Ok(Stmt::Bind(Binding {
                pattern,
                value,
                span,
                type_params: Vec::new(),
            }));
        }
        Ok(Stmt::Expr(self.expr()?))
    }

    /// Parts inside `{ }` until the closing `}`, with recovery at breaks.
    fn braced<X>(&mut self, mut part: impl FnMut(&mut Self) -> PResult<X>) -> PResult<Vec<X>> {
        let Some(end) = self.close[self.pos] else {
            return self.error("a closed `{`");
        };
        let end = end as usize;
        self.bump();
        self.nested(|p| {
            let mut parts = Vec::new();
            while p.pos < end {
                let start = p.pos;
                match part(p) {
                    Ok(x) => parts.push(x),
                    Err(d) => {
                        p.diags.push(d);
                        p.recover(start, end);
                    }
                }
            }
            p.pos = end;
            p.bump();
            Ok(parts)
        })
    }

    fn block(&mut self) -> PResult<Block> {
        let start = self.start();
        let errors = self.diags.len();
        let stmts = self.braced(Self::stmt)?;
        let span = self.span_from(start);
        if !matches!(stmts.last(), Some(Stmt::Expr(_))) && self.diags.len() == errors {
            let msg = "a block ends with an expression";
            self.diags.push(Diagnostic::new(Code::BlockEnd, span, msg));
        }
        Ok(Block { stmts, span })
    }

    // ---- Expressions (§6.1) ----

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(0)
    }

    fn binary(&mut self, min: u8) -> PResult<Expr> {
        let start = self.start();
        let mut lhs = self.unary()?;
        while !self.tok().brk {
            let Some(op) = binop(self.kind()) else { break };
            if op.level() <= min {
                break;
            }
            self.bump();
            let rhs = self.binary(op.level())?;
            if op == BinOp::Pipe
                && let ExprKind::Clauses(_) = rhs.kind
            {
                let ExprKind::Clauses(clauses) = rhs.kind else {
                    unreachable!()
                };
                lhs = Expr {
                    kind: ExprKind::Case {
                        subject: Box::new(lhs),
                        clauses,
                    },
                    span: self.span_from(start),
                };
                continue;
            }
            if op == BinOp::Pipe
                && !(is_name(&rhs) || matches!(rhs.kind, ExprKind::Call { .. } | ExprKind::Fn(_)))
            {
                let msg = if matches!(
                    rhs.kind,
                    ExprKind::Binary { .. } | ExprKind::Field { .. } | ExprKind::TupleIndex { .. }
                ) && starts_with_clauses(&rhs)
                {
                    "a match is an operation at level 6, so what comes after the clause block is part of the right side; put the match in a group: `{ e |> { … } } * 10`"
                } else {
                    "the right side of `|>` must be a name, a call, a function, or a clause block"
                };
                self.diags
                    .push(Diagnostic::new(Code::PipeTarget, rhs.span, msg));
            }
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span: self.span_from(start),
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Expr> {
        let start = self.start();
        let op = match self.kind() {
            T::Minus => UnOp::Neg,
            T::MinusDot => UnOp::NegFloat,
            _ => return self.postfix(),
        };
        self.bump();
        let expr = Box::new(self.postfix()?);
        Ok(Expr {
            kind: ExprKind::Unary { op, expr },
            span: self.span_from(start),
        })
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.primary()?;
        loop {
            let kind = if self.at_joined(&T::LParen) {
                self.call(e)?
            } else if self.eat(&T::Dot) {
                match self.kind().clone() {
                    T::Name(_) => ExprKind::Field {
                        target: Box::new(e),
                        name: self.name()?,
                    },
                    T::Int(index) => ExprKind::TupleIndex {
                        target: Box::new(e),
                        index,
                        index_span: self.bump(),
                    },
                    _ => return self.error("a field name or a decimal after `.`"),
                }
            } else {
                return Ok(e);
            };
            e = Expr {
                kind,
                span: self.span_from(start),
            };
        }
    }

    /// The arguments of a call, after its `(`, and the `)`.
    fn args(&mut self) -> PResult<Vec<Arg>> {
        let args = self.seq(T::RParen, true, |p| {
            let label = if matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Colon {
                let label = p.name()?;
                p.bump();
                Some(label)
            } else {
                None
            };
            Ok(Arg {
                label,
                value: p.expr()?,
            })
        })?;
        // §6.6: arguments by position come before arguments by label.
        let mut labeled = false;
        for arg in &args {
            labeled |= arg.label.is_some();
            if labeled && arg.label.is_none() {
                let msg = "an argument by position may not come after an argument by label";
                self.diags
                    .push(Diagnostic::new(Code::ArgOrder, arg.value.span, msg));
            }
        }
        Ok(args)
    }

    fn call(&mut self, callee: Expr) -> PResult<ExprKind> {
        self.bump();
        if !self.at(&T::DotDot) {
            let args = self.args()?;
            let callee = Box::new(callee);
            return Ok(ExprKind::Call { callee, args });
        }
        if !is_name(&callee) {
            let msg = "a record update `C(..v, …)` needs a constructor name before it";
            return Err(Diagnostic::new(Code::BadSpread, callee.span, msg));
        }
        self.bump();
        self.nested(|p| {
            let base = Box::new(p.expr()?);
            let mut fields = Vec::new();
            while !p.eat(&T::RParen) {
                if !p.eat(&T::Comma) && !p.tok().brk {
                    return p.error("`,` or `)`");
                }
                if p.eat(&T::RParen) {
                    break;
                }
                let label = p.name()?;
                p.expect(&T::Colon)?;
                fields.push((label, p.expr()?));
            }
            let constructor = Box::new(callee);
            Ok(ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            })
        })
    }

    fn primary(&mut self) -> PResult<Expr> {
        let start = self.start();
        let kind = match self.kind().clone() {
            T::Int(v) => {
                self.bump();
                ExprKind::Int(v)
            }
            T::Float(v) => {
                self.bump();
                ExprKind::Float(v)
            }
            T::Str(s) => {
                self.bump();
                ExprKind::String(s)
            }
            T::Name(_) | T::UpName(_) => {
                let module = self.qualifier()?;
                let name = match self.kind() {
                    T::Name(_) => self.name()?,
                    _ => self.up_name()?,
                };
                ExprKind::Name { module, name }
            }
            T::Discard(_) => {
                let msg = "a discard binds nothing, so it may not be used as a value";
                return Err(Diagnostic::new(Code::DiscardValue, self.tok().span, msg));
            }
            T::LParen => return self.paren(),
            T::LBracket => self.list()?,
            T::LtLt => {
                self.bump();
                ExprKind::BitArray(self.seq(T::GtGt, true, |p| {
                    let start = p.start();
                    let value = p.expr()?;
                    p.segment(value, start)
                })?)
            }
            T::LBrace => match self.decide() {
                Brace::Block => ExprKind::Block(self.block()?),
                Brace::Clauses => ExprKind::Clauses(self.clauses()?),
                Brace::Record => {
                    let at = self.tok().span;
                    self.diags
                        .push(Diagnostic::new(Code::RecordReserved, at, RECORD_RESERVED));
                    ExprKind::Clauses(self.clauses()?)
                }
            },
            _ => return self.error("an expression"),
        };
        Ok(Expr {
            kind,
            span: self.span_from(start),
        })
    }

    fn segment<X>(&mut self, value: X, start: u32) -> PResult<Segment<X>> {
        let format = if self.eat(&T::Colon) {
            let name = self.name()?;
            let arg = if self.at_joined(&T::LParen) {
                self.bump();
                let arg = self.nested(Self::expr)?;
                self.expect(&T::RParen)?;
                Some(Box::new(arg))
            } else {
                None
            };
            Some(Format { name, arg })
        } else {
            None
        };
        Ok(Segment {
            value,
            format,
            span: self.span_from(start),
        })
    }

    /// `(` at a primary: a function when `->`, or a `{` on the same line, comes
    /// after its `)`; a tuple when not (§6.1, §1.7). In a guard, `->` does not
    /// count.
    fn paren(&mut self) -> PResult<Expr> {
        let start = self.start();
        let is_fn = self.close[self.pos].is_some_and(|c| {
            let next = &self.toks[c as usize + 1];
            next.kind == T::Arrow && !self.guard || next.kind == T::LBrace && !next.brk
        });
        self.bump();
        let kind = if is_fn {
            ExprKind::Fn(Box::new(self.function()?))
        } else {
            ExprKind::Tuple(self.seq(T::RParen, true, Self::expr)?)
        };
        Ok(Expr {
            kind,
            span: self.span_from(start),
        })
    }

    /// After the `(` of a function (§6.8).
    fn function(&mut self) -> PResult<Function> {
        let params = self.seq(T::RParen, true, |p| {
            let env =
                matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Bang && !p.toks[p.pos + 1].brk;
            if !env {
                return Ok(Param::Pattern(p.pattern()?));
            }
            let name = p.name()?;
            p.bump();
            Ok(Param::Env {
                name,
                ty: p.named_type()?,
            })
        })?;
        let ret = if self.eat(&T::Arrow) {
            let (value, err, env) = self.arrow_tail()?;
            Some(RetType { value, err, env })
        } else {
            None
        };
        if !self.at_joined(&T::LBrace) {
            let msg = format!(
                "expected the function body, a `{{` on the same line, found {}",
                self.kind()
            );
            return Err(Diagnostic::new(Code::MissingBody, self.tok().span, msg));
        }
        let body = self.block()?;
        Ok(Function { params, ret, body })
    }

    fn list(&mut self) -> PResult<ExprKind> {
        self.bump();
        self.nested(|p| {
            let mut items = Vec::new();
            let mut rest = None;
            while !p.eat(&T::RBracket) {
                if !items.is_empty() {
                    if !p.eat(&T::Comma) && !p.tok().brk {
                        return p.error("`,` or `]`");
                    }
                    if p.eat(&T::RBracket) {
                        break;
                    }
                }
                if p.at(&T::DotDot) {
                    if items.is_empty() {
                        let msg = "`..rest` comes after the parts of a list";
                        return Err(Diagnostic::new(Code::BadSpread, p.tok().span, msg));
                    }
                    p.bump();
                    rest = Some(Box::new(p.expr()?));
                    p.eat(&T::Comma);
                    p.expect(&T::RBracket)?;
                    break;
                }
                items.push(p.expr()?);
            }
            Ok(ExprKind::List { items, rest })
        })
    }

    /// The clauses of a clause block, from its `{` (§6.12).
    fn clauses(&mut self) -> PResult<Vec<Clause>> {
        self.braced(Self::clause)
    }

    /// `Pattern { "|" Pattern } [ "?" Expr ] "->" Expr`.
    fn clause(&mut self) -> PResult<Clause> {
        let start = self.start();
        self.tuple_type = false;
        let patterns = self.clause_patterns();
        let note = self.tuple_type;
        let noted = |mut d: Diagnostic| {
            if note {
                d.message.push_str(TUPLE_NOTE);
            }
            d
        };
        let patterns = patterns.map_err(noted)?;
        let (guard, body) = self.clause_tail().map_err(noted)?;
        Ok(Clause {
            patterns,
            guard,
            body,
            span: self.span_from(start),
        })
    }

    fn clause_patterns(&mut self) -> PResult<Vec<Pattern>> {
        let mut patterns = vec![self.pattern()?];
        while self.eat(&T::Pipe) {
            patterns.push(self.pattern()?);
        }
        Ok(patterns)
    }

    fn clause_tail(&mut self) -> PResult<(Option<Expr>, Expr)> {
        let guard = if self.eat(&T::Question) {
            self.guard = true;
            let guard = self.expr();
            self.guard = false;
            Some(guard?)
        } else {
            None
        };
        self.expect(&T::Arrow)?;
        let body = self.expr()?;
        Ok((guard, body))
    }

    // ---- Deciding a `{` (§6.1) ----

    /// Whether the `{` at the current token begins a block or a clause
    /// block. Each `{` is decided once; the answer is kept by its index.
    fn decide(&mut self) -> Brace {
        let at = self.pos;
        if self.cache
            && let Some(&b) = self.decided.get(&at)
        {
            return b;
        }
        self.decisions += 1;
        let ndiags = self.diags.len();
        let b = self.nested(|p| Ok(p.probe(at))).unwrap_or(Brace::Block);
        self.pos = at;
        self.diags.truncate(ndiags);
        self.decided.insert(at, b);
        b
    }

    /// The tests of §6.1 on the first statement of the `{` at `at`. Moves
    /// `pos`, which `decide` puts back.
    fn probe(&mut self, at: usize) -> Brace {
        let Some(end) = self.close[at] else {
            return Brace::Block;
        };
        let end = end as usize;
        let s = at + 1;
        self.pos = s;
        // 0: `{}`.
        if s == end {
            return Brace::Clauses;
        }
        // 1: `name(A, …) =` or `:`.
        if self.at_type_params() {
            return Brace::Block;
        }
        // 2: the longest pattern, and what follows it.
        self.pos = s;
        let mut chain = 0;
        while matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Eq {
            self.pos += 2;
            chain += 1;
        }
        let last = self.pos;
        let p_end = match self.pattern_in(false) {
            Ok(_) => Some(self.pos),
            Err(_) if chain > 0 => Some(last - 1),
            Err(_) => None,
        };
        let last_is_tuple = p_end.is_some_and(|e| {
            e > last
                && self.toks[last].kind == T::LParen
                && self.close[last].is_some_and(|c| c as usize + 1 == e)
        });
        if let Some(e) = p_end {
            self.pos = e;
            let follows = self.kind().clone();
            if matches!(follows, T::Pipe | T::Question | T::Arrow) {
                if chain > 0 && last_is_tuple && follows == T::Arrow {
                    self.bump();
                    if self.arrow_tail().is_ok() && self.at_joined(&T::LBrace) {
                        return Brace::Block;
                    }
                }
                return Brace::Clauses;
            }
            // 3: a binding.
            if chain > 0 || follows == T::Eq {
                return Brace::Block;
            }
        }
        self.pos = s;
        // 3: an environment statement.
        if self.at(&T::Bang) {
            return Brace::Block;
        }
        // 4: `{ name: … }`, kept for records.
        if matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Colon {
            return Brace::Record;
        }
        // 5: a clause whose pattern is wrong.
        if self.arrow_before_break(s, end) {
            self.pos = s;
            match self.expr() {
                Err(_) => return Brace::Clauses,
                Ok(_) if self.at(&T::Arrow) => return Brace::Clauses,
                Ok(_) => {}
            }
        }
        // 6.
        Brace::Block
    }

    /// A `->` outside brackets from `s` to the next break or to `end`.
    fn arrow_before_break(&self, s: usize, end: usize) -> bool {
        let mut i = s;
        while i < end {
            if i > s && self.toks[i].brk {
                return false;
            }
            if self.toks[i].kind == T::Arrow {
                return true;
            }
            i = match self.close[i] {
                Some(c) => c as usize + 1,
                None => i + 1,
            };
        }
        false
    }

    // ---- Patterns (§7.1) ----

    fn pattern(&mut self) -> PResult<Pattern> {
        self.pattern_in(false)
    }

    /// `segment` is set at the top of a bit array segment, where `:` begins a
    /// format and not a type (§8.1).
    fn pattern_in(&mut self, segment: bool) -> PResult<Pattern> {
        if !(matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Eq) {
            return self.pattern_atom(segment);
        }
        let start = self.start();
        let name = self.name()?;
        self.bump();
        let pattern = Box::new(self.pattern_in(segment)?);
        Ok(Pattern {
            kind: PatternKind::Named { name, pattern },
            span: self.span_from(start),
        })
    }

    fn binder(&mut self) -> PResult<Binder> {
        let (name, discard) = match self.kind() {
            T::Name(n) => (n.clone(), false),
            T::Discard(n) => (n.clone(), true),
            _ => return self.error("a name or a discard"),
        };
        Ok(Binder {
            name,
            discard,
            span: self.bump(),
        })
    }

    /// `PrefixPat | Atom`.
    fn pattern_atom(&mut self, segment: bool) -> PResult<Pattern> {
        let start = self.start();
        let kind = match self.kind().clone() {
            T::Name(_) if self.at_qualified() => self.constructor_pattern()?,
            T::Name(_) | T::Discard(_) => {
                let binder = self.binder()?;
                if !segment && self.eat(&T::Colon) {
                    self.tuple_type |= self.at(&T::LParen);
                    PatternKind::Typed {
                        binder,
                        ty: self.ty()?,
                    }
                } else if binder.discard {
                    PatternKind::Discard(binder.name)
                } else {
                    PatternKind::Var(binder.name)
                }
            }
            T::UpName(_) => self.constructor_pattern()?,
            T::Int(value) => {
                self.bump();
                PatternKind::Int {
                    negative: false,
                    value,
                }
            }
            T::Float(value) => {
                self.bump();
                PatternKind::Float {
                    negative: false,
                    value,
                }
            }
            T::Minus | T::MinusDot => {
                let int = self.at(&T::Minus);
                self.bump();
                match self.kind().clone() {
                    T::Int(value) if int => {
                        self.bump();
                        PatternKind::Int {
                            negative: true,
                            value,
                        }
                    }
                    T::Float(value) if !int => {
                        self.bump();
                        PatternKind::Float {
                            negative: true,
                            value,
                        }
                    }
                    _ => {
                        let msg = "in a pattern, `-` comes before an int literal and `-.` before a float literal";
                        return Err(Diagnostic::new(Code::BadPattern, self.tok().span, msg));
                    }
                }
            }
            T::Str(prefix) => {
                self.bump();
                if self.eat(&T::LtGt) {
                    PatternKind::Prefix {
                        prefix,
                        rest: self.binder()?,
                    }
                } else {
                    PatternKind::String(prefix)
                }
            }
            T::LParen => {
                self.bump();
                PatternKind::Tuple(self.seq(T::RParen, true, Self::pattern)?)
            }
            T::LBracket => self.list_pattern()?,
            T::LtLt => {
                self.bump();
                PatternKind::BitArray(self.seq(T::GtGt, true, |p| {
                    let start = p.start();
                    let value = p.pattern_in(true)?;
                    p.segment(value, start)
                })?)
            }
            _ => return self.error("a pattern"),
        };
        Ok(Pattern {
            kind,
            span: self.span_from(start),
        })
    }

    /// `[m.]C[(p, …, ..)]`: the parts and `..` (§7.1).
    fn constructor_pattern(&mut self) -> PResult<PatternKind> {
        let module = self.qualifier()?;
        let name = self.up_name()?;
        let mut args = Vec::new();
        let mut spread = false;
        if self.at_joined(&T::LParen) {
            self.bump();
            self.nested(|p| {
                if p.eat(&T::RParen) {
                    return Ok(());
                }
                loop {
                    if p.eat(&T::DotDot) {
                        spread = true;
                        p.expect(&T::RParen)?;
                        return Ok(());
                    }
                    args.push(p.pattern()?);
                    if p.eat(&T::RParen) || p.eat(&T::Comma) && p.eat(&T::RParen) {
                        return Ok(());
                    }
                    let sep = p.toks[p.pos - 1].kind == T::Comma || p.tok().brk;
                    if !sep {
                        return p.error("`,` or `)`");
                    }
                }
            })?;
        }
        Ok(PatternKind::Constructor {
            module,
            name,
            args,
            spread,
        })
    }

    fn list_pattern(&mut self) -> PResult<PatternKind> {
        self.bump();
        self.nested(|p| {
            let mut items = Vec::new();
            let mut rest = None;
            if p.eat(&T::RBracket) {
                return Ok(PatternKind::List { items, rest });
            }
            loop {
                if p.eat(&T::DotDot) {
                    let binder = match p.kind() {
                        T::Name(_) | T::Discard(_) => Some(p.binder()?),
                        _ => None,
                    };
                    rest = Some(binder);
                    p.expect(&T::RBracket)?;
                    break;
                }
                items.push(p.pattern()?);
                if p.eat(&T::RBracket) || p.eat(&T::Comma) && p.eat(&T::RBracket) {
                    break;
                }
                let sep = p.toks[p.pos - 1].kind == T::Comma || p.tok().brk;
                if !sep {
                    return p.error("`,` or `]`");
                }
            }
            Ok(PatternKind::List { items, rest })
        })
    }

    // ---- Types (§3) ----

    fn ty(&mut self) -> PResult<TypeExpr> {
        self.type_list(T::Pipe, Self::type_inter, TypeKind::Union)
    }

    fn type_inter(&mut self) -> PResult<TypeExpr> {
        self.type_list(T::Amp, Self::type_diff, TypeKind::Inter)
    }

    fn type_diff(&mut self) -> PResult<TypeExpr> {
        self.type_list(T::Minus, Self::type_prim, TypeKind::Diff)
    }

    fn type_list(
        &mut self,
        op: T,
        part: fn(&mut Self) -> PResult<TypeExpr>,
        make: fn(Vec<TypeExpr>) -> TypeKind,
    ) -> PResult<TypeExpr> {
        let start = self.start();
        let first = part(self)?;
        if !self.at_joined(&op) {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.at_joined(&op) {
            self.bump();
            parts.push(part(self)?);
        }
        Ok(TypeExpr {
            kind: make(parts),
            span: self.span_from(start),
        })
    }

    /// The part of a function type or a function after `->` (§3, §6.8).
    #[allow(clippy::type_complexity)]
    fn arrow_tail(&mut self) -> PResult<(TypeExpr, Option<TypeExpr>, Option<TypeExpr>)> {
        let value = self.ty()?;
        let err = if self.eat(&T::Question) {
            Some(self.ty()?)
        } else {
            None
        };
        let env = if self.at_joined(&T::Bang) {
            self.bump();
            Some(self.ty()?)
        } else {
            None
        };
        // §3: this `->` is the nearest one, and it has taken what it can.
        if self.at(&T::Question) || self.at_joined(&T::Bang) {
            let msg = format!(
                "the nearest `->` cannot take this {}: it already has one, or `!` came first; put the inner function type in `{{ }}`",
                self.kind()
            );
            return Err(Diagnostic::new(Code::ArrowTail, self.tok().span, msg));
        }
        Ok((value, err, env))
    }

    fn named_type(&mut self) -> PResult<TypeExpr> {
        let start = self.start();
        if !(self.at_qualified() || matches!(self.kind(), T::UpName(_))) {
            return self.error("a named type");
        }
        let module = self.qualifier()?;
        let name = self.up_name()?;
        let mut args = Vec::new();
        if self.at_joined(&T::LParen) {
            self.bump();
            args = self.seq(T::RParen, false, Self::ty)?;
        }
        Ok(TypeExpr {
            kind: TypeKind::Named { module, name, args },
            span: self.span_from(start),
        })
    }

    /// Only integer atoms and brace groups can follow a literal type's minus.
    fn type_int_part(&mut self) -> PResult<num_bigint::BigUint> {
        match self.kind().clone() {
            T::Int(n) => {
                self.bump();
                Ok(n)
            }
            T::LBrace => {
                self.bump();
                let n = self.nested(Self::type_int_part)?;
                if self.tok().brk {
                    return Err(Diagnostic::new(
                        Code::TypeGroupBreak,
                        self.tok().span,
                        "a line break directly inside a `{ }` that groups a type is a compile error",
                    ));
                }
                self.expect(&T::RBrace)?;
                Ok(n)
            }
            T::Float(_) => Err(Diagnostic::new(
                Code::Unexpected,
                self.tok().span,
                "Float has no literal types",
            )),
            _ => self.error("an integer literal after `-` in a type"),
        }
    }

    fn type_prim(&mut self) -> PResult<TypeExpr> {
        let start = self.start();
        let kind = match self.kind().clone() {
            T::Int(n) => {
                self.bump();
                TypeKind::Lit(LitValue::Int(n.into()))
            }
            T::Str(s) => {
                self.bump();
                TypeKind::Lit(LitValue::Str(s))
            }
            T::Minus => {
                self.bump();
                TypeKind::Lit(LitValue::Int(-num_bigint::BigInt::from(
                    self.type_int_part()?,
                )))
            }
            T::Float(_) => {
                return Err(Diagnostic::new(
                    Code::Unexpected,
                    self.tok().span,
                    "Float has no literal types",
                ));
            }
            T::LParen => {
                let is_fn = self.close[self.pos]
                    .is_some_and(|c| self.toks[c as usize + 1].kind == T::Arrow);
                self.bump();
                if !is_fn {
                    TypeKind::Tuple(self.seq(T::RParen, true, Self::ty)?)
                } else {
                    let params = self.seq(T::RParen, true, |p| {
                        let label = if matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Colon {
                            let label = p.name()?;
                            p.bump();
                            Some(label)
                        } else {
                            None
                        };
                        Ok(FnParamType { label, ty: p.ty()? })
                    })?;
                    self.expect(&T::Arrow)?;
                    let (ret, err, env) = self.arrow_tail()?;
                    TypeKind::Fn {
                        params,
                        ret: Box::new(ret),
                        err: err.map(Box::new),
                        env: env.map(Box::new),
                    }
                }
            }
            T::LBrace => {
                self.bump();
                let inner = self.nested(Self::ty)?;
                if self.tok().brk {
                    let msg = "a line break directly inside a `{ }` that groups a type is a compile error";
                    return Err(Diagnostic::new(Code::TypeGroupBreak, self.tok().span, msg));
                }
                self.expect(&T::RBrace)?;
                return Ok(inner);
            }
            T::Name(_) if self.at_qualified() => return self.named_type(),
            T::UpName(_) => return self.named_type(),
            // gave an error here for `a.x`.
            T::Name(_)
                if (*self.nth(1) == T::Dot && matches!(self.nth(2), T::Name(_))
                    || *self.nth(1) == T::LParen && !self.toks[self.pos + 1].brk) =>
            {
                let module = if *self.nth(1) == T::Dot {
                    let module = self.name()?;
                    self.bump();
                    Some(module)
                } else {
                    None
                };
                let name = self.name()?;
                if self.at_joined(&T::LParen) {
                    self.bump();
                    let args = self.seq(T::RParen, true, Self::ty)?;
                    TypeKind::Call { module, name, args }
                } else {
                    TypeKind::ValueOf { module, name }
                }
            }
            T::Name(_) => TypeKind::ValueOf {
                module: None,
                name: self.name()?,
            },
            T::Discard(d) if d == "_" => {
                self.bump();
                TypeKind::Hole
            }
            _ => return self.error("a type"),
        };
        Ok(TypeExpr {
            kind,
            span: self.span_from(start),
        })
    }
}
