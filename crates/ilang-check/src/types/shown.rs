//! A shown type (§4.7) as a tree: the type of [`super::Typed::schemes`]
//! before it is written as text, with each named type known by what it is
//! (its `TypeRef`), not by the name that a module gives it.

use crate::hir::TypeRef;

/// A type as it is shown (§4.7): simplified, with its type variables named.
/// [`ShownType::text`] writes it in the syntax of §3.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShownType {
    /// Data with labels (syntax-3 §2.3), its keys in canonical order:
    /// `None` for a position. `(a: A, b: B)`.
    Record(Vec<Option<String>>, Vec<ShownType>),
    /// The canonical source text of an Int or String literal.
    Lit(String),
    /// A built-in or custom type, with its arguments.
    Named(TypeRef, Vec<ShownType>),
    /// `(A, B)`.
    Tuple(Vec<ShownType>),
    /// `(label: A, B) -> R ^ E ! N`.
    Fn(Box<ShownFn>),
    /// A type variable, by its shown name. The variables of one shown type
    /// have different names.
    Var(String),
    /// `_`: a part with no effect, as a member of `H` in `A - H` that only
    /// counts by its type constructor (§4.4).
    Hole,
    /// `Never`, the type with no values.
    Never,
    /// `A | B`.
    Union(Vec<ShownType>),
    /// `A & B`.
    Inter(Vec<ShownType>),
    /// `A - H1 - H2`.
    Diff(Box<ShownType>, Vec<ShownType>),
}

/// A function type: its parameters with their labels, its value, and its
/// `^ E` and `! N` parts, which are `None` when they are not written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShownFn {
    pub params: Vec<(Option<String>, ShownType)>,
    pub ret: ShownType,
    pub err: Option<ShownType>,
    pub env: Option<ShownType>,
}

impl ShownType {
    /// The text in the syntax of §3, with the custom type `TypeId(i)` named
    /// `type_names[i]`, as [`super::Typed::schemes`] has it.
    pub fn text(&self, type_names: &[String]) -> String {
        self.text_with(
            &mut |r| match r {
                TypeRef::Builtin(b) => b.name().to_string(),
                TypeRef::Custom(id) => type_names
                    .get(id.0 as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("T{}", id.0)),
            },
            &mut |v| v.to_string(),
        )
    }

    /// Human text may abbreviate a scalar set; exact text never does.
    pub fn text_limited(&self, type_names: &[String], limit: usize) -> String {
        self.text_mode(type_names, Some(limit))
    }

    fn text_mode(&self, type_names: &[String], limit: Option<usize>) -> String {
        let mut ty = |r| match r {
            TypeRef::Builtin(b) => b.name().to_string(),
            TypeRef::Custom(id) => type_names
                .get(id.0 as usize)
                .cloned()
                .unwrap_or_else(|| format!("T{}", id.0)),
        };
        let mut var = |v: &str| v.to_string();
        let mut out = String::new();
        self.write(
            0,
            &mut out,
            &mut Names {
                ty: &mut ty,
                var: &mut var,
                limit,
            },
        );
        out
    }

    /// The text in the syntax of §3, with each named type named by `ty`
    /// and each type variable by `var`. Both are called in the order of
    /// the text, so `var` may number the variables by where they first
    /// come.
    pub fn text_with(
        &self,
        ty: &mut dyn FnMut(TypeRef) -> String,
        var: &mut dyn FnMut(&str) -> String,
    ) -> String {
        let mut out = String::new();
        self.write(
            0,
            &mut out,
            &mut Names {
                ty,
                var,
                limit: None,
            },
        );
        out
    }

