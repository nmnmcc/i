//! The formatter of `i fmt` for the normal surface: the layout of syntax
//! three (`crate::print::format`, see its module documentation).

use crate::Diagnostic;

/// The width that the layout tries to keep to.
pub const WIDTH: usize = crate::print::WIDTH;
/// The columns of one tab, for the width.
pub const TAB_WIDTH: usize = 4;

/// Formats a source file. Syntax errors leave the source untouched.
pub fn format(source: &str) -> Result<String, Vec<Diagnostic>> {
    crate::print::format(source)
}
