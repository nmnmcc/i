//! One part of a tree as text of the normal surface, for the positions of
//! `i/syntax` (docs/design/syntax-3.md §6.7). The text is on one line but
//! for the line ends that the layout always has (the statements of a block,
//! an attribute before its item, a call of depth greater than
//! [`MAX_CALL_DEPTH`]). The tree has its groups already (`group.rs`).

use super::*;
use crate::syn3::lisp::position::{Needs, PatternItem, PatternPart, ResultPart, SegmentPart};

fn printer() -> Printer<'static> {
    Printer {
        src: None,
        comments: &[],
        next: 0,
    }
}

fn wide(doc: Doc) -> String {
    render_in(&doc, usize::MAX / 4).trim_end().to_string()
}

pub(crate) fn import(i: &Import) -> String {
    let mut s = String::new();
    if let Some(a) = &i.alias {
        s.push_str(&format!("{} = ", a.name));
    }
    if !i.names.is_empty() {
        let names: Vec<&str> = i.names.iter().map(|n| n.name.as_str()).collect();
        s.push_str(&format!("({}) = ", names.join(", ")));
    }
    let path: Vec<&str> = i.path.iter().map(|n| n.name.as_str()).collect();
    s.push_str(&path.join("/"));
    s
}

pub(crate) fn export(e: &Export) -> String {
    if e.constructors {
        format!("{}(..)", e.name.name)
    } else {
        e.name.name.clone()
    }
}

pub(crate) fn item(i: &Item) -> String {
    wide(printer().item(i))
}

pub(crate) fn attribute(a: &Attribute) -> String {
    let mut parts = vec![t(format!("@{}", qualified(&a.module, &a.name)))];
    if let Some(args) = &a.args {
        parts.push(printer().data(args, false));
    }
    wide(cat(parts))
}

pub(crate) fn ty(ty: &Type) -> String {
    wide(printer().ty(ty, 0))
}

pub(crate) fn type_item(i: &TItem) -> String {
    let mut p = printer();
    wide(match i {
        TItem::Pos(ty) => p.ty(ty, 0),
        TItem::Label(n, ty) => cat(vec![t(format!("{} = ", n.name)), p.ty(ty, 0)]),
        TItem::Spread(ty) => cat(vec![t(".."), p.ty(ty, 3)]),
    })
}

/// `-> R`, `<- needs -> R`, or `<- (services)` when a function has a
/// service pattern and no written result. No result and no `<-` is `-> _`.
pub(crate) fn function_result(r: &ResultPart) -> String {
    let mut p = printer();
    let mut out = Vec::new();
    match &r.needs {
        Some(Needs::Services(services)) => {
            out.push(t("<- "));
            let parts = services
                .iter()
                .map(|s| {
                    let head = format!("{}{}: ", if s.rest { ".." } else { "" }, s.name.name);
                    cat(vec![t(head), p.ty(&s.ty, 0)])
                })
                .collect();
            out.push(p.delimited("(", ")", Printer::plain(parts)));
        }
        Some(Needs::Type(need)) => {
            out.push(t("<- "));
            out.push(p.ty_raw(need));
        }
        None => {}
    }
    match &r.ret {
        Some(ret) => {
            out.push(t(if out.is_empty() { "-> " } else { " -> " }));
            out.push(p.ty(ret, 0));
        }
        None if !matches!(r.needs, Some(Needs::Services(_))) => {
            out.push(t(if out.is_empty() { "-> _" } else { " -> _" }));
        }
        None => {}
    }
    wide(cat(out))
}

pub(crate) fn expr(e: &Expr) -> String {
    wide(printer().expr(e, LOWEST))
}

pub(crate) fn value_item(i: &DItem) -> String {
    wide(printer().data_item(i).doc)
}

pub(crate) fn statement(s: &Stmt) -> String {
    wide(printer().stmt(s))
}

fn pitem(p: &mut Printer, item: &PItem) -> Doc {
    let mut out = Vec::new();
    if let Some(k) = &item.key {
        out.push(t(format!("{} = ", k.name)));
    }
    for (j, c) in item.choices.iter().enumerate() {
        if j > 0 {
            out.push(t(" | "));
        }
        out.push(p.pattern(c));
    }
    cat(out)
}

pub(crate) fn pattern(part: &PatternPart) -> String {
    let mut p = printer();
    match part {
        PatternPart::Pattern(pattern) => wide(p.pattern(pattern)),
        PatternPart::Params(items, guard) => {
            let mut out = vec![t("(")];
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(t(", "));
                }
                out.push(pitem(&mut p, item));
            }
            if let Some(g) = guard {
                out.push(t(if items.is_empty() { "? " } else { " ? " }));
                out.push(p.expr(g, LOWEST));
            }
            out.push(t(")"));
            wide(cat(out))
        }
    }
}

pub(crate) fn pattern_item(item: &PatternItem) -> String {
    match item {
        PatternItem::Item(item) => wide(pitem(&mut printer(), item)),
        PatternItem::Rest => "..".into(),
    }
}

pub(crate) fn segment(s: &SegmentPart) -> String {
    let mut p = printer();
    wide(match s {
        SegmentPart::Expr(s) => cat(vec![p.expr(&s.value, LOWEST), p.format(&s.format)]),
        SegmentPart::Pattern(s) => cat(vec![p.pattern(&s.value), p.format(&s.format)]),
    })
}

/// The format of a segment, without the `:` that stands before it.
pub(crate) fn format(f: &Format) -> String {
    let text = wide(printer().format(&Some(f.clone())));
    text.strip_prefix(':').unwrap_or(&text).to_string()
}