    /// `prec` is what the place allows: 0 any type, 1 no union, 2 no
    /// intersection, 3 no difference and no function type. A part that
    /// binds less is put in `{ }`.
    fn write(&self, prec: u8, out: &mut String, n: &mut Names) {
        let open = |out: &mut String, own: u8| {
            if prec > own {
                out.push_str("{ ");
            }
        };
        let close = |out: &mut String, own: u8| {
            if prec > own {
                out.push_str(" }");
            }
        };
        let list = |xs: &[ShownType], sep: &str, p: u8, out: &mut String, n: &mut Names| {
            let mut seen = [0usize; 2];
            let counts = [false, true].map(|string| {
                xs.iter()
                    .filter(|x| matches!(x, ShownType::Lit(s) if s.starts_with('"') == string))
                    .count()
            });
            let mut written = 0;
            for x in xs {
                let kind = match x {
                    ShownType::Lit(s) => Some(usize::from(s.starts_with('"'))),
                    _ => None,
                };
                if let Some(k) = kind {
                    seen[k] += 1;
                    if n.limit.is_some_and(|limit| counts[k] > limit) && seen[k] > 10 {
                        if seen[k] == 11 {
                            if written > 0 {
                                out.push_str(sep);
                            }
                            out.push_str(&format!("… ({} more)", counts[k] - 10));
                            written += 1;
                        }
                        continue;
                    }
                }
                if written > 0 {
                    out.push_str(sep);
                }
                x.write(p, out, n);
                written += 1;
            }
        };
        match self {
            ShownType::Record(keys, parts) => {
                out.push('(');
                for (i, (l, p)) in keys.iter().zip(parts).enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    if let Some(l) = l {
                        out.push_str(l);
                        out.push_str(": ");
                    }
                    p.write(0, out, n);
                }
                out.push(')');
            }
            ShownType::Lit(s) => out.push_str(s),
            ShownType::Var(v) => out.push_str(&(n.var)(v)),
            ShownType::Never => out.push_str("Never"),
            ShownType::Hole => out.push('_'),
            ShownType::Named(r, args) => {
                out.push_str(&(n.ty)(*r));
                if !args.is_empty() {
                    out.push('(');
                    list(args, ", ", 0, out, n);
                    out.push(')');
                }
            }
            ShownType::Tuple(parts) => {
                out.push('(');
                list(parts, ", ", 0, out, n);
                out.push(')');
            }
            ShownType::Fn(f) => {
                open(out, 0);
                if let [(Some(l), p)] = f.params.as_slice()
                    && l == super::groups::DATA
                {
                    // A multi-shape function type: its parameter data
                    // types in a group (syntax three §2.4).
                    out.push_str("{ ");
                    p.write(0, out, n);
                    out.push_str(" } -> ");
                } else {
                    out.push('(');
                    for (i, (l, p)) in f.params.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        if let Some(l) = l {
                            out.push_str(l);
                            out.push_str(": ");
                        }
                        p.write(0, out, n);
                    }
                    out.push_str(") -> ");
                }
                // A function type followed by `^` or `!` is put in `{ }`.
                let part = |x: &ShownType, followed: bool, out: &mut String, n: &mut Names| {
                    let braces = followed && matches!(x, ShownType::Fn(_));
                    if braces {
                        out.push_str("{ ");
                    }
                    x.write(0, out, n);
                    if braces {
                        out.push_str(" }");
                    }
                };
                part(&f.ret, f.err.is_some() || f.env.is_some(), out, n);
                if let Some(e) = &f.err {
                    out.push_str(" ^ ");
                    part(e, f.env.is_some(), out, n);
                }
                if let Some(e) = &f.env {
                    out.push_str(" ! ");
                    part(e, false, out, n);
                }
                close(out, 0);
            }
            ShownType::Union(ms) => {
                open(out, 0);
                list(ms, " | ", 1, out, n);
                close(out, 0);
            }
            ShownType::Inter(ms) => {
                open(out, 1);
                list(ms, " & ", 2, out, n);
                close(out, 1);
            }
            ShownType::Diff(a, h) => {
                // Only the type constructors of `H` matter (§4.4).
                open(out, 2);
                a.write(2, out, n);
                if !h.is_empty() {
                    out.push_str(" - ");
                    list(h, " - ", 3, out, n);
                }
                close(out, 2);
            }
        }
    }
}

/// How the names in a text are found.
struct Names<'a> {
    ty: &'a mut dyn FnMut(TypeRef) -> String,
    var: &'a mut dyn FnMut(&str) -> String,
    limit: Option<usize>,
}

impl ShownType {
    /// Nominal identities in the complete tree, including function inputs.
    pub fn type_refs(&self, out: &mut Vec<TypeRef>) {
        match self {
            ShownType::Record(_, parts) => {
                for t in parts {
                    t.type_refs(out);
                }
            }
            ShownType::Named(ty, ts) => {
                out.push(*ty);
                for t in ts {
                    t.type_refs(out);
                }
            }
            ShownType::Tuple(ts) | ShownType::Union(ts) | ShownType::Inter(ts) => {
                for t in ts {
                    t.type_refs(out);
                }
            }
            ShownType::Fn(f) => {
                for (_, t) in &f.params {
                    t.type_refs(out);
                }
                f.ret.type_refs(out);
                for t in f.err.iter().chain(&f.env) {
                    t.type_refs(out);
                }
            }
            ShownType::Diff(t, ts) => {
                t.type_refs(out);
                for t in ts {
                    t.type_refs(out);
                }
            }
            ShownType::Lit(_) | ShownType::Var(_) | ShownType::Hole | ShownType::Never => {}
        }
    }
}
