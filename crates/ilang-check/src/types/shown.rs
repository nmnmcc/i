//! A shown type (§4.7) as a tree: the type of [`super::Typed::schemes`]
//! before it is written as text, with each named type known by what it is
//! (its `TypeRef`), not by the name that a module gives it.

use crate::hir::TypeRef;

/// A type as it is shown (§4.7): simplified, with its type variables named.
/// [`ShownType::text`] writes it in the syntax of §3.
pub enum ShownType {
    /// The canonical source text of an Int or String literal.
    Lit(String),
    /// A built-in or custom type, with its arguments.
    Named(TypeRef, Vec<ShownType>),
    /// `(A, B)`.
    Tuple(Vec<ShownType>),
    /// `(label: A, B) -> R ? E ! N`.
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
/// `? E` and `! N` parts, which are `None` when they are not written.
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
        enum Task<'a> {
            Node(&'a ShownType, u8),
            Part(&'a ShownType, bool),
            Text(&'a str),
            Owned(String),
            Type(TypeRef),
            Var(&'a str),
        }
        fn list<'a>(
            xs: &'a [ShownType],
            sep: &'a str,
            prec: u8,
            limit: Option<usize>,
        ) -> Vec<Task<'a>> {
            let mut seen = [0usize; 2];
            let counts = [false, true].map(|string| {
                xs.iter()
                    .filter(|x| matches!(x, ShownType::Lit(s) if s.starts_with('"') == string))
                    .count()
            });
            let mut items = Vec::new();
            let mut written = 0;
            for x in xs {
                let kind = match x {
                    ShownType::Lit(s) => Some(usize::from(s.starts_with('"'))),
                    _ => None,
                };
                if let Some(k) = kind {
                    seen[k] += 1;
                    if limit.is_some_and(|limit| counts[k] > limit) && seen[k] > 10 {
                        if seen[k] == 11 {
                            if written > 0 {
                                items.push(Task::Text(sep));
                            }
                            items.push(Task::Owned(format!("… ({} more)", counts[k] - 10)));
                            written += 1;
                        }
                        continue;
                    }
                }
                if written > 0 {
                    items.push(Task::Text(sep));
                }
                items.push(Task::Node(x, prec));
                written += 1;
            }
            items
        }
        let mut tasks = vec![Task::Node(self, prec)];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Text(s) => out.push_str(s),
                Task::Owned(s) => out.push_str(&s),
                Task::Type(r) => out.push_str(&(n.ty)(r)),
                Task::Var(v) => out.push_str(&(n.var)(v)),
                Task::Part(x, followed) => {
                    let braces = followed && matches!(x, ShownType::Fn(_));
                    if braces {
                        tasks.push(Task::Text(" }"));
                    }
                    tasks.push(Task::Node(x, 0));
                    if braces {
                        tasks.push(Task::Text("{ "));
                    }
                }
                Task::Node(t, prec) => match t {
                    ShownType::Lit(s) => tasks.push(Task::Text(s)),
                    ShownType::Var(v) => tasks.push(Task::Var(v)),
                    ShownType::Never => tasks.push(Task::Text("Never")),
                    ShownType::Hole => tasks.push(Task::Text("_")),
                    ShownType::Named(r, xs) => {
                        if !xs.is_empty() {
                            tasks.push(Task::Text(")"));
                            tasks.extend(list(xs, ", ", 0, n.limit).into_iter().rev());
                            tasks.push(Task::Text("("));
                        }
                        tasks.push(Task::Type(*r));
                    }
                    ShownType::Tuple(xs) => {
                        tasks.push(Task::Text(")"));
                        tasks.extend(list(xs, ", ", 0, n.limit).into_iter().rev());
                        tasks.push(Task::Text("("));
                    }
                    ShownType::Fn(f) => {
                        if prec > 0 {
                            tasks.push(Task::Text(" }"));
                        }
                        if let Some(e) = &f.env {
                            tasks.push(Task::Part(e, false));
                            tasks.push(Task::Text(" ! "));
                        }
                        if let Some(e) = &f.err {
                            tasks.push(Task::Part(e, f.env.is_some()));
                            tasks.push(Task::Text(" ? "));
                        }
                        tasks.push(Task::Part(&f.ret, f.err.is_some() || f.env.is_some()));
                        tasks.push(Task::Text(") -> "));
                        for (i, (label, p)) in f.params.iter().enumerate().rev() {
                            tasks.push(Task::Node(p, 0));
                            if let Some(label) = label {
                                tasks.push(Task::Text(": "));
                                tasks.push(Task::Text(label));
                            }
                            if i > 0 {
                                tasks.push(Task::Text(", "));
                            }
                        }
                        tasks.push(Task::Text("("));
                        if prec > 0 {
                            tasks.push(Task::Text("{ "));
                        }
                    }
                    ShownType::Union(xs) | ShownType::Inter(xs) => {
                        let (own, sep, child) = if matches!(t, ShownType::Union(_)) {
                            (0, " | ", 1)
                        } else {
                            (1, " & ", 2)
                        };
                        if prec > own {
                            tasks.push(Task::Text(" }"));
                        }
                        tasks.extend(list(xs, sep, child, n.limit).into_iter().rev());
                        if prec > own {
                            tasks.push(Task::Text("{ "));
                        }
                    }
                    ShownType::Diff(a, xs) => {
                        if prec > 2 {
                            tasks.push(Task::Text(" }"));
                        }
                        if !xs.is_empty() {
                            tasks.extend(list(xs, " - ", 3, n.limit).into_iter().rev());
                            tasks.push(Task::Text(" - "));
                        }
                        tasks.push(Task::Node(a, 2));
                        if prec > 2 {
                            tasks.push(Task::Text("{ "));
                        }
                    }
                },
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
        let mut pending = vec![self];
        while let Some(t) = pending.pop() {
            match t {
                ShownType::Named(r, xs) => {
                    out.push(*r);
                    pending.extend(xs.iter().rev());
                }
                ShownType::Tuple(xs) | ShownType::Union(xs) | ShownType::Inter(xs) => {
                    pending.extend(xs.iter().rev())
                }
                ShownType::Fn(f) => {
                    if let Some(t) = &f.env {
                        pending.push(t);
                    }
                    if let Some(t) = &f.err {
                        pending.push(t);
                    }
                    pending.push(&f.ret);
                    pending.extend(f.params.iter().rev().map(|(_, t)| t));
                }
                ShownType::Diff(a, xs) => {
                    pending.extend(xs.iter().rev());
                    pending.push(a);
                }
                ShownType::Lit(_) | ShownType::Var(_) | ShownType::Hole | ShownType::Never => {}
            }
        }
    }
}
