//! Deep lexer diagnostics.

#[path = "oracle.rs"]
mod ilang_syntax;
#[test]
fn many_mismatched_closers_keep_diagnostics_and_do_not_rescan_openers() {
    let n = 10_000;
    let source = format!("{}{}", "(".repeat(n), "]".repeat(n));
    let lexed = ilang_syntax::lex(&source);
    assert_eq!(lexed.diagnostics.len(), 2 * n);
    assert_eq!(lexed.diagnostics[0].span.start as usize, n);
    assert_eq!(lexed.diagnostics[n - 1].span.start as usize, 2 * n - 1);
    assert_eq!(lexed.diagnostics[n].span.start, 0);
    assert_eq!(lexed.diagnostics[2 * n - 1].span.start as usize, n - 1);
}
