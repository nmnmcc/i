//! The parser of syntax three (docs/design/syntax-3.md §3, rules P1 to P13,
//! with the services amendment). It reads the tokens of [`crate::lexer::lex3`].

use super::ast::*;
use crate::lexer::{DocComment, generics_end, lex3, pair_brackets};
use crate::span::Span;
use crate::token::{StringPart, Token, TokenKind as T};
use crate::{Code, Diagnostic};

type PResult<X> = Result<X, Diagnostic>;

/// Parses a syntax-three module. The diagnostics are sorted by position.
pub fn parse_module(source: &str) -> (Module, Vec<Diagnostic>) {
    let lexed = lex3(source);
    let mut parser = Parser {
        source,
        toks: &lexed.tokens,
        close: &lexed.close,
        docs: &lexed.docs,
        doc_pos: 0,
        pos: 0,
        diags: Vec::new(),
    };
    let module = parser.module();
    let mut diags = lexed.diagnostics.clone();
    diags.append(&mut parser.diags);
    diags.sort_by_key(|d| d.span.start);
    diags.dedup();
    (module, diags)
}

fn parse_with<X>(
    source: &str,
    f: impl FnOnce(&mut Parser) -> PResult<X>,
) -> Result<X, Vec<Diagnostic>> {
    let lexed = lex3(source);
    let mut parser = Parser {
        source,
        toks: &lexed.tokens,
        close: &lexed.close,
        docs: &lexed.docs,
        doc_pos: 0,
        pos: 0,
        diags: Vec::new(),
    };
    let result = f(&mut parser).and_then(|x| {
        parser.expect(&T::Eof)?;
        Ok(x)
    });
    let mut diags = lexed.diagnostics.clone();
    diags.append(&mut parser.diags);
    match result {
        Ok(x) if diags.is_empty() => Ok(x),
        Ok(_) => Err(diags),
        Err(d) => {
            diags.push(d);
            Err(diags)
        }
    }
}

/// Parses one expression.
pub fn parse_expr(source: &str) -> Result<Expr, Vec<Diagnostic>> {
    parse_with(source, |p| p.expr())
}

/// Parses one type.
pub fn parse_type(source: &str) -> Result<Type, Vec<Diagnostic>> {
    parse_with(source, |p| p.ty())
}

/// Parses one pattern (`Part { "=" Part }`).
pub fn parse_pattern(source: &str) -> Result<Pattern, Vec<Diagnostic>> {
    parse_with(source, |p| p.pattern())
}

struct Parser<'a> {
    source: &'a str,
    toks: &'a [Token],
    close: &'a [Option<u32>],
    docs: &'a [DocComment],
    doc_pos: usize,
    pos: usize,
    diags: Vec<Diagnostic>,
}

fn binop(kind: &T) -> Option<BinOp> {
    Some(match kind {
        T::PipePipe => BinOp::Or,
        T::AmpAmp => BinOp::And,
        T::EqEq => BinOp::Eq,
        T::NotEq => BinOp::NotEq,
        T::Lt => BinOp::Lt,
        T::LtEq => BinOp::LtEq,
        T::Gt => BinOp::Gt,
        T::GtEq => BinOp::GtEq,
        T::LtDot => BinOp::LtFloat,
        T::LtEqDot => BinOp::LtEqFloat,
        T::GtDot => BinOp::GtFloat,
        T::GtEqDot => BinOp::GtEqFloat,
        T::LtGt => BinOp::Concat,
        T::Plus => BinOp::Add,
        T::Minus => BinOp::Sub,
        T::PlusDot => BinOp::AddFloat,
        T::MinusDot => BinOp::SubFloat,
        T::Star => BinOp::Mul,
        T::Slash => BinOp::Div,
        T::Percent => BinOp::Rem,
        T::StarDot => BinOp::MulFloat,
        T::SlashDot => BinOp::DivFloat,
        _ => return None,
    })
}

/// Whether `e` may be the right side of `|>` (P11): a name, a call, or a
/// chain literal.
pub fn pipe_target(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name { .. } | ExprKind::Call { .. } | ExprKind::Chain(_) => true,
        ExprKind::Field { target, .. } => pipe_target_path(target),
        _ => false,
    }
}

fn pipe_target_path(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name { .. } => true,
        ExprKind::Field { target, .. } => pipe_target_path(target),
        _ => false,
    }
}

