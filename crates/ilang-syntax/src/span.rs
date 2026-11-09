//! Byte spans and line/column positions.

use std::fmt;

/// A range of bytes in a source file: `start..end`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    /// The smallest span that covers both spans.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}..{}", self.start, self.end)
    }
}

/// Finds the line and column of a byte offset. Lines count from 1. Columns
/// count Unicode scalar values from 1 (§11.7). `\r\n`, `\r`, and `\n` all end
/// a line (§1.1).
pub struct LineIndex<'a> {
    source: &'a str,
    line_starts: Vec<u32>,
}

impl<'a> LineIndex<'a> {
    pub fn new(source: &'a str) -> LineIndex<'a> {
        let bytes = source.as_bytes();
        let mut line_starts = vec![0];
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => line_starts.push(i as u32 + 1),
                b'\r' if bytes.get(i + 1) != Some(&b'\n') => line_starts.push(i as u32 + 1),
                _ => {}
            }
            i += 1;
        }
        LineIndex {
            source,
            line_starts,
        }
    }

    /// The line and column of a byte offset, both counting from 1.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line] as usize;
        let end = (offset as usize).min(self.source.len());
        let column = self.source.get(start..end).map_or(0, |s| s.chars().count());
        (line as u32 + 1, column as u32 + 1)
    }
}
