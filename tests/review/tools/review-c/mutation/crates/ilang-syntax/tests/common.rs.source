//! Helpers shared by the tests.

#![allow(dead_code)]

use ilang_syntax::ast::Module;

/// The debug form of a module without spans, to compare trees.
pub fn shape(module: &Module) -> String {
    let text = format!("{module:?}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find('@') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let skip = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(tail.len());
        if skip > 0 && tail[..skip].contains("..") {
            rest = &tail[skip..];
        } else {
            out.push('@');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}
