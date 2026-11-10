// SPDX-License-Identifier: Apache-2.0
// The scanning of numbers, strings, escapes, and comments is adapted from the
// Gleam lexer (https://github.com/gleam-lang/gleam, Apache-2.0, see
// licenses/gleam-LICENCE). SPDX-FileCopyrightText: 2020 The Gleam contributors

//! The lexer (§1): tokens, the line rule (§1.7), and bracket pairs.

use crate::span::Span;
use crate::token::{Token, TokenKind as T};
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
    lex_surface(source, false)
}

/// Lex a Lisp-surface file. Line endings are trivia, and quote tokens are
/// available only in this surface.
pub fn lex_lisp(source: &str) -> Lexed {
    lex_surface(source, true)
}

fn lex_surface(source: &str, lisp: bool) -> Lexed {
    let mut lexer = Lexer {
        src: source.as_bytes(),
        source,
        pos: 0,
        tokens: Vec::new(),
        docs: Vec::new(),
        comments: Vec::new(),
        diags: Vec::new(),
        lisp,
    };
    if source.starts_with('\u{feff}') {
        lexer.pos = 3;
    }
    lexer.run();
    let Lexer {
        mut tokens,
        docs,
        comments,
        mut diags,
        ..
    } = lexer;
    mark_breaks(&mut tokens);
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
fn mark_breaks(tokens: &mut [Token]) {
    for i in 1..tokens.len() {
        let brk = tokens[i].nl && tokens[i - 1].kind.can_end() && tokens[i].kind.can_begin();
        tokens[i].brk = brk;
    }
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

fn pair_brackets(tokens: &[Token], diags: &mut Vec<Diagnostic>) -> Vec<Option<u32>> {
    let mut close = vec![None; tokens.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, tok) in tokens.iter().enumerate() {
        match &tok.kind {
            T::LParen | T::LBracket | T::LBrace | T::LtLt => stack.push(i),
            kind => {
                let Some(open) = opener(kind) else { continue };
                if stack.last().is_some_and(|&o| tokens[o].kind == open) {
                    close[stack.pop().unwrap()] = Some(i as u32);
                    continue;
                }
                let msg = match stack.last() {
                    Some(&o) => format!("{} does not close {}", kind, tokens[o].kind),
                    None => format!("{kind} has no opening bracket"),
                };
                diags.push(Diagnostic::new(Code::UnmatchedBracket, tok.span, msg));
                if let Some(depth) = stack.iter().rposition(|&o| tokens[o].kind == open) {
                    stack.truncate(depth + 1);
                    close[stack.pop().unwrap()] = Some(i as u32);
                }
            }
        }
    }
    for o in stack {
        let msg = format!("{} is not closed", tokens[o].kind);
        diags.push(Diagnostic::new(Code::UnclosedBracket, tokens[o].span, msg));
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

    fn run(&mut self) {
        loop {
            let nl = self.skip_trivia();
            let start = self.pos;
            let Some(&c) = self.src.get(self.pos) else {
                self.push(T::Eof, start, true);
                return;
            };
            let after_dot = !self.lisp && self.tokens.last().is_some_and(|t| t.kind == T::Dot);
            let kind = match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.word(),
                b'0'..=b'9' if after_dot => self.tuple_index(),
                b'0'..=b'9' => self.number(),
                b'"' => self.string(),
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
            self.push(kind, start, nl);
        }
    }

    fn symbol(&mut self, c: u8) -> Option<T> {
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
            (b'!', b'=', _) => (T::NotEq, 2),
            (b'!', _, _) => (T::Bang, 1),
            (b'|', b'>', _) => (T::PipeGt, 2),
            (b'|', b'|', _) => (T::PipePipe, 2),
            (b'|', _, _) => (T::Pipe, 1),
            (b'@', _, _) => (T::At, 1),
            (b'`', _, _) if self.lisp => (T::Quote, 1),
            (b'~', _, _) if self.lisp => (T::Tilde, 1),
            (b'^', _, _) if self.lisp => (T::Caret, 1),
            (b'(', _, _) => (T::LParen, 1),
            (b')', _, _) => (T::RParen, 1),
            (b'[', _, _) => (T::LBracket, 1),
            (b']', _, _) => (T::RBracket, 1),
            (b'{', _, _) => (T::LBrace, 1),
            (b'}', _, _) => (T::RBrace, 1),
            (b'<', b'<', _) => (T::LtLt, 2),
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

    /// §1.6: string literals and escapes.
    fn string(&mut self) -> T {
        let start = self.pos;
        self.pos += 1;
        let mut value = String::new();
        loop {
            let Some(c) = self.source[self.pos..].chars().next() else {
                let msg = "this string is not closed";
                self.error(
                    Code::UnterminatedString,
                    Span::new(start as u32, start as u32 + 1),
                    msg,
                );
                return T::Str(value);
            };
            self.pos += c.len_utf8();
            match c {
                '"' => return T::Str(value),
                '\r' => {
                    if self.at(0) == b'\n' {
                        self.pos += 1;
                    }
                    value.push('\n');
                }
                '\\' => self.escape(&mut value),
                c => value.push(c),
            }
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
