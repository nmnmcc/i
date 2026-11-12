//! `types` (syntax-3.md §7.2 stage 3, §7.3 first rows): in today's syntax,
//! every type with two or more constructors becomes one type per
//! constructor plus an alias for their union:
//!
//! ```text
//! Option(A) { Some(A) None }
//! ```
//!
//! becomes
//!
//! ```text
//! Option(A) = Some(A) | None
//! Some(A) { Some(A) }
//! None { None }
//! ```
//!
//! Each constructor keeps the type parameters its fields use, in the order
//! of the original head. Exports follow: `T(..)` becomes `T, C1(..), C2(..)`
//! and `T` becomes `T, C1, C2`. `T {}` and one-constructor types stay.
//!
//! A constructor whose name is already a type name (a built-in type, the
//! type itself, another type of the module, an imported name) gives its new
//! type the name `<name>Case`, so that the constructor and every use of it
//! stay as they are (§7.3's manual list, made mechanical): `Paused {
//! Paused(K) Done }` becomes `Paused = PausedCase | Done` with
//! `PausedCase { Paused(K) }`. A wrapper of the built-in type of the same
//! name (`Int(Int)`) is also listed with §7.3's suggestion to drop it.
//! Left for manual work: a type with a parameter that no constructor uses.

use crate::common::{KEEP, Outcome, Source};
use ilang_syntax::{ast::*, lexer::lex, token::TokenKind as T};
use std::collections::HashSet;

pub const NAME: &str = "types";

/// Types built into the compiler; a new type with such a name would cover them.
const BUILTIN: &[&str] = &[
    "Int", "Float", "String", "List", "BitArray", "UtfCodepoint", "Never", "Boundary",
    "Continuation",
];

/// Prelude types that keep their meaning in every module unless covered.
const PRELUDE: &[&str] = &["Bool", "Nil", "Result", "AssertionFailure", "Assert"];

/// The constructors of a type with two or more. A head that is not a list
/// of type parameters is not today's syntax (`T(a) { … }`), so such a
/// definition is left alone.
fn multi(t: &TypeDef) -> Option<&[Constructor]> {
    match &t.body {
        TypeBody::Custom(cs) if cs.len() >= 2 && t.head.is_empty() => Some(cs),
        _ => None,
    }
}

