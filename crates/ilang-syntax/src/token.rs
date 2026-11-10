// SPDX-License-Identifier: Apache-2.0
// The token set follows the structure of the Gleam lexer's tokens
// (https://github.com/gleam-lang/gleam, Apache-2.0, see licenses/gleam-LICENCE),
// changed for the symbols of i (§1.4, §1.5).

//! Tokens (§1.3 to §1.6) and the token sets of the line rule (§1.7).

use crate::span::Span;
use num_bigint::BigUint;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Name(String),
    UpName(String),
    Discard(String),
    Int(BigUint),
    Float(f64),
    Str(String),
    // Symbols (§1.5)
    Eq,
    Colon,
    Comma,
    Dot,
    DotDot,
    Arrow,
    Question,
    Bang,
    Pipe,
    PipeGt,
    At,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LtLt,
    GtGt,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    LtEq,
    Gt,
    GtEq,
    EqEq,
    NotEq,
    AmpAmp,
    PipePipe,
    LtGt,
    Amp,
    PlusDot,
    MinusDot,
    StarDot,
    SlashDot,
    LtDot,
    LtEqDot,
    GtDot,
    GtEqDot,
    Eof,
}

/// A token. `nl` is set when a line end or a comment comes before it, and
/// `brk` when that line end is a break (§1.7).
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub nl: bool,
    pub brk: bool,
}

impl TokenKind {
    /// The last token before a line end that may be a break (§1.7).
    pub fn can_end(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Name(_) | UpName(_) | Discard(_) | Int(_) | Float(_) | Str(_)
        ) || matches!(self, RParen | RBracket | RBrace | GtGt)
    }

    /// The first token after a line end that may be a break (§1.7).
    pub fn can_begin(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Name(_) | UpName(_) | Discard(_) | Int(_) | Float(_) | Str(_)
        ) || matches!(
            self,
            LParen | LBracket | LBrace | LtLt | Minus | MinusDot | DotDot | Bang | At
        )
    }

    /// The source text of a symbol.
    pub fn text(&self) -> &'static str {
        use TokenKind::*;
        match self {
            Name(_) => "a name",
            UpName(_) => "an UpName",
            Discard(_) => "a discard",
            Int(_) => "an int",
            Float(_) => "a float",
            Str(_) => "a string",
            Eq => "=",
            Colon => ":",
            Comma => ",",
            Dot => ".",
            DotDot => "..",
            Arrow => "->",
            Question => "?",
            Bang => "!",
            Pipe => "|",
            PipeGt => "|>",
            At => "@",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            LBrace => "{",
            RBrace => "}",
            LtLt => "<<",
            GtGt => ">>",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            Lt => "<",
            LtEq => "<=",
            Gt => ">",
            GtEq => ">=",
            EqEq => "==",
            NotEq => "!=",
            AmpAmp => "&&",
            PipePipe => "||",
            LtGt => "<>",
            Amp => "&",
            PlusDot => "+.",
            MinusDot => "-.",
            StarDot => "*.",
            SlashDot => "/.",
            LtDot => "<.",
            LtEqDot => "<=.",
            GtDot => ">.",
            GtEqDot => ">=.",
            Eof => "the end of the file",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Name(n) | TokenKind::UpName(n) | TokenKind::Discard(n) => {
                write!(f, "`{n}`")
            }
            TokenKind::Int(i) => write!(f, "`{i}`"),
            TokenKind::Float(x) => write!(f, "`{x}`"),
            TokenKind::Str(_) | TokenKind::Eof => f.write_str(self.text()),
            _ => write!(f, "`{}`", self.text()),
        }
    }
}