/// An item with a key, a positional item, or a spread or rest, for the order
/// rule of §2.3.
enum ItemOrder {
    Pos,
    Key,
    Other,
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
        let mut msg = format!("expected {expected}, found {found}");
        if self.at(&T::Eq)
            && let Some(open) = self.pos.checked_sub(1).and_then(|close| {
                self.close[..self.pos]
                    .iter()
                    .rposition(|end| *end == Some(close as u32))
            })
            && spaced_binding_call(self.source, self.toks, open, self.pos)
        {
            msg.push_str("; ");
            msg.push_str(ONE_BINDING_HINT);
        }
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
                Ok(Ident::new(name, self.bump()))
            }
            _ => self.error("a name"),
        }
    }

    fn up_name(&mut self) -> PResult<Ident> {
        match self.kind() {
            T::UpName(n) => {
                let name = n.clone();
                Ok(Ident::new(name, self.bump()))
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

    /// `name =`: the start of a labelled (key) item (P3).
    fn at_key(&self) -> bool {
        matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Eq
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
        let mut parts = Vec::new();
        loop {
            if self.at(&close) {
                if parts.is_empty() && !empty {
                    return self.error("a part");
                }
                break;
            }
            parts.push(part(self)?);
            if self.at(&close) {
                break;
            }
            if !self.eat(&T::Comma) && !self.tok().brk {
                return self.error(&format!("`,` or `{}`", close.text()));
            }
        }
        self.bump();
        Ok(parts)
    }

    /// §2.3: an explicit positional item may not follow a labelled item.
    fn check_order<X>(&mut self, items: &[X], order: impl Fn(&X) -> (ItemOrder, Span)) {
        let mut keyed = false;
        for item in items {
            match order(item) {
                (ItemOrder::Key, _) => keyed = true,
                (ItemOrder::Pos, span) if keyed => {
                    let msg = "an item by position may not come after an item by label";
                    self.diags.push(Diagnostic::new(Code::ArgOrder, span, msg));
                }
                _ => {}
            }
        }
    }

    /// After an error in a part that began at `start`, skips to the next break
    /// at the same level, or to the closing token at `end`.
    fn recover(&mut self, start: usize, end: usize) {
        let failed = self.pos;
        let top = end + 1 == self.toks.len();
        let resumes = |t: &Token| t.brk && !(top && matches!(t.kind, T::LBrace | T::DotDot));
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

    // ---- Module ----

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
        while self.at_header_group() {
            let start = self.pos;
            let from = self.start();
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
            let result = if is_import {
                imports = true;
                self.import().map(|items| module.imports = items)
            } else {
                exports = true;
                self.export().map(|items| module.exports = items)
            };
            if let Err(d) = result {
                self.diags.push(d);
                self.recover(start, eof);
            }
        }
        while !self.at(&T::Eof) {
            let start = self.pos;
            match self.item() {
                Ok(item) => module.items.push(item),
                Err(d) => {
                    self.diags.push(d);
                    self.recover(start, eof);
                    if self.pos == start {
                        self.bump();
                    }
                }
            }
        }
        module
    }

    /// A `(` whose `)` is not followed by `=`, `->`, `<-` or a `{` on its
    /// line: the imports or the exports at the start of a file.
    fn at_header_group(&self) -> bool {
        self.at(&T::LParen)
            && self.close[self.pos].is_some_and(|c| {
                let next = &self.toks[c as usize + 1];
                !matches!(next.kind, T::Eq | T::Arrow | T::LArrow)
                    && !(next.kind == T::LBrace && !next.brk)
            })
    }

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

    fn import(&mut self) -> PResult<Vec<Import>> {
        self.expect(&T::LParen)?;
        self.seq(T::RParen, true, |p| {
            let start = p.start();
            let alias = if p.at_key() {
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
            Ok(Import {
                alias,
                names,
                path,
                span: Sp(span),
            })
        })
    }

    fn export(&mut self) -> PResult<Vec<Export>> {
        self.expect(&T::LParen)?;
        self.seq(T::RParen, true, |p| {
            let start = p.start();
            if matches!(p.kind(), T::Name(_)) {
                let name = p.name()?;
                let span = name.span;
                return Ok(Export {
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
            Ok(Export {
                name,
                constructors,
                span: Sp(p.span_from(start)),
            })
        })
    }

    fn item(&mut self) -> PResult<Item> {
        let mut doc = self.take_docs();
        let start = self.start();
        let mut attributes = Vec::new();
        while self.at(&T::At) {
            attributes.push(self.attribute()?);
            doc.extend(self.take_docs());
        }
        if self.at_header_group() {
            let close = self.close[self.pos].unwrap() as usize;
            let span = Span::new(self.start(), self.toks[close].span.end);
            let msg = "a group in `( )` that is not followed by `=` is the imports or the exports, which come first in a file";
            self.pos = (close + 1).min(self.toks.len() - 1);
            return Err(Diagnostic::new(Code::HeaderGroup, span, msg));
        }
        let kind = if matches!(self.kind(), T::UpName(_)) {
            // P12.
            ItemKind::TypeDef(self.type_def()?)
        } else if self.at_macro_item() {
            let module = if *self.nth(1) == T::Dot {
                let module = self.name()?;
                self.bump();
                Some(module)
            } else {
                None
            };
            let name = self.name()?;
            let args = self.data()?;
            ItemKind::Macro { module, name, args }
        } else {
            let bstart = self.start();
            self.removed_type_params()?;
            match self.binding_head()? {
                Some(pattern) => {
                    let value = self.expr()?;
                    ItemKind::Bind(Binding {
                        pattern,
                        value,
                        span: Sp(self.span_from(bstart)),
                    })
                }
                None if matches!(self.kind(), T::Name(_)) && *self.nth(1) == T::Colon => {
                    let name = self.name()?;
                    self.bump();
                    let ty = self.annot()?;
                    ItemKind::Decl { name, ty }
                }
                None => {
                    return self
                        .error("an item: a type definition, or a binding `pattern = value`");
                }
            }
        };
        Ok(Item {
            doc,
            attributes,
            kind,
            span: Sp(self.span_from(start)),
        })
    }

    /// `[name .] name (` … `)` with no `=` after the `)`: a macro item.
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
        let args = if self.at_joined(&T::LParen) {
            Some(self.data()?)
        } else {
            None
        };
        Ok(Attribute {
            module,
            name,
            args,
            span: Sp(self.span_from(start)),
        })
    }

    /// §2.8: `Name [ (head) ] = right side`.
    fn type_def(&mut self) -> PResult<TypeDef> {
        let name = self.up_name()?;
        let mut head = Vec::new();
        if self.at_joined(&T::LParen) {
            self.bump();
            head = self.seq(T::RParen, false, Self::ty)?;
        }
        if self.at_joined(&T::LBrace) {
            let msg = "a type definition is `Name = right side`; the constructor list `Name { … }` is removed: write each constructor as its own labelled type, `C = (…)`, and the sum as `Name = C1 | C2`";
            return Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg));
        }
        self.expect(&T::Eq)?;
        let first = self.kind().clone();
        let ty = self.ty()?;
        let rhs = match (first, ty.kind) {
            (
                T::UpName(n),
                TypeKind::Named {
                    module: None, args, ..
                },
            ) if n == "Opaque" && args.is_empty() => TypeRhs::Opaque,
            (T::LParen, TypeKind::Data(items)) => TypeRhs::Data(items),
            (_, kind) => TypeRhs::Type(Type {
                kind,
                span: ty.span,
            }),
        };
        Ok(TypeDef { name, head, rhs })
    }

    // ---- Statements ----

    /// P6(d): pattern parts while each is followed by `=`. Returns `None`,
    /// and moves nothing, when no part is followed by `=`.
    fn binding_head(&mut self) -> PResult<Option<Pattern>> {
        let start = self.start();
        let mut parts = Vec::new();
        loop {
            let (save, ndiags, ndoc) = (self.pos, self.diags.len(), self.doc_pos);
            match self.pattern_part(false) {
                Ok(p) if self.at(&T::Eq) => {
                    self.bump();
                    parts.push(p);
                }
                _ => {
                    self.pos = save;
                    self.diags.truncate(ndiags);
                    self.doc_pos = ndoc;
                    break;
                }
            }
        }
        Ok(match parts.len() {
            0 => None,
            1 => parts.pop(),
            _ => {
                let end = self.toks[self.pos - 2].span.end;
                Some(Pattern::new(PatternKind::And(parts), Span::new(start, end)))
            }
        })
    }

    /// P6: `name ( UpName … )` followed by `=` or `:` is the removed
    /// binding head with type parameters.
    fn removed_type_params(&self) -> PResult<()> {
        if !matches!(self.kind(), T::Name(_)) {
            return Ok(());
        }
        let open = self.pos + 1;
        if self.toks[open].kind != T::LParen || self.toks[open].brk {
            return Ok(());
        }
        let Some(close) = self.close[open] else {
            return Ok(());
        };
        let close = close as usize;
        let inside = &self.toks[open + 1..close];
        let ups = !inside.is_empty()
            && inside
                .iter()
                .all(|t| matches!(t.kind, T::UpName(_) | T::Comma));
        if ups && matches!(self.toks[close + 1].kind, T::Eq | T::Colon) {
            let msg = "type parameters are written on the function, not on the name: `id = <A>(x: A) -> A { x }`, or in a written type, `id: <A>(A) -> A`";
            let span = Span::new(self.toks[open].span.start, self.toks[close].span.end);
            return Err(Diagnostic::new(Code::Unexpected, span, msg));
        }
        Ok(())
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let start = self.start();
        self.removed_type_params()?;
        if let Some(pattern) = self.binding_head()? {
            let value = self.expr()?;
            return Ok(Stmt::Bind(Binding {
                pattern,
                value,
                span: Sp(self.span_from(start)),
            }));
        }
        Ok(Stmt::Expr(self.expr()?))
    }

    fn block(&mut self) -> PResult<Block> {
        let start = self.start();
        let Some(end) = self.close[self.pos] else {
            return self.error("a closed `{`");
        };
        let end = end as usize;
        let errors = self.diags.len();
        self.bump();
        let mut stmts = Vec::new();
        while self.pos < end {
            let at = self.pos;
            match self.stmt() {
                Ok(s) => stmts.push(s),
                Err(d) => {
                    self.diags.push(d);
                    self.recover(at, end);
                    if self.pos == at {
                        self.pos = end;
                    }
                }
            }
        }
        self.pos = end;
        self.bump();
        let span = self.span_from(start);
        if !matches!(stmts.last(), Some(Stmt::Expr(_))) && self.diags.len() == errors {
            let msg = if stmts.is_empty() {
                "a block has at least an expression; `{}` and the empty match are removed: use `core.never`"
            } else {
                "a block ends with an expression"
            };
            self.diags.push(Diagnostic::new(Code::BlockEnd, span, msg));
        }
        Ok(Block {
            stmts,
            span: Sp(span),
        })
    }

    // ---- Expressions ----

    /// `Provide = Pipe { "<-" Postfix }`: the weakest level, left
    /// associative.
    fn expr(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut lhs = self.pipe()?;
        while self.eat(&T::LArrow) {
            let services = self.postfix()?;
            if let ExprKind::Data(items) = &services.kind
                && let Some(DItem::Label(label, _)) =
                    items.iter().find(|i| matches!(i, DItem::Label(..)))
            {
                let msg = "provided services have no labels: each is keyed by its own type";
                self.diags
                    .push(Diagnostic::new(Code::Unexpected, label.span.0, msg));
            }
            lhs = Expr::new(
                ExprKind::Provide {
                    target: Box::new(lhs),
                    services: Box::new(services),
                },
                self.span_from(start),
            );
        }
        Ok(lhs)
    }

    /// P11: `Pipe = Binary { "|>" Binary }`, left associative.
    fn pipe(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut lhs = self.binary(0)?;
        while self.eat(&T::PipeGt) {
            let rhs = self.binary(0)?;
            if !pipe_target(&rhs) {
                let msg = "the right side of `|>` must be a name, a call, or a chain of functions; `|>` is the weakest operator, so put anything else in a group: `{ x |> g } == 2`";
                self.diags
                    .push(Diagnostic::new(Code::PipeTarget, rhs.span.0, msg));
            }
            lhs = Expr::new(
                ExprKind::Pipe {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                self.span_from(start),
            );
        }
        Ok(lhs)
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
            lhs = Expr::new(
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                self.span_from(start),
            );
        }
        if self.at(&T::Pipe) {
            let msg =
                "`|` joins only function literals: it may follow only the `}` of a function body";
            return Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg));
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
        Ok(Expr::new(
            ExprKind::Unary { op, expr },
            self.span_from(start),
        ))
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let start = self.start();
        let mut e = self.primary()?;
        loop {
            let kind = if self.at_joined(&T::LParen) {
                // P2.
                ExprKind::Call {
                    callee: Box::new(e),
                    args: self.data()?,
                }
            } else if self.eat(&T::Dot) {
                match self.kind().clone() {
                    T::Name(_) => ExprKind::Field {
                        target: Box::new(e),
                        name: self.name()?,
                    },
                    T::Int(index) => {
                        self.bump();
                        ExprKind::Index {
                            target: Box::new(e),
                            index,
                        }
                    }
                    _ => return self.error("a label or a decimal after `.`"),
                }
            } else {
                return Ok(e);
            };
            e = Expr::new(kind, self.span_from(start));
        }
    }

    /// `Data`, from its `(` (P3).
    fn data(&mut self) -> PResult<Vec<DItem>> {
        self.expect(&T::LParen)?;
        let items = self.seq(T::RParen, true, |p| {
            if p.eat(&T::DotDot) {
                return Ok(DItem::Spread(p.expr()?));
            }
            if p.at_key() {
                let label = p.name()?;
                p.bump();
                return Ok(DItem::Label(label, p.expr()?));
            }
            if matches!(p.kind(), T::Name(_)) && *p.nth(1) == T::Colon {
                let msg = "a label in data is written `label = value`; `:` only gives a type to a pattern";
                return Err(Diagnostic::new(Code::Unexpected, p.toks[p.pos + 1].span, msg));
            }
            Ok(DItem::Pos(p.expr()?))
        })?;
        self.check_order(&items, |i| match i {
            DItem::Pos(e) => (ItemOrder::Pos, e.span.0),
            DItem::Label(n, _) => (ItemOrder::Key, n.span.0),
            DItem::Spread(e) => (ItemOrder::Other, e.span.0),
        });
        Ok(items)
    }

    fn interpolation(&mut self, parts: Vec<StringPart>) -> PResult<ExprKind> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                StringPart::Text(s, _) if !s.is_empty() => out.push(InterpPart::Text(s)),
                StringPart::Text(_, _) => {}
                StringPart::Expr { tokens, .. } => {
                    let close = pair_brackets(&tokens, &mut self.diags);
                    let mut child = Parser {
                        source: self.source,
                        toks: &tokens,
                        close: &close,
                        docs: &[],
                        doc_pos: 0,
                        pos: 0,
                        diags: Vec::new(),
                    };
                    let expression = child.expr()?;
                    child.expect(&T::Eof)?;
                    self.diags.extend(child.diags);
                    out.push(InterpPart::Expr(expression));
                }
            }
        }
        Ok(ExprKind::Interp(out))
    }

    /// P1: a `(` that begins a primary is a function's parameter group when
    /// its `)` is followed by `->`, by `{` on the same line, or by a service
    /// group `<- ( … )` that is followed on the same line by `->` or `{`.
    fn at_function(&self, open: usize) -> bool {
        let Some(close) = self.close[open] else {
            return false;
        };
        let next = &self.toks[close as usize + 1];
        match next.kind {
            T::Arrow => true,
            T::LBrace => !next.brk,
            T::LArrow => {
                let group = close as usize + 2;
                let end = match self.toks[group].kind {
                    T::LParen => self.close[group].map(|c| c as usize),
                    // `<- N` is not a service pattern; `function` says so.
                    T::UpName(_) => Some(group),
                    _ => None,
                };
                end.is_some_and(|c| {
                    let after = &self.toks[(c + 1).min(self.toks.len() - 1)];
                    !after.brk && matches!(after.kind, T::Arrow | T::LBrace)
                })
            }
            _ => false,
        }
    }

    fn at_generics(&self) -> bool {
        generics_end(self.toks, self.pos).is_some()
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
            T::Interpolated(parts) => {
                self.bump();
                self.interpolation(parts)?
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
            T::LParen if self.at_function(self.pos) => self.chain()?,
            T::Lt if self.at_generics() => self.chain()?,
            T::LParen => ExprKind::Data(self.data()?),
            T::LBracket => self.list()?,
            T::LtLt => {
                self.bump();
                ExprKind::BitArray(self.seq(T::GtGt, true, |p| {
                    let value = p.expr()?;
                    p.segment(value)
                })?)
            }
            T::LBrace => ExprKind::Block(self.block()?),
            _ => return self.error("an expression"),
        };
        Ok(Expr::new(kind, self.span_from(start)))
    }

    fn segment<X>(&mut self, value: X) -> PResult<Segment<X>> {
        let format = if self.eat(&T::Colon) {
            let name = self.name()?;
            let arg = if self.at_joined(&T::LParen) {
                self.bump();
                let arg = self.expr()?;
                self.expect(&T::RParen)?;
                Some(Box::new(arg))
            } else {
                None
            };
            Some(Format { name, arg })
        } else {
            None
        };
        Ok(Segment { value, format })
    }

    /// P4: `Branches = Function { "|" Function }`.
    fn chain(&mut self) -> PResult<ExprKind> {
        let mut branches = vec![self.function()?];
        while self.at(&T::Pipe) {
            let bar = self.bump();
            let begins = self.at(&T::LParen) && self.at_function(self.pos) || self.at_generics();
            if !begins {
                let msg = "`|` joins only function literals: a function literal `( … ) { … }` must follow it";
                return Err(Diagnostic::new(Code::Unexpected, bar, msg));
            }
            branches.push(self.function()?);
        }
        Ok(ExprKind::Chain(branches))
    }

    fn generics(&mut self) -> PResult<Vec<Ident>> {
        if !self.eat(&T::Lt) {
            return Ok(Vec::new());
        }
        self.seq(T::Gt, false, Self::up_name)
    }

    /// `[<A, …>] Params [<- (services)] [-> Union] Block`.
    fn function(&mut self) -> PResult<Function> {
        let start = self.start();
        let generics = self.generics()?;
        self.expect(&T::LParen)?;
        let mut params = Vec::new();
        let mut guard = None;
        loop {
            if self.at(&T::RParen) {
                break;
            }
            if self.eat(&T::Question) {
                // P9.
                guard = Some(self.expr()?);
                if !self.at(&T::RParen) {
                    return self
                        .error("`)` after the guard, which is the last part of a parameter group");
                }
                break;
            }
            if self.at(&T::DotDot) {
                let msg = "a parameter group has no rest `..` at its top: the shape of a function is written out; a nested data pattern may have one";
                return Err(Diagnostic::new(Code::BadSpread, self.tok().span, msg));
            }
            let key = if self.at_key() {
                let key = self.name()?;
                self.bump();
                Some(key)
            } else {
                None
            };
            // P8.
            let mut choices = vec![self.pattern()?];
            while self.eat(&T::Pipe) {
                choices.push(self.pattern()?);
            }
            params.push(PItem { key, choices });
            if self.at(&T::RParen) || self.at(&T::Question) {
                continue;
            }
            if !self.eat(&T::Comma) && !self.tok().brk {
                return self.error("`,`, `?` or `)`");
            }
        }
        self.bump();
        self.check_order(&params, |i| match &i.key {
            Some(k) => (ItemOrder::Key, k.span.0),
            None => (ItemOrder::Pos, i.choices[0].span.0),
        });
        let services = if self.eat(&T::LArrow) {
            if !self.at(&T::LParen) {
                let msg = "the service pattern of a function is in brackets: `<- (name: Type)`; a bare row variable is written only in a type";
                return Err(Diagnostic::new(Code::BadPattern, self.tok().span, msg));
            }
            let open = self.bump();
            let items = self.seq(T::RParen, true, |p| {
                let rest = p.eat(&T::DotDot);
                let name = match p.name() {
                    Ok(name) if p.at(&T::Colon) => name,
                    _ => {
                        let msg = "a service in a function head is written `name: Type`, and the rest of the environment `..name: N`; the type is the key";
                        return Err(Diagnostic::new(Code::BadPattern, p.tok().span, msg));
                    }
                };
                p.bump();
                let ty = p.named_type()?;
                Ok(SItem { rest, name, ty })
            })?;
            if items.is_empty() {
                let msg =
                    "an empty service pattern `<- ()` is not written: leave the `<-` part out";
                self.diags
                    .push(Diagnostic::new(Code::BadPattern, open, msg));
            }
            if let Some(i) = items.iter().position(|i| i.rest)
                && i + 1 != items.len()
            {
                let msg = "the rest `..name: N` is the last item of a service pattern";
                self.diags
                    .push(Diagnostic::new(Code::BadSpread, items[i].name.span.0, msg));
            }
            Some(items)
        } else {
            None
        };
        let ret = if self.eat(&T::Arrow) {
            Some(self.ty()?)
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
        Ok(Function {
            generics,
            params,
            guard,
            services,
            ret,
            body,
            span: Sp(self.span_from(start)),
        })
    }

    fn list(&mut self) -> PResult<ExprKind> {
        self.bump();
        let mut items = Vec::new();
        let mut rest = None;
        while !self.eat(&T::RBracket) {
            if !items.is_empty() {
                if !self.eat(&T::Comma) && !self.tok().brk {
                    return self.error("`,` or `]`");
                }
                if self.eat(&T::RBracket) {
                    break;
                }
            }
            if self.at(&T::DotDot) {
                if items.is_empty() {
                    let msg = "`..rest` comes after the parts of a list";
                    return Err(Diagnostic::new(Code::BadSpread, self.tok().span, msg));
                }
                self.bump();
                rest = Some(Box::new(self.expr()?));
                self.eat(&T::Comma);
                self.expect(&T::RBracket)?;
                break;
            }
            items.push(self.expr()?);
        }
        Ok(ExprKind::List { items, rest })
    }

    // ---- Patterns ----

    /// `Pattern = Part { "=" Part }`.
    fn pattern(&mut self) -> PResult<Pattern> {
        self.pattern_in(false)
    }

    /// `segment` is set at the top of a bit array segment, where `:` begins a
    /// format and not a type.
    fn pattern_in(&mut self, segment: bool) -> PResult<Pattern> {
        let start = self.start();
        let first = self.pattern_part(segment)?;
        if !self.at(&T::Eq) {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.eat(&T::Eq) {
            parts.push(self.pattern_part(segment)?);
        }
        Ok(Pattern::new(PatternKind::And(parts), self.span_from(start)))
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
            span: Sp(self.bump()),
        })
    }

    fn pattern_string(&mut self) -> PResult<String> {
        match self.kind().clone() {
            T::Str(s) => {
                self.bump();
                Ok(s)
            }
            _ => self.error("a string literal in a string pattern"),
        }
    }

    /// `Part = Atom | StringPat`.
    fn pattern_part(&mut self, segment: bool) -> PResult<Pattern> {
        let start = self.start();
        let kind = match self.kind().clone() {
            T::Name(_) if self.at_qualified() => self.constructor_pattern()?,
            T::Name(_) | T::Discard(_) => {
                if matches!(self.kind(), T::Name(_))
                    && self.toks[self.pos + 1].kind == T::LParen
                    && !self.toks[self.pos + 1].brk
                {
                    let msg = "a lowercase name followed by `(` is not a pattern; a constructor pattern begins with an UpName";
                    return Err(Diagnostic::new(Code::BadPattern, self.tok().span, msg));
                }
                let binder = self.binder()?;
                if self.eat(&T::LtGt) {
                    let suffix = self.pattern_string()?;
                    PatternKind::Prefix {
                        prefix: String::new(),
                        suffix,
                        rest: binder,
                    }
                } else if !segment && self.eat(&T::Colon) {
                    PatternKind::Typed {
                        binder,
                        ty: self.annot()?,
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
            T::Interpolated(_) => {
                let msg = "string interpolation is not allowed in patterns";
                return Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg));
            }
            T::Str(prefix) => {
                self.bump();
                if self.eat(&T::LtGt) {
                    let rest = self.binder()?;
                    let suffix = if self.eat(&T::LtGt) {
                        self.pattern_string()?
                    } else {
                        String::new()
                    };
                    PatternKind::Prefix {
                        prefix,
                        suffix,
                        rest,
                    }
                } else {
                    PatternKind::String(prefix)
                }
            }
            T::LParen => PatternKind::Data(self.data_pattern()?),
            T::LBracket => self.list_pattern()?,
            T::LtLt => {
                self.bump();
                PatternKind::BitArray(self.seq(T::GtGt, true, |p| {
                    let value = p.pattern_in(true)?;
                    p.segment(value)
                })?)
            }
            T::Question => {
                let msg = "a guard `?` is written only at the end of a parameter group, after the last item";
                return Err(Diagnostic::new(Code::BadPattern, self.tok().span, msg));
            }
            _ => return self.error("a pattern"),
        };
        Ok(Pattern::new(kind, self.span_from(start)))
    }

    /// `DataPat`, from its `(` (P3, P7).
    fn data_pattern(&mut self) -> PResult<Vec<DPItem>> {
        self.expect(&T::LParen)?;
        let items = self.seq(T::RParen, true, |p| {
            if p.eat(&T::DotDot) {
                let binder = match p.kind() {
                    T::Name(_) | T::Discard(_) => Some(p.binder()?),
                    _ => None,
                };
                if !p.at(&T::RParen) && !(p.at(&T::Comma) && *p.nth(1) == T::RParen) {
                    let msg = "the rest `..` is the last item of a data pattern";
                    return Err(Diagnostic::new(Code::BadSpread, p.tok().span, msg));
                }
                return Ok(DPItem::Rest(binder));
            }
            if p.at_key() {
                let key = p.name()?;
                p.bump();
                return Ok(DPItem::Key(key, p.pattern()?));
            }
            Ok(DPItem::Pos(p.pattern()?))
        })?;
        self.check_order(&items, |i| match i {
            DPItem::Pos(p) => (ItemOrder::Pos, p.span.0),
            DPItem::Key(k, _) => (ItemOrder::Key, k.span.0),
            DPItem::Rest(_) => (ItemOrder::Other, Span::default()),
        });
        Ok(items)
    }

    /// `[m.]C[DataPat]`.
    fn constructor_pattern(&mut self) -> PResult<PatternKind> {
        let module = self.qualifier()?;
        let name = self.up_name()?;
        let args = if self.at_joined(&T::LParen) {
            Some(self.data_pattern()?)
        } else {
            None
        };
        Ok(PatternKind::Ctor { module, name, args })
    }

    fn list_pattern(&mut self) -> PResult<PatternKind> {
        self.bump();
        let mut items = Vec::new();
        let mut rest = None;
        if self.eat(&T::RBracket) {
            return Ok(PatternKind::List { items, rest });
        }
        loop {
            if self.eat(&T::DotDot) {
                let binder = match self.kind() {
                    T::Name(_) | T::Discard(_) => Some(self.binder()?),
                    _ => None,
                };
                rest = Some(binder);
                self.eat(&T::Comma);
                self.expect(&T::RBracket)?;
                break;
            }
            items.push(self.pattern()?);
            if self.eat(&T::RBracket) || self.eat(&T::Comma) && self.eat(&T::RBracket) {
                break;
            }
            let sep = self.toks[self.pos - 1].kind == T::Comma || self.tok().brk;
            if !sep {
                return self.error("`,` or `]`");
            }
        }
        Ok(PatternKind::List { items, rest })
    }

    // ---- Types ----

    fn ty(&mut self) -> PResult<Type> {
        self.type_list(T::Pipe, Self::type_inter::<false>, TypeKind::Union)
    }

    /// The one precedence rule after every type colon. Bracket contents
    /// use `ty`; unbracketed function results keep this boundary.
    fn annot(&mut self) -> PResult<Type> {
        self.type_inter::<true>()
    }

    fn type_inter<const ANNOT: bool>(&mut self) -> PResult<Type> {
        self.type_list(T::Amp, Self::type_diff::<ANNOT>, TypeKind::Inter)
    }

    fn type_diff<const ANNOT: bool>(&mut self) -> PResult<Type> {
        self.type_list(T::Minus, Self::type_prim::<ANNOT>, TypeKind::Diff)
    }

    fn type_list(
        &mut self,
        op: T,
        part: fn(&mut Self) -> PResult<Type>,
        make: fn(Vec<Type>) -> TypeKind,
    ) -> PResult<Type> {
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
        Ok(Type::new(make(parts), self.span_from(start)))
    }

    fn named_type(&mut self) -> PResult<Type> {
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
        Ok(Type::new(
            TypeKind::Named { module, name, args },
            self.span_from(start),
        ))
    }

    fn type_int_part(&mut self) -> PResult<num_bigint::BigUint> {
        match self.kind().clone() {
            T::Int(n) => {
                self.bump();
                Ok(n)
            }
            T::LBrace => {
                self.bump();
                let n = self.type_int_part()?;
                self.close_type_group()?;
                Ok(n)
            }
            _ => self.error("an integer literal after `-` in a type"),
        }
    }

    fn close_type_group(&mut self) -> PResult<()> {
        if self.tok().brk {
            let msg = "a line break directly inside a `{ }` that groups a type is a compile error";
            return Err(Diagnostic::new(Code::TypeGroupBreak, self.tok().span, msg));
        }
        self.expect(&T::RBrace)?;
        Ok(())
    }

    /// `DataType`, from its `(` (P10).
    fn data_type(&mut self) -> PResult<Vec<TItem>> {
        self.expect(&T::LParen)?;
        let items = self.seq(T::RParen, true, |p| {
            if p.at(&T::DotDot) {
                let msg = "a data type has no spread; `..N` is written only in the needs of a function type, after `<-`";
                return Err(Diagnostic::new(Code::BadSpread, p.tok().span, msg));
            }
            if p.at_key() {
                let label = p.name()?;
                p.bump();
                return Ok(TItem::Label(label, p.ty()?));
            }
            if matches!(p.kind(), T::Name(_) | T::UpName(_) | T::Discard(_))
                && *p.nth(1) == T::Colon
            {
                let msg = "`:` is not written in a type: a label is `label = Type`, and a function type has no parameter names";
                return Err(Diagnostic::new(Code::Unexpected, p.toks[p.pos + 1].span, msg));
            }
            Ok(TItem::Pos(p.ty()?))
        })?;
        self.check_order(&items, |i| match i {
            TItem::Pos(t) => (ItemOrder::Pos, t.span.0),
            TItem::Label(n, _) => (ItemOrder::Key, n.span.0),
            TItem::Spread(t) => (ItemOrder::Other, t.span.0),
        });
        Ok(items)
    }

    /// `[ "<-" Services ] "->" Union`, after the parameter part.
    fn fn_type<const ANNOT: bool>(
        &mut self,
        generics: Vec<Ident>,
        param: Type,
        start: u32,
    ) -> PResult<Type> {
        let services = if self.eat(&T::LArrow) {
            Some(self.needs()?)
        } else {
            None
        };
        self.expect(&T::Arrow)?;
        let ret = if ANNOT { self.annot()? } else { self.ty()? };
        Ok(Type::new(
            TypeKind::Fn(Box::new(FnType {
                generics,
                param,
                services,
                ret,
            })),
            self.span_from(start),
        ))
    }

    /// `Needs = UpName | "(" [ NItem { "," NItem } ] ")"`, where an item is a
    /// named service type or one row variable `..N`.
    fn needs(&mut self) -> PResult<Type> {
        let from = self.start();
        if !self.at(&T::LParen) {
            if !matches!(self.kind(), T::UpName(_)) {
                return self.error(
                    "the needs of a function type: `(Db, Log)`, `(..N, Db)` or a row variable `N`",
                );
            }
            let name = self.up_name()?;
            return Ok(Type::new(
                TypeKind::Named {
                    module: None,
                    name,
                    args: Vec::new(),
                },
                self.span_from(from),
            ));
        }
        let open = self.bump();
        let items = self.seq(T::RParen, true, |p| {
            if p.eat(&T::DotDot) {
                let at = p.start();
                let name = p.up_name()?;
                return Ok(TItem::Spread(Type::new(
                    TypeKind::Named {
                        module: None,
                        name,
                        args: Vec::new(),
                    },
                    p.span_from(at),
                )));
            }
            if p.at_key() {
                let msg = "a need has no label: a service is keyed by its own type";
                return Err(Diagnostic::new(Code::Unexpected, p.tok().span, msg));
            }
            Ok(TItem::Pos(p.named_type()?))
        })?;
        if items.is_empty() {
            let msg = "an empty need `<- ()` is not written: leave the `<-` part out";
            self.diags
                .push(Diagnostic::new(Code::Unexpected, open, msg));
        }
        if items
            .iter()
            .filter(|i| matches!(i, TItem::Spread(_)))
            .count()
            > 1
        {
            let msg = "a need has at most one row variable `..N`";
            self.diags.push(Diagnostic::new(Code::BadSpread, open, msg));
        }
        Ok(Type::new(TypeKind::Data(items), self.span_from(from)))
    }

    fn at_fn_tail(&self) -> bool {
        matches!(self.kind(), T::Arrow | T::LArrow)
    }

    fn type_prim<const ANNOT: bool>(&mut self) -> PResult<Type> {
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
                let msg = "Float has no literal types";
                return Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg));
            }
            T::Lt => {
                let generics = self.generics()?;
                let param = match self.kind() {
                    T::LParen => {
                        let from = self.start();
                        let items = self.data_type()?;
                        Type::new(TypeKind::Data(items), self.span_from(from))
                    }
                    T::LBrace => {
                        self.bump();
                        let inner = self.ty()?;
                        self.close_type_group()?;
                        inner
                    }
                    _ => return self.error("the parameter part of a function type"),
                };
                if !self.at_fn_tail() {
                    return self.error("`->` or `<-`: type parameters `<A>` begin a function type");
                }
                return self.fn_type::<ANNOT>(generics, param, start);
            }
            T::LParen => {
                let items = self.data_type()?;
                let data = Type::new(TypeKind::Data(items), self.span_from(start));
                if self.at_fn_tail() {
                    return self.fn_type::<ANNOT>(Vec::new(), data, start);
                }
                return Ok(data);
            }
            T::LBrace => {
                self.bump();
                let inner = self.ty()?;
                self.close_type_group()?;
                if self.at_fn_tail() {
                    return self.fn_type::<ANNOT>(Vec::new(), inner, start);
                }
                return Ok(inner);
            }
            T::Name(_) if self.at_qualified() => return self.named_type(),
            T::UpName(_) => return self.named_type(),
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
            T::Caret | T::Bang => {
                let msg = "`^` and `!` are removed from function types: errors and services are written after `<-`, as in `(String) <- (Db, Fail(NotFound)) -> User`";
                return Err(Diagnostic::new(Code::Unexpected, self.tok().span, msg));
            }
            _ => return self.error("a type"),
        };
        Ok(Type::new(kind, self.span_from(start)))
    }
}