pub fn run(source: &Source) -> Outcome {
    if source.text.contains(KEEP) {
        return Outcome::default();
    }
    let src = source.text.as_str();
    // A source with syntax errors elsewhere still has its type definitions
    // rewritten: fixtures for other errors keep only the error they test.
    let (module, _) = ilang_syntax::parse_module(src);
    let defs: Vec<(&Item, &TypeDef)> = module
        .items
        .iter()
        .filter_map(|i| match &i.kind {
            ItemKind::TypeDef(t) => Some((i, t)),
            _ => None,
        })
        .collect();
    if !defs.iter().any(|(_, t)| multi(t).is_some()) {
        return Outcome::default();
    }
    let prelude = source.path == "lib/prelude.i";
    // Type names and type parameter names of the module: a new type may
    // not take one (a parameter may not have the name of a type, E0231).
    let type_names: HashSet<&str> = defs
        .iter()
        .flat_map(|(_, t)| std::iter::once(t.name.name.as_str()).chain(t.params.iter().map(|p| p.name.as_str())))
        .chain(module.items.iter().flat_map(|i| match &i.kind {
            ItemKind::Binding(b) => b.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ItemKind::Declaration { type_params, .. } => type_params.iter().map(|p| p.name.as_str()).collect(),
            _ => Vec::new(),
        }))
        .collect();
    let imported: HashSet<&str> = module
        .imports
        .iter()
        .flat_map(|i| i.names.iter().map(|n| n.name.as_str()))
        .collect();
    let mut out = Outcome {
        found: defs.iter().filter(|(_, t)| multi(t).is_some()).count(),
        ..Outcome::default()
    };
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    // The constructors of each migrated type, for the exports.
    let mut migrated: Vec<(&str, Vec<String>)> = Vec::new();
    // The names given to new types so far.
    let mut taken: HashSet<String> = HashSet::new();
    for (item, def) in &defs {
        let Some(ctors) = multi(def) else { continue };
        let at = format!("{}:{}", source.path, source.line + src[..def.name.span.start as usize].matches('\n').count());
        let mut problems = Vec::new();
        // The name of each constructor's new type: the constructor's name,
        // or `<name>Case` when that name is taken (§7.3 manual list).
        let mut type_of = Vec::new();
        for c in ctors {
            let n = c.name.name.as_str();
            let wraps_builtin = c.fields.len() == 1
                && matches!(&c.fields[0].ty.kind, TypeKind::Named { module: None, name, .. } if name.name == n)
                && BUILTIN.contains(&n);
            if wraps_builtin {
                out.manual.push(format!(
                    "{at}: `{}`: constructor `{n}` wraps the built-in type `{n}`; its type is named `{n}Case`. Suggested by hand: drop the wrapper and make `{n}` a member of `{}`; `{n}(e)` becomes `e`, the pattern `{n}(p)` becomes `p: {n}` (§7.3)",
                    def.name.name, def.name.name
                ));
            }
            let clash = |x: &str| {
                BUILTIN.contains(&x)
                    || (!prelude && PRELUDE.contains(&x))
                    || x == def.name.name
                    || type_names.contains(x)
                    || imported.contains(x)
                    || taken.contains(x)
            };
            let mut t = n.to_string();
            let mut k = 1;
            while clash(&t) {
                t = if k == 1 { format!("{n}Case") } else { format!("{n}Case{k}") };
                k += 1;
            }
            taken.insert(t.clone());
            type_of.push(t);
        }
        let used: Vec<Vec<&str>> = ctors.iter().map(|c| used_params(src, def, c)).collect();
        for p in &def.params {
            if !used.iter().any(|u| u.contains(&p.name.as_str())) {
                problems.push(format!("type parameter `{}` is used by no constructor", p.name));
            }
        }
        if !problems.is_empty() {
            out.remaining += 1;
            for p in problems {
                out.manual.push(format!("{at}: `{}`: {p}", def.name.name));
            }
            continue;
        }
        let start = def.name.span.start as usize;
        let end = item.span.end as usize;
        edits.push((start, end, definitions(src, def, ctors, &type_of, &used, start)));
        migrated.push((def.name.name.as_str(), type_of));
    }
    if let Some(span) = module.exports_span {
        let group = &src[span.start as usize..span.end as usize];
        for e in &module.exports {
            let Some((_, names)) = migrated.iter().find(|(t, _)| *t == e.name.name) else {
                continue;
            };
            let sep = if group.contains('\n') {
                format!("\n{}", indent_of(src, e.span.start as usize))
            } else {
                ", ".to_string()
            };
            let suffix = if e.constructors { "(..)" } else { "" };
            let mut text = e.name.name.clone();
            for n in names {
                text.push_str(&sep);
                text.push_str(n.as_str());
                text.push_str(suffix);
            }
            edits.push((e.span.start as usize, e.span.end as usize, text));
        }
    }
    if edits.is_empty() {
        return out;
    }
    edits.sort_by_key(|e| e.0);
    let mut text = src.to_string();
    for (s, e, r) in edits.into_iter().rev() {
        text.replace_range(s..e, &r);
    }
    out.text = Some(text);
    out
}

/// The parameters of `def` that the fields of `c` name, in head order.
fn used_params<'a>(src: &str, def: &'a TypeDef, c: &Constructor) -> Vec<&'a str> {
    let mut names = HashSet::new();
    for f in &c.fields {
        let text = &src[f.ty.span.start as usize..f.ty.span.end as usize];
        let toks = lex(text).tokens;
        for (k, t) in toks.iter().enumerate() {
            if let T::UpName(n) = &t.kind
                && (k == 0 || toks[k - 1].kind != T::Dot)
            {
                names.insert(n.clone());
            }
        }
    }
    def.params
        .iter()
        .map(|p| p.name.as_str())
        .filter(|p| names.contains(*p))
        .collect()
}

fn indent_of(src: &str, at: usize) -> &str {
    let line = src[..at].rfind('\n').map_or(0, |i| i + 1);
    let rest = &src[line..at];
    &rest[..rest.len() - rest.trim_start().len()]
}

