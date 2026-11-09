//! The formatter behind `textDocument/formatting`.
//!
//! The formatter is `ilang_syntax::format::format_source(&str) ->
//! Option<String>`, which gives `None` when the text has a syntax error.
//! It is used only with the cargo feature `format` of this crate, which is
//! off until that function is in the tree. Without the feature, the server
//! does not offer formatting, and a formatting request is answered with an
//! error.
//!
//! To switch it on once `ilang_syntax::format` is merged, make `format` a
//! default feature in `crates/ilang-lsp/Cargo.toml`:
//!
//! ```toml
//! [features]
//! default = ["format"]
//! format = []
//! ```
//!
//! or remove the feature and the `cfg`s below.

/// The formatter, when this build has one.
pub fn formatter() -> Option<fn(&str) -> Option<String>> {
    #[cfg(feature = "format")]
    {
        Some(ilang_syntax::format::format_source)
    }
    #[cfg(not(feature = "format"))]
    {
        None
    }
}
