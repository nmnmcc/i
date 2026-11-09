//! Places in the text of a file that an analysis was made from, and in
//! the text of its document now: a request is answered from the last
//! analysis while a newer one is being made, where the text around the
//! position has not changed.

use ilang_syntax::span::Span;

/// How a text changed: the bytes before `prefix` and the last `suffix`
/// bytes are the same in both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Change {
    same: bool,
    prefix: u32,
    suffix: u32,
    old_len: u32,
    new_len: u32,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Change {
    pub fn new(old: &str, new: &str) -> Change {
        let (o, n) = (old.as_bytes(), new.as_bytes());
        let prefix = o.iter().zip(n).take_while(|(a, b)| a == b).count();
        let room = o.len().min(n.len()) - prefix;
        let suffix = o
            .iter()
            .rev()
            .zip(n.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        Change {
            same: old == new,
            prefix: prefix as u32,
            suffix: suffix as u32,
            old_len: o.len() as u32,
            new_len: n.len() as u32,
        }
    }

    pub fn is_same(&self) -> bool {
        self.same
    }

    /// A span of the new text in the old text, when it and the bytes on
    /// each side of it did not change.
    pub fn old_span(&self, s: Span) -> Option<Span> {
        if self.same || s.end < self.prefix {
            Some(s)
        } else if s.start > self.new_len - self.suffix {
            let shift = self.old_len.wrapping_sub(self.new_len);
            Some(Span::new(
                s.start.wrapping_add(shift),
                s.end.wrapping_add(shift),
            ))
        } else {
            None
        }
    }

    /// A span of the old text in the new text, when it did not change.
    pub fn new_span(&self, s: Span) -> Option<Span> {
        if self.same || s.end <= self.prefix {
            Some(s)
        } else if s.start >= self.old_len - self.suffix {
            let shift = self.new_len.wrapping_sub(self.old_len);
            Some(Span::new(
                s.start.wrapping_add(shift),
                s.end.wrapping_add(shift),
            ))
        } else {
            None
        }
    }

    /// The span in the old text of the name `base` of the new text, just
    /// before a dot that may have been written since: the name and what
    /// is before it did not change, and in the old text it is not followed
    /// by more of a name.
    pub fn old_name(&self, base: Span, old: &str) -> Option<Span> {
        if self.same {
            return Some(base);
        }
        if base.end <= self.prefix
            && !old
                .as_bytes()
                .get(base.end as usize)
                .is_some_and(|b| is_word_byte(*b))
        {
            return Some(base);
        }
        self.old_span(base)
    }
}

/// The span of the name or number at `offset`, or of the one byte there.
pub(crate) fn word_at(text: &str, offset: u32) -> Span {
    let b = text.as_bytes();
    let o = (offset as usize).min(b.len());
    let mut start = o;
    while start > 0 && is_word_byte(b[start - 1]) {
        start -= 1;
    }
    let mut end = o;
    while end < b.len() && is_word_byte(b[end]) {
        end += 1;
    }
    if start == end {
        end = (o + 1).min(b.len());
    }
    Span::new(start as u32, end as u32)
}

/// The span of the name just before `end`, maybe empty.
pub(crate) fn name_before(text: &str, end: u32) -> Span {
    let b = text.as_bytes();
    let end = (end as usize).min(b.len());
    let mut start = end;
    while start > 0 && is_word_byte(b[start - 1]) {
        start -= 1;
    }
    Span::new(start as u32, end as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(text: &str, needle: &str) -> Span {
        let i = text.find(needle).unwrap() as u32;
        Span::new(i, i + needle.len() as u32)
    }

    #[test]
    fn places_that_did_not_change() {
        let old = "a = 1\nb = foo(a)\nc = 3\n";
        let new = "a = 1\nb = foo(a, 22)\nc = 3\n";
        let c = Change::new(old, new);
        assert!(!c.is_same());
        // Before the change, and after it.
        assert_eq!(c.old_span(span(new, "a =")), Some(span(old, "a =")));
        assert_eq!(c.old_span(span(new, "foo")), Some(span(old, "foo")));
        assert_eq!(c.old_span(span(new, "c")), Some(span(old, "c")));
        assert_eq!(c.new_span(span(old, "c")), Some(span(new, "c")));
        // At the change.
        assert_eq!(c.old_span(span(new, "22")), None);
        // Next to it: `a` is followed by `)` in the old text.
        assert_eq!(c.old_span(span(new, "a,")), None);
        assert_eq!(c.new_span(span(old, "foo(a)")), None);
    }

    #[test]
    fn a_name_just_before_a_new_dot() {
        let old = "f = (u) { u }\n";
        let new = "f = (u) { u. }\n";
        let c = Change::new(old, new);
        let dot = new.find("u.").unwrap() as u32 + 1;
        let base = name_before(new, dot);
        assert_eq!(c.old_name(base, old), Some(base));
        // `uu` in the old text is another name.
        let old = "f = (uu) { uu }\n";
        let new = "f = (uu) { u.u }\n";
        let c = Change::new(old, new);
        let dot = new.find("u.").unwrap() as u32 + 1;
        assert_eq!(c.old_name(name_before(new, dot), old), None);
    }

    #[test]
    fn words() {
        assert_eq!(word_at("ab cd", 4), Span::new(3, 5));
        assert_eq!(word_at("ab cd", 2), Span::new(0, 2));
        assert_eq!(word_at("a + b", 2), Span::new(2, 3));
        assert_eq!(name_before("x.ab", 4), Span::new(2, 4));
        assert_eq!(name_before("x).", 2), Span::new(2, 2));
    }
}
