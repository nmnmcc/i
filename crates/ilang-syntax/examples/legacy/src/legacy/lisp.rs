//! The pre-syntax-three datum reader, private to migration tools.
use crate::lisp::{Datum, DatumKind, Document};
use crate::token::{Token, TokenKind as T};
use crate::{Code, Diagnostic};

pub fn parse_source_legacy(source: &str) -> (crate::ast::Module, Vec<Diagnostic>) {
    match parse_lexed(super::lexer::lex_lisp(source))
        .and_then(|d| crate::lisp::parse_module(&d).map_err(|e| vec![e]))
    {
        Ok(module) => (module, Vec::new()),
        Err(diags) => (crate::ast::Module::default(), diags),
    }
}

fn parse_lexed(lexed: crate::lexer::Lexed) -> Result<Document, Vec<Diagnostic>> {
    if !lexed.diagnostics.is_empty() {
        return Err(lexed.diagnostics);
    }
    let mut parser = Parser {
        tokens: &lexed.tokens,
        at: 0,
    };
    let mut result = Vec::new();
    while parser.current().kind != T::Eof {
        result.push(parser.datum(false).map_err(|d| vec![d])?);
    }
    let mut docs = lexed.docs.into_iter().peekable();
    for datum in &mut result {
        while docs
            .peek()
            .is_some_and(|doc| doc.span.end <= datum.span.start)
        {
            datum.docs.push(docs.next().unwrap());
        }
    }
    Ok(Document {
        datums: result,
        trailing_docs: docs.collect(),
    })
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

impl Parser<'_> {
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }

    fn datum(&mut self, mut quoted: bool) -> Result<Datum, Diagnostic> {
        enum Frame {
            Group {
                token: Token,
                end: T,
                parts: Vec<Datum>,
                quoted: bool,
            },
            Prefix {
                token: Token,
                splice: Option<bool>,
            },
        }
        let mut frames = Vec::new();
        loop {
            if let Some(Frame::Group {
                token, end, parts, ..
            }) = frames.last_mut()
            {
                if self.current().kind == *end {
                    let close = self.current().span;
                    self.at += 1;
                    let token = token.clone();
                    let parts = std::mem::take(parts);
                    frames.pop();
                    let kind = match token.kind {
                        T::LParen => DatumKind::Round(parts),
                        T::LBracket => DatumKind::Square(parts),
                        T::LBrace => DatumKind::Curly(parts),
                        T::LtLt => DatumKind::Bits(parts),
                        _ => unreachable!(),
                    };
                    let completed = Datum {
                        span: token.span.to(close),
                        docs: Vec::new(),
                        kind,
                    };
                    if let Some(result) = complete(&mut frames, completed, &mut quoted) {
                        return Ok(result);
                    }
                    continue;
                }
                if self.current().kind == T::Eof {
                    return Err(Diagnostic::new(
                        Code::UnclosedBracket,
                        token.span,
                        "unclosed datum",
                    ));
                }
            }
            let token = self.current().clone();
            self.at += 1;
            let kind = match token.kind.clone() {
                T::Name(s) => DatumKind::Name(s),
                T::Discard(s) => DatumKind::Discard(s),
                T::UpName(s) => DatumKind::UpName(s),
                T::Int(n) => DatumKind::Int(n),
                T::Float(n) => DatumKind::Float(n),
                T::Str(s) => DatumKind::String(s),
                T::LParen | T::LBracket | T::LBrace | T::LtLt => {
                    let end = match token.kind {
                        T::LParen => T::RParen,
                        T::LBracket => T::RBracket,
                        T::LBrace => T::RBrace,
                        T::LtLt => T::GtGt,
                        _ => unreachable!(),
                    };
                    frames.push(Frame::Group {
                        token,
                        end,
                        parts: Vec::new(),
                        quoted,
                    });
                    continue;
                }
                T::Quote => {
                    if quoted {
                        return Err(Diagnostic::new(
                            Code::Unexpected,
                            token.span,
                            "a quote inside a quote must occur in an unquote or splice",
                        ));
                    }
                    frames.push(Frame::Prefix {
                        token,
                        splice: None,
                    });
                    quoted = true;
                    continue;
                }
                T::Tilde => {
                    if !quoted {
                        return Err(Diagnostic::new(
                            Code::Unexpected,
                            token.span,
                            "unquote is only allowed inside a quote",
                        ));
                    }
                    let splice = self.current().kind == T::DotDot;
                    if splice {
                        self.at += 1;
                    }
                    frames.push(Frame::Prefix {
                        token,
                        splice: Some(splice),
                    });
                    quoted = false;
                    continue;
                }
                T::Eof | T::RParen | T::RBracket | T::RBrace | T::GtGt => {
                    return Err(Diagnostic::new(
                        Code::Unexpected,
                        token.span,
                        "expected a datum",
                    ));
                }
                symbol => DatumKind::Symbol(symbol.text().into()),
            };
            let completed = Datum {
                kind,
                span: token.span,
                docs: Vec::new(),
            };
            if let Some(result) = complete(&mut frames, completed, &mut quoted) {
                return Ok(result);
            }
        }

        fn complete(frames: &mut Vec<Frame>, mut value: Datum, quoted: &mut bool) -> Option<Datum> {
            loop {
                match frames.last_mut() {
                    Some(Frame::Group {
                        parts,
                        quoted: group_quoted,
                        ..
                    }) => {
                        parts.push(value);
                        *quoted = *group_quoted;
                        return None;
                    }
                    Some(Frame::Prefix { .. }) => {
                        let Frame::Prefix { token, splice } = frames.pop().unwrap() else {
                            unreachable!()
                        };
                        let span = token.span.to(value.span);
                        let kind = match splice {
                            None => DatumKind::Quote(Box::new(value)),
                            Some(false) => DatumKind::Unquote(Box::new(value)),
                            Some(true) => DatumKind::Splice(Box::new(value)),
                        };
                        value = Datum {
                            kind,
                            span,
                            docs: Vec::new(),
                        };
                    }
                    None => return Some(value),
                }
            }
        }
    }
}