fn head(name: &str, params: &[&str]) -> String {
    if params.is_empty() {
        name.to_string()
    } else {
        format!("{name}({})", params.join(", "))
    }
}

/// The comment lines in `src[from..to]`, each trimmed.
fn comments(src: &str, from: usize, to: usize) -> Vec<String> {
    let mut out = Vec::new();
    for l in src[from..to].lines() {
        let t = l.trim();
        if let Some(i) = t.find("//") {
            out.push(t[i..].to_string());
        }
    }
    out
}

fn definitions(
    src: &str,
    def: &TypeDef,
    ctors: &[Constructor],
    type_of: &[String],
    used: &[Vec<&str>],
    start: usize,
) -> String {
    let base = indent_of(src, start).to_string();
    let body = &src[ctors[0].span.start as usize..ctors.last().unwrap().span.end as usize];
    let multi_line = body.contains('\n') || src[start..ctors[0].span.start as usize].contains('\n');
    let unit = if multi_line {
        let ind = indent_of(src, ctors[0].span.start as usize);
        ind.strip_prefix(base.as_str()).unwrap_or("\t").to_string()
    } else {
        String::new()
    };
    let params: Vec<&str> = def.params.iter().map(|p| p.name.as_str()).collect();
    let members: Vec<String> = type_of.iter().zip(used).map(|(t, u)| head(t, u)).collect();
    let mut parts = vec![format!("{} = {}", head(&def.name.name, &params), members.join(" | "))];
    let open = src[start..].find('{').unwrap() + start;
    let mut prev = open + 1;
    for (c, member) in ctors.iter().zip(&members) {
        let mut part = String::new();
        for line in comments(src, prev, c.span.start as usize) {
            part.push_str(&line);
            part.push('\n');
            part.push_str(&base);
        }
        let ctor = &src[c.span.start as usize..c.span.end as usize];
        if multi_line {
            part.push_str(&format!("{member} {{\n{base}{unit}{ctor}\n{base}}}"));
        } else {
            part.push_str(&format!("{member} {{ {ctor} }}"));
        }
        prev = c.span.end as usize;
        parts.push(part);
    }
    let close = src[..def_end(src, prev)].len();
    let trailing = comments(src, prev, close);
    let sep = if multi_line { format!("\n\n{base}") } else { format!("\n{base}") };
    let mut text = parts.join(&sep);
    for line in trailing {
        text.push('\n');
        text.push_str(&base);
        text.push_str(&line);
    }
    text
}

/// The position of the `}` that closes the body, from the end of its last
/// constructor.
fn def_end(src: &str, from: usize) -> usize {
    let mut i = from;
    let b = src.as_bytes();
    while b[i] != b'}' {
        if src[i..].starts_with("//") {
            i += src[i..].find('\n').unwrap_or(src.len() - i);
        } else {
            i += 1;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Kind;

    fn migrate(path: &str, text: &str) -> Outcome {
        run(&Source { path: path.into(), line: 1, kind: Kind::File, text: text.into(), start: 0, end: text.len() })
    }

    #[test]
    fn splits_a_sum() {
        let out = migrate("a.i", "(Option(..), T)\n\nOption(A) {\n\tSome(A)\n\t// none\n\tNone\n}\n\nT { X Y(a: Int) }\n");
        assert_eq!(
            out.text.unwrap(),
            "(Option, Some(..), None(..), T, X, Y)\n\nOption(A) = Some(A) | None\n\nSome(A) {\n\tSome(A)\n}\n\n// none\nNone {\n\tNone\n}\n\nT = X | Y\nX { X }\nY { Y(a: Int) }\n"
        );
    }

    #[test]
    fn renames_clashing_constructor_types() {
        let out = migrate("a.i", "Value { Int(Int) Name(String) }\nPaused { Paused(Paused) Done }\n");
        assert_eq!(
            out.text.unwrap(),
            "Value = IntCase | Name\nIntCase { Int(Int) }\nName { Name(String) }\nPaused = PausedCase | Done\nPausedCase { Paused(Paused) }\nDone { Done }\n"
        );
        assert_eq!(out.remaining, 0);
        assert!(out.manual[0].contains("drop the wrapper"));
    }
}
