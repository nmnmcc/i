// SPDX-License-Identifier: Apache-2.0
// The scanning of numbers, strings, escapes, and comments is adapted from the
// Gleam lexer (https://github.com/gleam-lang/gleam, Apache-2.0, see
// licenses/gleam-LICENCE). SPDX-FileCopyrightText: 2020 The Gleam contributors

//! The lexer (§1): tokens, the line rule (§1.7), and bracket pairs.

use crate::span::Span;
use crate::token::{StringPart, Token, TokenKind as T};
use crate::{Code, Diagnostic};
use num_bigint::BigUint;

/// A `///` comment (the documentation of the next item) or a `////` comment
/// (the documentation of the module) (§1.2).
#[derive(Clone, Debug, PartialEq)]
pub struct DocComment {
    pub module: bool,
    pub text: String,
    pub span: Span,
}

/// Any comment, `//`, `///`, or `////`, from its `//` to the end of its
/// line (§1.2). Comments have no meaning; the formatter puts them back.
#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}

/// The result of lexing a file.
#[derive(Clone, Debug)]
pub struct Lexed {
    /// The tokens; the last one is always `Eof`.
    pub tokens: Vec<Token>,
    /// For each `(`, `[`, `{`, and `<<`, the index of its closing token.
    pub close: Vec<Option<u32>>,
    pub docs: Vec<DocComment>,
    /// Every comment, in source order, documentation or not.
    pub comments: Vec<Comment>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lex(source: &str) -> Lexed {
    lex_surface(source, false, false)
}

/// Lex a Lisp-surface file. Line endings are trivia, and quote tokens are
/// available only in this surface.
pub fn lex_lisp(source: &str) -> Lexed {
    lex_surface(source, true, false)
}

/// Lex a syntax-three file (docs/design/syntax-3.md): `<-` is one token, and
/// a line that begins with `<A, …>(` begins a statement.
pub fn lex3(source: &str) -> Lexed {
    lex_surface(source, false, true)
}

/// Lex a syntax-three Lisp-surface file.
pub fn lex3_lisp(source: &str) -> Lexed {
    lex_surface(source, true, true)
}

fn lex_surface(source: &str, lisp: bool, syn3: bool) -> Lexed {
    let mut lexer = Lexer {
        src: source.as_bytes(),
        source,
        pos: 0,
        tokens: Vec::new(),
        docs: Vec::new(),
        comments: Vec::new(),
        diags: Vec::new(),
        lisp,
        syn3,
    };
    if source.starts_with('\u{feff}') {
        lexer.pos = 3;
    }
    lexer.run(false);
    let Lexer {
        mut tokens,
        docs,
        comments,
        mut diags,
        ..
    } = lexer;
    mark_breaks(&mut tokens, syn3);
    let close = pair_brackets(&tokens, &mut diags);
    Lexed {
        tokens,
        close,
        docs,
        comments,
        diagnostics: diags,
    }
}

/// §1.7: a line end is a break when the token before it can end an expression
/// and the token after it can begin a statement.
fn mark_breaks(tokens: &mut [Token], syn3: bool) {
    for i in 1..tokens.len() {
        let begins = tokens[i].kind.can_begin() || syn3 && generics_end(tokens, i).is_some();
        let brk = tokens[i].nl && tokens[i - 1].kind.can_end() && begins;
        tokens[i].brk = brk;
    }
}

/// Syntax three: `<` followed by `UpName { , UpName } > (` begins the type
/// parameters of a function. Returns the index of the `(`.
pub(crate) fn generics_end(tokens: &[Token], at: usize) -> Option<usize> {
    if tokens.get(at)?.kind != T::Lt {
        return None;
    }
    let mut i = at + 1;
    loop {
        if !matches!(tokens.get(i)?.kind, T::UpName(_)) {
            return None;
        }
        i += 1;
        match tokens.get(i)?.kind {
            T::Comma => {
                i += 1;
                if tokens.get(i)?.kind == T::Gt {
                    break;
                }
            }
            T::Gt => break,
            _ => return None,
        }
    }
    (tokens.get(i + 1)?.kind == T::LParen).then_some(i + 1)
}

fn opener(kind: &T) -> Option<T> {
    Some(match kind {
        T::RParen => T::LParen,
        T::RBracket => T::LBracket,
        T::RBrace => T::LBrace,
        T::GtGt => T::LtLt,
        _ => return None,
    })
}

pub(crate) fn pair_brackets(tokens: &[Token], diags: &mut Vec<Diagnostic>) -> Vec<Option<u32>> {
    // Each kind has a link to its nearest open bracket. When a mismatched
    // closer searches below the top, every discarded opener is popped once;
    // an absent opener is found without rescanning the whole stack.
    fn index(kind: &T) -> usize {
        match kind {
            T::LParen => 0,
            T::LBracket => 1,
            T::LBrace => 2,
            T::LtLt => 3,
            _ => unreachable!("not an opening bracket"),
        }
    }
    struct Open {
        token: usize,
        kind: usize,
        previous: Option<usize>,
    }
    let mut close = vec![None; tokens.len()];
    let mut stack: Vec<Open> = Vec::new();
    let mut nearest = [None; 4];
    for (i, tok) in tokens.iter().enumerate() {
        match &tok.kind {
            kind @ (T::LParen | T::LBracket | T::LBrace | T::LtLt) => {
                let kind = index(kind);
                stack.push(Open {
                    token: i,
                    kind,
                    previous: nearest[kind],
                });
                nearest[kind] = Some(stack.len() - 1);
            }
            kind => {
                let Some(open) = opener(kind) else { continue };
                let target = index(&open);
                if let Some(entry) = stack.pop_if(|entry| entry.kind == target) {
                    nearest[target] = entry.previous;
                    close[entry.token] = Some(i as u32);
                    continue;
                }
                let msg = match stack.last() {
                    Some(entry) => format!("{} does not close {}", kind, tokens[entry.token].kind),
                    None => format!("{kind} has no opening bracket"),
                };
                diags.push(Diagnostic::new(Code::UnmatchedBracket, tok.span, msg));
                if let Some(depth) = nearest[target] {
                    while stack.len() > depth {
                        let entry = stack.pop().unwrap();
                        nearest[entry.kind] = entry.previous;
                        if entry.kind == target {
                            close[entry.token] = Some(i as u32);
                        }
                    }
                }
            }
        }
    }
    for entry in stack {
        let msg = format!("{} is not closed", tokens[entry.token].kind);
        diags.push(Diagnostic::new(
            Code::UnclosedBracket,
            tokens[entry.token].span,
            msg,
        ));
    }
    close
}

struct Lexer<'a> {
    src: &'a [u8],
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    docs: Vec<DocComment>,
    comments: Vec<Comment>,
    diags: Vec<Diagnostic>,
    lisp: bool,
    syn3: bool,
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Lexer<'_> {
    fn at(&self, offset: usize) -> u8 {
        self.src.get(self.pos + offset).copied().unwrap_or(0)
    }