/// The line rule is unchanged; this only explains a likely misplaced statement (§1.7).
pub const ONE_BINDING_HINT: &str =
    "this is one binding, not two; to write two, put the second on its own line (§1.7)";

fn spaced_binding_call(source: &str, toks: &[Token], open: usize, eq: usize) -> bool {
    if open == 0 || toks[open].kind != T::LParen {
        return false;
    }
    let name = &toks[open - 1];
    if !matches!(name.kind, T::Name(_) | T::UpName(_)) {
        return false;
    }
    let gap = &source[name.span.end as usize..toks[open].span.start as usize];
    gap.len() >= 2
        && gap.bytes().all(|b| b == b' ')
        && !source[name.span.start as usize..toks[eq].span.end as usize].contains(['\n', '\r'])
}

/// Suspicious spacing inside an already parsed conjunction binding. The caller
/// supplies the binding's pattern through its final `=`, excluding the value.
pub fn binding_line_traps(source: &str, span: Span) -> Vec<Span> {
    let Some(text) = source.get(span.start as usize..span.end as usize) else {
        return Vec::new();
    };
    if !text.contains("  (") {
        return Vec::new();
    }
    let lexed = lex3(text);
    let mut out = Vec::new();
    for (open, token) in lexed.tokens.iter().enumerate() {
        if token.kind != T::LParen || open < 2 {
            continue;
        }
        let name_start = if open >= 4 && lexed.tokens[open - 2].kind == T::Dot {
            open - 3
        } else {
            open - 1
        };
        if name_start == 0 || lexed.tokens[name_start - 1].kind != T::Eq {
            continue;
        }
        let Some(close) = lexed.close[open] else {
            continue;
        };
        let eq = close as usize + 1;
        if lexed.tokens.get(eq).is_some_and(|t| t.kind == T::Eq)
            && spaced_binding_call(text, &lexed.tokens, open, eq)
        {
            let name = lexed.tokens[open - 1].span;
            out.push(Span::new(
                span.start + name.start,
                span.start + token.span.end,
            ));
        }
    }
    out
}
