//! Tests of the lexer: the token sets of §1.7 and positions.

use ilang_syntax::lex;
use ilang_syntax::span::LineIndex;
use ilang_syntax::token::TokenKind as T;

fn breaks(src: &str) -> Vec<bool> {
    lex(src).tokens.iter().map(|t| t.brk).collect()
}

#[test]
fn breaks_need_both_sides() {
    // `a` can end and `b` can begin.
    assert_eq!(breaks("a\nb"), [false, true, false]);
    // `+` cannot end, `)` cannot begin.
    assert_eq!(breaks("a +\nb"), [false, false, false, false]);
    assert_eq!(breaks("(a\n)"), [false, false, false, false]);
    // `|>`, `.`, `|`, `?`, `->`, `if` never begin a statement.
    for op in ["|> f", ".f", "| f", "? f", "-> f", "if f", "* f", "== f"] {
        assert!(!lex(&format!("a\n{op}")).tokens[1].brk, "{op}");
    }
    // `-`, `-.`, `..`, `!`, `@`, and keywords other than `if` do.
    for op in [
        "- f", "-. f", "..f", "! F", "@main", "case", "import", "export",
    ] {
        assert!(lex(&format!("a\n{op}")).tokens[1].brk, "{op}");
    }
    // A comment between the two sides does not change the rule.
    assert_eq!(breaks("a // c\nb"), [false, true, false]);
    assert_eq!(breaks("a + // c\nb"), [false, false, false, false]);
}

#[test]
fn longest_match_and_decimals_after_dot() {
    let kinds: Vec<T> = lex("<=. << t.0.1 1.5 ..")
        .tokens
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(kinds[0], T::LtEqDot);
    assert_eq!(kinds[1], T::LtLt);
    assert!(matches!(kinds[3], T::Dot) && matches!(kinds[4], T::Int(_)));
    assert!(matches!(kinds[5], T::Dot) && matches!(kinds[6], T::Int(_)));
    assert!(matches!(kinds[7], T::Float(x) if x == 1.5));
    assert_eq!(kinds[8], T::DotDot);
}

#[test]
fn brackets_are_paired() {
    let lexed = lex("( [ { << >> } ] )");
    assert_eq!(lexed.close[0], Some(7));
    assert_eq!(lexed.close[3], Some(4));
    assert!(lexed.diagnostics.is_empty());
    assert_eq!(lex("( ]").diagnostics.len(), 2);
}

#[test]
fn line_and_column_count_scalar_values() {
    let src = "aé\r\nb\rcd";
    let index = LineIndex::new(src);
    assert_eq!(index.line_col(0), (1, 1));
    assert_eq!(index.line_col(3), (1, 3));
    assert_eq!(index.line_col(5), (2, 1));
    assert_eq!(index.line_col(8), (3, 2));
}

#[test]
fn docs_are_kept() {
    let lexed = lex("//// m\n/// d\n// c\nx = 1");
    assert_eq!(lexed.docs.len(), 2);
    assert!(lexed.docs[0].module && !lexed.docs[1].module);
}

#[test]
fn every_comment_is_kept() {
    let src = "//// m\n/// d\n// c\nx = 1 // t\n";
    let lexed = lex(src);
    let texts: Vec<&str> = lexed.comments.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["//// m", "/// d", "// c", "// t"]);
    for c in &lexed.comments {
        assert_eq!(&src[c.span.start as usize..c.span.end as usize], c.text);
    }
}

#[test]
fn carriage_returns_are_line_ends() {
    assert_eq!(breaks("a\r\nb\rc"), [false, true, true, false]);
    let lexed = lex("\"x\r\ny\rz\"");
    assert_eq!(lexed.tokens[0].kind, T::Str("x\ny\nz".into()));
}
