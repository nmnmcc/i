//! The formatter behind `textDocument/formatting`: `ilang_syntax::format`,
//! the same formatter as `i fmt`. It gives no edit when the text has a
//! syntax error.

/// Returns the formatter, a function from source text to formatted text.
pub fn formatter() -> Option<fn(&str) -> Option<String>> {
    Some(format_source)
}

fn format_source(text: &str) -> Option<String> {
    ilang_syntax::format::format(text).ok()
}