    fn span(&self, start: usize) -> Span {
        Span::new(start as u32, self.pos as u32)
    }

    fn error(&mut self, code: Code, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic::new(code, span, msg));
    }

    fn push(&mut self, kind: T, start: usize, nl: bool) {
        let span = self.span(start);
        self.tokens.push(Token {
            kind,
            span,
            nl,
            brk: false,
        });
    }

    /// Skips spaces, line ends, and comments. Returns whether a line end or a
    /// comment was passed.
    fn skip_trivia(&mut self) -> bool {
        let mut nl = false;
        loop {
            match self.at(0) {
                b' ' | b'\t' => self.pos += 1,
                b'\n' | b'\r' => {
                    nl = true;
                    self.pos += 1;
                }
                b'/' if self.at(1) == b'/' => {
                    nl = true;
                    self.comment();
                }
                _ => return nl,
            }
        }
    }

    fn comment(&mut self) {
        let start = self.pos;
        let mut slashes = 0;
        while self.at(0) == b'/' && slashes < 4 {
            self.pos += 1;
            slashes += 1;
        }
        let text_start = self.pos;
        while self.pos < self.src.len() && !matches!(self.src[self.pos], b'\n' | b'\r') {
            self.pos += 1;
        }
        self.comments.push(Comment {
            text: self.source[start..self.pos].to_string(),
            span: self.span(start),
        });
        if slashes >= 3 {
            self.docs.push(DocComment {
                module: slashes == 4,
                text: self.source[text_start..self.pos].to_string(),
                span: self.span(start),
            });
        }
    }

    fn run(&mut self, interpolation: bool) {
        let mut parens = 0usize;
        loop {
            let nl = self.skip_trivia();
            let start = self.pos;
            let Some(&c) = self.src.get(self.pos) else {
                self.push(T::Eof, start, true);
                return;
            };
            if interpolation && c == b')' && parens == 0 {
                self.push(T::Eof, start, true);
                self.pos += 1;
                return;
            }
            let after_dot = !self.lisp && self.tokens.last().is_some_and(|t| t.kind == T::Dot);
            let kind = match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.word(),
                b'0'..=b'9' if after_dot => self.tuple_index(),
                b'0'..=b'9' => self.number(),
                b'"' | b'#' => self.string(),
                _ => match self.symbol(c) {
                    Some(kind) => kind,
                    None => {
                        let ch = self.source[self.pos..].chars().next().unwrap();
                        self.pos += ch.len_utf8();
                        let msg = if ch.is_control() {
                            format!("unexpected character U+{:04X}", ch as u32)
                        } else {
                            format!("unexpected character `{ch}`")
                        };
                        self.error(Code::UnexpectedChar, self.span(start), msg);
                        continue;
                    }
                },
            };
            match kind {
                T::LParen => parens += 1,
                T::RParen => parens = parens.saturating_sub(1),
                _ => {}
            }
            self.push(kind, start, nl);
        }
    }

    fn symbol(&mut self, c: u8) -> Option<T> {
        if self.syn3 && !self.lisp && (c == b'^' || c == b'!' && self.at(1) != b'=') {
            // Syntax three P0: `^` and `!` alone are no symbols of i.
            let span = Span::new(self.pos as u32, self.pos as u32 + 1);
            self.error(
                Code::UnexpectedChar,
                span,
                format!(
                    "`{}` is not a symbol of i: errors and services are written after `<-`, as in `(String) <- (Db, Fail(NotFound)) -> User`",
                    c as char
                ),
            );
        }
        let (kind, len) = match (c, self.at(1), self.at(2)) {
            (b'=', b'=', _) => (T::EqEq, 2),
            (b'=', _, _) => (T::Eq, 1),
            (b':', _, _) => (T::Colon, 1),
            (b',', _, _) => (T::Comma, 1),
            (b'.', b'.', _) => (T::DotDot, 2),
            (b'.', _, _) => (T::Dot, 1),
            (b'-', b'>', _) => (T::Arrow, 2),
            (b'-', b'.', _) => (T::MinusDot, 2),
            (b'-', _, _) => (T::Minus, 1),
            (b'?', _, _) => (T::Question, 1),
            (b'^', _, _) => (T::Caret, 1),
            (b'!', b'=', _) => (T::NotEq, 2),
            (b'!', _, _) => (T::Bang, 1),
            (b'|', b'>', _) => (T::PipeGt, 2),
            (b'|', b'|', _) => (T::PipePipe, 2),
            (b'|', _, _) => (T::Pipe, 1),
            (b'@', _, _) => (T::At, 1),
            (b'`', _, _) if self.lisp => (T::Quote, 1),
            (b'~', _, _) if self.lisp => (T::Tilde, 1),
            (b'(', _, _) => (T::LParen, 1),
            (b')', _, _) => (T::RParen, 1),
            (b'[', _, _) => (T::LBracket, 1),
            (b']', _, _) => (T::RBracket, 1),
            (b'{', _, _) => (T::LBrace, 1),
            (b'}', _, _) => (T::RBrace, 1),
            (b'<', b'<', _) => (T::LtLt, 2),
            (b'<', b'-', _) if self.syn3 => (T::LArrow, 2),
            (b'<', b'=', b'.') => (T::LtEqDot, 3),
            (b'<', b'=', _) => (T::LtEq, 2),
            (b'<', b'.', _) => (T::LtDot, 2),
            (b'<', b'>', _) => (T::LtGt, 2),
            (b'<', _, _) => (T::Lt, 1),
            (b'>', b'>', _) => (T::GtGt, 2),
            (b'>', b'=', b'.') => (T::GtEqDot, 3),
            (b'>', b'=', _) => (T::GtEq, 2),
            (b'>', b'.', _) => (T::GtDot, 2),
            (b'>', _, _) => (T::Gt, 1),
            (b'+', b'.', _) => (T::PlusDot, 2),
            (b'+', _, _) => (T::Plus, 1),
            (b'*', b'.', _) => (T::StarDot, 2),
            (b'*', _, _) => (T::Star, 1),
            (b'/', b'.', _) => (T::SlashDot, 2),
            (b'/', _, _) => (T::Slash, 1),
            (b'%', _, _) => (T::Percent, 1),
            (b'&', b'&', _) => (T::AmpAmp, 2),
            (b'&', _, _) => (T::Amp, 1),
            _ => return None,
        };
        self.pos += len;
        Some(kind)
    }

    /// §1.3: names, discards, and UpNames.
    fn word(&mut self) -> T {
        let start = self.pos;
        while is_word(self.at(0)) {
            self.pos += 1;
        }
        let w = &self.source[start..self.pos];
        let b = w.as_bytes();
        let lower_tail = |s: &[u8]| {
            s.iter()
                .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        };
        let (kind, ok) = if b[0].is_ascii_uppercase() {
            (T::UpName(w.into()), b.iter().all(u8::is_ascii_alphanumeric))
        } else if w == "_" || w.starts_with("__") {
            (T::Discard(w.into()), lower_tail(b))
        } else if b[0] == b'_' {
            let ok = b[1].is_ascii_lowercase() && lower_tail(b);
            (T::Name(w.into()), ok)
        } else {
            (T::Name(w.into()), lower_tail(b))
        };
        if !ok {
            let msg = format!("`{w}` is not a name, a discard, or an UpName");
            self.error(Code::BadName, self.span(start), msg);
        }
        kind
    }

    /// Reads a run of digits of the given radix, with `_` between groups.
    /// Returns the digits without `_`, or `None` after reporting an error.
    fn digits(&mut self, radix: u32, letters: bool) -> Option<String> {
        let start = self.pos;
        let accept = |b: u8| b == b'_' || (b as char).is_digit(radix) || letters && is_word(b);
        while accept(self.at(0)) {
            self.pos += 1;
        }
        let run = &self.source[start..self.pos];
        let span = self.span(start);
        if let Some(bad) = run.chars().find(|&c| c != '_' && !c.is_digit(radix)) {
            let msg = format!("`{bad}` is not a digit of base {radix}");
            self.error(Code::BadNumber, span, msg);
            return None;
        }
        if run.is_empty() {
            self.error(Code::BadNumber, span, "expected digits");
            return None;
        }
        if run.starts_with('_') || run.ends_with('_') || run.contains("__") {
            let msg = "`_` may only be between two digits";
            self.error(Code::BadDigitGroup, span, msg);
            return None;
        }
        Some(run.replace('_', ""))
    }

    /// Reports letters right after a number, such as `12px` or `1.0E3`.
    fn check_number_end(&mut self, start: usize) -> bool {
        if !is_word(self.at(0)) {
            return true;
        }
        while is_word(self.at(0)) {
            self.pos += 1;
        }
        let msg = format!("`{}` is not a number", &self.source[start..self.pos]);
        self.error(Code::BadNumber, self.span(start), msg);
        false
    }

    fn int(digits: Option<String>, radix: u32) -> T {
        let value = digits.and_then(|d| BigUint::parse_bytes(d.as_bytes(), radix));
        T::Int(value.unwrap_or_default())
    }

    /// The `decimal` after `.`: `t.0.1` is `(t.0).1` (§1.5).
    fn tuple_index(&mut self) -> T {
        let start = self.pos;
        let digits = self.digits(10, false);
        let ok = self.check_number_end(start);
        Self::int(digits.filter(|_| ok), 10)
    }

    /// §1.6: int and float literals.
    fn number(&mut self) -> T {
        let start = self.pos;
        let radix = match (self.at(0), self.at(1)) {
            (b'0', b'x') => 16,
            (b'0', b'o') => 8,
            (b'0', b'b') => 2,
            _ => 10,
        };
        if radix != 10 {
            self.pos += 2;
            let digits = self.digits(radix, true);
            return Self::int(digits, radix);
        }
        let int = self.digits(10, false);
        if self.at(0) != b'.' {
            let ok = self.check_number_end(start);
            return Self::int(int.filter(|_| ok), 10);
        }
        self.pos += 1;
        let mut ok = int.is_some();
        let mut frac = String::from("0");
        if self.at(0).is_ascii_digit() {
            match self.digits(10, false) {
                Some(d) => frac = d,
                None => ok = false,
            }
        }
        let mut exp = String::from("0");
        if self.at(0) == b'e' && (self.at(1).is_ascii_digit() || self.at(1) == b'-') {
            self.pos += 1;
            let neg = self.at(0) == b'-';
            if neg {
                self.pos += 1;
            }
            match self.digits(10, false) {
                Some(d) => exp = if neg { format!("-{d}") } else { d },
                None => ok = false,
            }
        }
        if !(self.check_number_end(start) && ok) {
            return T::Float(0.0);
        }
        let text = format!("{}.{frac}e{exp}", int.unwrap());
        let value: f64 = text.parse().unwrap_or(f64::INFINITY);
        if !value.is_finite() {
            let msg = "this float literal is not a finite binary64 number";
            self.error(Code::FloatNotFinite, self.span(start), msg);
            return T::Float(0.0);
        }
        T::Float(value)
    }

    /// Swift-style delimiter matching: a raw escape needs exactly the opening
    /// number of hashes. A closing quote consumes that many hashes.
    fn string(&mut self) -> T {
        enum Piece {
            Text(usize, usize),
            Expr {
                tokens: Vec<Token>,
                span: Span,
                comments: Vec<Comment>,
            },
        }
        let start = self.pos;
        while self.at(0) == b'#' {
            self.pos += 1;
        }
        let hashes = self.pos - start;
        if self.at(0) != b'"' {
            self.error(
                Code::UnexpectedChar,
                self.span(start),
                "a raw string delimiter needs a quote after `#`",
            );
            return T::Str(String::new());
        }
        let triple = self.src[self.pos..].starts_with(b"\"\"\"");
        // Swift distinguishes raw quote-only strings such as #"""# from
        // multi-line openers by a matching raw close on the same line.
        let same_line_close = hashes > 0 && triple && {
            let line_end = self.source[self.pos..]
                .find(['\r', '\n'])
                .map_or(self.src.len(), |n| self.pos + n);
            (self.pos + 2..line_end).any(|at| {
                self.src[at] == b'"'
                    && self
                        .src
                        .get(at + 1..at + 1 + hashes)
                        .is_some_and(|s| s.iter().all(|b| *b == b'#'))
            })
        };
        let multiline = triple && !same_line_close;
        let quotes = if multiline { 3 } else { 1 };
        self.pos += quotes;
        if multiline {
            if !matches!(self.at(0), b'\r' | b'\n') {
                self.error(
                    Code::BadEscape,
                    self.span(start),
                    "a multi-line string opening delimiter must be followed by a line break",
                );
            } else {
                self.newline();
            }
        }
        let body = self.pos;
        let mut text_start = body;
        let mut pieces = Vec::new();
        let end;
        loop {
            if self.pos == self.src.len() {
                self.error(
                    Code::UnterminatedString,
                    Span::new(start as u32, start as u32 + 1),
                    "this string is not closed",
                );
                end = self.pos;
                pieces.push(Piece::Text(text_start, end));
                break;
            }
            if self.src[self.pos..].starts_with(&vec![b'"'; quotes])
                && self
                    .src
                    .get(self.pos + quotes..self.pos + quotes + hashes)
                    .is_some_and(|s| s.iter().all(|b| *b == b'#'))
            {
                end = self.pos;
                pieces.push(Piece::Text(text_start, end));
                self.pos += quotes + hashes;
                break;
            }
            if self.at(0) == b'\\' && self.escape_hashes(hashes) {
                let slash = self.pos;
                self.pos += 1 + hashes;
                if self.at(0) == b'(' {
                    pieces.push(Piece::Text(text_start, slash));
                    self.pos += 1;
                    let expression_start = self.pos;
                    let mut child = Lexer {
                        src: self.src,
                        source: self.source,
                        pos: self.pos,
                        tokens: Vec::new(),
                        docs: Vec::new(),
                        comments: Vec::new(),
                        diags: Vec::new(),
                        lisp: self.lisp,
                        syn3: self.syn3,
                    };
                    child.run(true);
                    if child.pos == child.src.len() && child.src.last() != Some(&b')') {
                        child.error(
                            Code::UnterminatedString,
                            child.span(slash),
                            "this string interpolation is missing its closing `)`",
                        );
                    }
                    self.pos = child.pos;
                    self.diags.extend(child.diags);
                    mark_breaks(&mut child.tokens, self.syn3);
                    let span = Span::new(
                        expression_start as u32,
                        child
                            .tokens
                            .last()
                            .expect("expression ends in EOF")
                            .span
                            .start,
                    );
                    pieces.push(Piece::Expr {
                        tokens: child.tokens,
                        span,
                        comments: child.comments,
                    });
                    text_start = self.pos;
                } else if self.pos < self.src.len() {
                    // Keep escapes in the text range; decode after indentation
                    // is known. Skip the escaped character while finding quotes.
                    self.pos += self.source[self.pos..].chars().next().unwrap().len_utf8();
                }
            } else {
                self.pos += self.source[self.pos..].chars().next().unwrap().len_utf8();
            }
        }
        let after = self.pos;
        let mut remove = Vec::new();
        if multiline {
            let line = self.source[..end]
                .rfind(['\r', '\n'])
                .map_or(body, |p| p + 1);
            let indent = &self.source[line..end];
            if !indent.bytes().all(|b| matches!(b, b' ' | b'\t')) {
                self.error(
                    Code::BadEscape,
                    Span::new(line as u32, end as u32),
                    "a multi-line string closing delimiter must be on its own line",
                );
            } else {
                let mut at = body;
                while at < line {
                    let stop = self.source[at..line]
                        .find(['\r', '\n'])
                        .map_or(line, |n| at + n);
                    let content = &self.source[at..stop];
                    let blank = content.bytes().all(|b| matches!(b, b' ' | b'\t'));
                    if content.starts_with(indent) {
                        remove.push((at, at + indent.len()));
                    } else if blank {
                        remove.push((at, stop));
                    } else {
                        self.error(Code::BadEscape, Span::new(at as u32, stop as u32), "a multi-line string line has less or different indentation than its closing delimiter");
                    }
                    at = stop;
                    if self.src.get(at) == Some(&b'\r') {
                        at += 1;
                    }
                    if self.src.get(at) == Some(&b'\n') {
                        at += 1;
                    }
                }
                let mut last = line;
                if last > body && self.src[last - 1] == b'\n' {
                    last -= 1;
                }
                if last > body && self.src[last - 1] == b'\r' {
                    last -= 1;
                }
                remove.push((last, end));
            }
        }
        let mut parts = Vec::new();
        let mut interpolated = false;
        for piece in pieces {
            match piece {
                Piece::Expr {
                    tokens,
                    span,
                    comments,
                } => {
                    interpolated = true;
                    parts.push(StringPart::Expr {
                        tokens,
                        span,
                        comments,
                    });
                }
                Piece::Text(a, z) => {
                    self.pos = a;
                    let mut value = String::new();
                    while self.pos < z {
                        if let Some((_, stop)) =
                            remove.iter().find(|(a, z)| *a <= self.pos && self.pos < *z)
                        {
                            self.pos = (*stop).min(z);
                            continue;
                        }
                        let c = self.source[self.pos..].chars().next().unwrap();
                        self.pos += c.len_utf8();
                        if c == '\\' && self.escape_hashes_after_slash(hashes) {
                            self.pos += hashes;
                            if multiline && matches!(self.at(0), b'\r' | b'\n') {
                                self.newline();
                            } else {
                                self.escape(&mut value);
                            }
                        } else if c == '\r' {
                            if self.at(0) == b'\n' {
                                self.pos += 1;
                            }
                            value.push('\n');
                        } else {
                            value.push(c);
                        }
                    }
                    parts.push(StringPart::Text(value, Span::new(a as u32, z as u32)));
                }
            }
        }
        self.pos = after;
        if interpolated {
            if self.lisp {
                self.error(
                    Code::BadEscape,
                    self.span(start),
                    "the Lisp surface has no string interpolation; use `<>`",
                );
            }
            T::Interpolated(parts)
        } else {
            let [StringPart::Text(value, _)] = parts.as_slice() else {
                unreachable!("a literal has one text part")
            };
            T::Str(value.clone())
        }
    }

    fn escape_hashes(&self, hashes: usize) -> bool {
        self.src
            .get(self.pos + 1..self.pos + 1 + hashes)
            .is_some_and(|s| s.iter().all(|b| *b == b'#'))
            && (hashes == 0 || self.src.get(self.pos + 1 + hashes) != Some(&b'#'))
    }

    fn escape_hashes_after_slash(&self, hashes: usize) -> bool {
        self.src
            .get(self.pos..self.pos + hashes)
            .is_some_and(|s| s.iter().all(|b| *b == b'#'))
            && (hashes == 0 || self.src.get(self.pos + hashes) != Some(&b'#'))
    }

    fn newline(&mut self) {
        if self.at(0) == b'\r' {
            self.pos += 1;
        }
        if self.at(0) == b'\n' {
            self.pos += 1;
        }
    }

    fn escape(&mut self, value: &mut String) {
        let start = self.pos - 1;
        let c = match self.at(0) {
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'f' => '\u{c}',
            b'"' => '"',
            b'\\' => '\\',
            b'u' if self.at(1) == b'{' => {
                self.pos += 2;
                let digits_start = self.pos;
                while self.at(0).is_ascii_hexdigit() {
                    self.pos += 1;
                }
                let digits = &self.source[digits_start..self.pos];
                let closed = self.at(0) == b'}';
                if closed {
                    self.pos += 1;
                }
                let span = self.span(start);
                let scalar = u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32);
                match scalar {
                    Some(c) if closed && (1..=6).contains(&digits.len()) => value.push(c),
                    _ => {
                        let msg =
                            "`\\u{…}` needs 1 to 6 hex digits that make a Unicode scalar value";
                        self.error(Code::BadUnicodeEscape, span, msg);
                    }
                }
                return;
            }
            _ => {
                let len = self.source[self.pos..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8);
                self.pos += len;
                self.error(Code::BadEscape, self.span(start), "unknown escape");
                return;
            }
        };
        self.pos += 1;
        value.push(c);
    }
}
