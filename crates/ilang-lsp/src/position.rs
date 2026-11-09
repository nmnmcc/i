//! Positions: LSP counts lines from 0 and characters in UTF-16 code units;
//! the compiler counts bytes. `\r\n`, `\r`, and `\n` all end a line, both
//! in LSP and in i (§1.1).

use ilang_syntax::span::Span;
use lsp_types::{Position, Range};

/// The starts of the lines of a text.
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(text: &'a str) -> LineIndex<'a> {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => starts.push(i + 1),
                b'\r' if bytes.get(i + 1) != Some(&b'\n') => starts.push(i + 1),
                _ => {}
            }
            i += 1;
        }
        LineIndex { text, starts }
    }

    /// The end of the text of a line, before its line end.
    fn line_end(&self, line: usize) -> usize {
        let end = self
            .starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        let s = &self.text.as_bytes()[..end];
        let mut e = end;
        if e > self.starts[line] && s[e - 1] == b'\n' {
            e -= 1;
        }
        if e > self.starts[line] && s[e - 1] == b'\r' {
            e -= 1;
        }
        e
    }

    /// The byte offset of a position. A line or character past the end is
    /// taken as the end of the text or of the line.
    pub fn offset(&self, p: Position) -> u32 {
        let line = p.line as usize;
        if line >= self.starts.len() {
            return self.text.len() as u32;
        }
        let start = self.starts[line];
        let end = self.line_end(line);
        let mut units = 0u32;
        for (i, c) in self.text[start..end].char_indices() {
            if units >= p.character {
                return (start + i) as u32;
            }
            units += c.len_utf16() as u32;
        }
        end as u32
    }

    /// The position of a byte offset. An offset inside a character is taken
    /// as the start of the character.
    pub fn position(&self, offset: u32) -> Position {
        let offset = (offset as usize).min(self.text.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let start = self.starts[line];
        let character = self.text[start..]
            .char_indices()
            .take_while(|(i, _)| start + i < offset)
            .map(|(_, c)| c.len_utf16() as u32)
            .sum();
        Position::new(line as u32, character)
    }

    pub fn range(&self, span: Span) -> Range {
        Range::new(self.position(span.start), self.position(span.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_positions() {
        // `é` is 2 bytes and 1 unit; `𝕏` is 4 bytes and 2 units.
        let text = "ab\r\né𝕏x = 1\rz\n";
        let ix = LineIndex::new(text);
        let x = text.find('x').unwrap() as u32;
        assert_eq!(ix.position(x), Position::new(1, 3));
        assert_eq!(ix.offset(Position::new(1, 3)), x);
        assert_eq!(
            ix.offset(Position::new(1, 1)),
            text.find('𝕏').unwrap() as u32
        );
        let z = text.find('z').unwrap() as u32;
        assert_eq!(ix.position(z), Position::new(2, 0));
        assert_eq!(ix.offset(Position::new(2, 0)), z);
        // Past the end of a line, and of the text.
        assert_eq!(ix.offset(Position::new(0, 99)), 2);
        assert_eq!(ix.offset(Position::new(9, 0)), text.len() as u32);
        assert_eq!(ix.position(text.len() as u32), Position::new(3, 0));
        for (i, _) in text
            .char_indices()
            .filter(|(_, c)| !matches!(c, '\r' | '\n'))
        {
            assert_eq!(ix.offset(ix.position(i as u32)), i as u32, "at {i}");
        }
    }
}
