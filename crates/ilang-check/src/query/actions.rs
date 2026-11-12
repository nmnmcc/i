//! What editors offer to change: imports to add and to remove, unused
//! bindings, the missing clauses of a match, type annotations, and the
//! inferred types to show in the text (inlay hints).

use super::nav::{Node, visit_module};
use super::*;

/// A change that fixes or improves the code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeFix {
    pub title: String,
    pub edits: Vec<Edit>,
    /// The fix of a problem, which an editor may apply by itself.
    pub preferred: bool,
}

/// The clauses that a match lacks to cover every value (§7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingClauses {
    /// The patterns of the clauses to add, as they are written.
    pub patterns: Vec<String>,
    pub edit: Edit,
}

/// An inferred type to show after a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlayHint {
    /// Where the hint goes: the end of the name.
    pub offset: u32,
    /// `: Type`.
    pub label: String,
    /// The name is a parameter.
    pub parameter: bool,
}

impl Query<'_> {
    /// The edit that imports the module with the written `path` into `m`,
    /// with `name` put in by name when it is given.
    pub fn import_edit(&self, m: ModuleId, path: &str, name: Option<&str>) -> Edit {
        let lisp = self.is_lisp(m);
        let source = self.source_of(m);
        let text = self.written_text(m);
        let ast = self.ast(m);
        // A module that is imported already gets the name in its import:
        // one module is imported once (§2.3).
        if let Some(n) = name
            && let Some(item) = ast.imports.iter().find(|i| {
                i.path
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
                    == path
            })
        {
            let (Some(first), Some(_)) = (item.path.first(), item.path.last()) else {
                unreachable!("an import has a path")
            };
            let edit = |at: u32, text: String| Edit {
                source,
                span: Span::new(at, at),
                text,
            };
            return match (item.names.last(), lisp) {
                (Some(last), false) => edit(last.span.end, format!(", {n}")),
                (Some(last), true) => edit(last.span.end, format!(" {n}")),
                (None, false) => edit(first.span.start, format!("({n}) = ")),
                (None, true) => {
                    // `(/ a b)` or `(= alias (/ a b))`: the names go before
                    // the path, which starts at its `(`.
                    let open = text[..first.span.start as usize]
                        .rfind('(')
                        .unwrap_or(first.span.start as usize) as u32;
                    if item.alias.is_some() {
                        edit(open, format!("(, {n}) "))
                    } else {
                        let whole = &text[open as usize..item.span.end as usize];
                        Edit {
                            source,
                            span: Span::new(open, item.span.end),
                            text: format!("(= (, {n}) {whole})"),
                        }
                    }
                }
            };
        }
        let item = match (lisp, name) {
            (false, Some(n)) => format!("({n}) = {path}"),
            (false, None) => path.to_string(),
            (true, Some(n)) => format!("(= (, {n}) (/ {}))", path.replace('/', " ")),
            (true, None) => format!("(/ {})", path.replace('/', " ")),
        };
        if let Some(group) = ast.imports_span {
            // Before the `)` that ends the group.
            let close = group.end - 1;
            let inner = &text[group.start as usize..close as usize];
            if lisp {
                return Edit {
                    source,
                    span: Span::new(close, close),
                    text: format!(" {item}"),
                };
            }
            if inner.contains('\n') {
                let line_start = text[..close as usize].rfind('\n').map_or(0, |i| i + 1) as u32;
                let indent = ast
                    .imports
                    .last()
                    .map(|i| {
                        let ls = text[..i.span.start as usize]
                            .rfind('\n')
                            .map_or(0, |x| x + 1);
                        text[ls..i.span.start as usize].to_string()
                    })
                    .unwrap_or_else(|| "\t".into());
                return Edit {
                    source,
                    span: Span::new(line_start, line_start),
                    text: format!("{indent}{item}\n"),
                };
            }
            return Edit {
                source,
                span: Span::new(close, close),
                text: format!(", {item}"),
            };
        }
        // A new group, after the comments of the module.
        let mut at = 0usize;
        for line in text.split_inclusive('\n') {
            if line.starts_with("////") {
                at += line.len();
            } else {
                break;
            }
        }
        let group = if lisp {
            format!("(, {item})\n")
        } else {
            format!("({item})\n")
        };
        let blank = if ast.exports_span.is_some() { "" } else { "\n" };
        Edit {
            source,
            span: Span::new(at as u32, at as u32),
            text: format!("{group}{blank}"),
        }
    }

    /// The imports that add a name that is not known at `span`: each
    /// module that exports a name with this text, or for `name.x`, each
    /// module whose path ends with `name`.
    pub fn missing_import_fixes(&self, m: ModuleId, span: Span) -> Vec<CodeFix> {
        let text = self.written_text(m);
        let Some(name) = text.get(span.start as usize..span.end as usize) else {
            return Vec::new();
        };
        let qualified = text.as_bytes().get(span.end as usize) == Some(&b'.');
        let p = self.program;
        let imported: Vec<ModuleId> = p.module(m).import_names.iter().map(|(_, t)| *t).collect();
        let mut out = Vec::new();
        for (i, module) in p.modules.iter().enumerate() {
            let t = ModuleId(i as u32);
            if t == m {
                continue;
            }
            let Some(path) = self.import_path(m, t) else {
                continue;
            };
            if qualified {
                if path.rsplit('/').next() == Some(name) && !imported.contains(&t) {
                    out.push(CodeFix {
                        title: format!("Import `{path}`"),
                        edits: vec![self.import_edit(m, &path, None)],
                        preferred: true,
                    });
                }
                continue;
            }
            let exported = module.defs.iter().any(|d| d.exported && d.name == name)
                || module.types.iter().any(|&ty| {
                    let def = p.ty(ty);
                    def.exported && def.name == name
                        || def.constructors_exported
                            && matches!(&def.body, TypeBody::Custom(cs) if cs.iter().any(|c| c.name == name))
                });
            if exported {
                out.push(CodeFix {
                    title: format!("Import `{name}` from `{path}`"),
                    edits: vec![self.import_edit(m, &path, Some(name))],
                    preferred: out.is_empty(),
                });
            }
        }
        out
    }

    /// The imports of a module that nothing in it uses: the span of each
    /// import item, and its path.
    pub fn unused_imports(&self, m: ModuleId) -> Vec<(Span, String)> {
        let ast = self.ast(m);
        let w = self.walk(m);
        let mut out = Vec::new();
        for item in &ast.imports {
            let alias = item.alias.as_ref().or(item.path.last());
            let Some(t) = alias.and_then(|a| self.import_module(m, &a.name)) else {
                continue;
            };
            let used = w.occ.iter().any(|(s, target)| {
                if contains(item.span, s.start) {
                    return false;
                }
                match target {
                    Target::Module(x) => *x == t,
                    Target::Def(d) => d.module == t,
                    Target::Type(TypeRef::Custom(ty)) | Target::Ctor(CtorRef { ty, .. }) => {
                        self.program.ty(*ty).module == t
                    }
                    _ => false,
                }
            });
            if !used {
                let path = item
                    .path
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((item.span, path));
            }
        }
        out
    }

    /// The edit that removes an import item, or the whole group when it is
    /// the only one.
    pub fn remove_import_edit(&self, m: ModuleId, item: Span) -> Edit {
        let ast = self.ast(m);
        let text = self.written_text(m);
        let source = self.source_of(m);
        if ast.imports.len() == 1
            && let Some(group) = ast.imports_span
        {
            return Edit {
                source,
                span: with_line_end(text, group),
                text: String::new(),
            };
        }
        // The item and the separator after it, or before it for the last.
        let b = text.as_bytes();
        let mut end = item.end as usize;
        while end < b.len() && matches!(b[end], b' ' | b'\t') {
            end += 1;
        }
        if end < b.len() && (b[end] == b',' || b[end] == b'\n') {
            end += 1;
            while end < b.len() && matches!(b[end], b' ' | b'\t') {
                end += 1;
            }
            let mut start = item.start as usize;
            if b.get(end - 1) == Some(&b'\t')
                || b.get(end - 1) == Some(&b' ')
                || text[item.end as usize..end].contains('\n')
            {
                while start > 0 && matches!(b[start - 1], b' ' | b'\t') {
                    start -= 1;
                }
                if text[item.end as usize..end].contains('\n') {
                    end = item.end as usize
                        + text[item.end as usize..].find('\n').map_or(0, |i| i + 1);
                }
            }
            return Edit {
                source,
                span: Span::new(start as u32, end as u32),
                text: String::new(),
            };
        }
        let mut start = item.start as usize;
        while start > 0 && matches!(b[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        if start > 0 && b[start - 1] == b',' {
            start -= 1;
        }
        Edit {
            source,
            span: Span::new(start as u32, item.end),
            text: String::new(),
        }
    }

    /// The edit that removes the top-level binding of the unused name at
    /// `span`, when it binds no other name.
    pub fn remove_binding_fix(&self, m: ModuleId, span: Span) -> Option<CodeFix> {
        let p = self.program;
        let module = p.module(m);
        let d = module.defs.iter().position(|d| d.span == span)?;
        let DefKind::Binding { binding, .. } = module.defs[d].kind else {
            return None;
        };
        if module.bindings[binding as usize].defs.len() != 1 {
            return None;
        }
        let item = self.item_around(m, span)?;
        let text = self.written_text(m);
        Some(CodeFix {
            title: format!("Remove the unused binding `{}`", module.defs[d].name),
            edits: vec![Edit {
                source: self.source_of(m),
                span: with_doc_and_line_end(text, item, &self.ast(m)),
                text: String::new(),
            }],
            preferred: true,
        })
    }

    /// The clauses that the match around `span` lacks (§7.4): one for each
    /// constructor of the subject type that no clause without a guard
    /// covers, or `_` when the subject has no constructors.
    pub fn missing_clauses(&self, m: ModuleId, span: Span) -> Option<MissingClauses> {
        let p = self.program;
        let mut case: Option<&Expr> = None;
        visit_module(p, m, &mut |n| {
            if let Node::Expr(e) = n
                && let ExprKind::Case { subject, .. } = &e.kind
                && (e.span.start <= span.start && span.start <= e.span.end
                    || subject.span.start <= span.start && span.start <= subject.span.end)
                && case.is_none_or(|c| c.span.start <= e.span.start)
            {
                case = Some(e);
            }
        });
        let case = case?;
        let ExprKind::Case { clauses, subject } = &case.kind else {
            unreachable!("a case")
        };
        let text = self.written_text(m);
        if case.span.end as usize > text.len() {
            return None;
        }
        let lisp = self.is_lisp(m);
        let mut types: Vec<TypeId> = Vec::new();
        for c in clauses {
            for pat in &c.patterns {
                if let PatternKind::Constructor { ctor, .. } = &pat.kind
                    && !types.contains(&ctor.ty)
                {
                    types.push(ctor.ty);
                }
            }
        }
        if types.is_empty() {
            types = self.subject_types(m, subject, clauses);
        }
        let covered = |ctor: CtorRef| {
            clauses.iter().any(|c| {
                c.guard.is_none()
                    && c.patterns.iter().any(|pat| match &pat.kind {
                        PatternKind::Constructor { ctor: x, args, .. } => {
                            *x == ctor && args.iter().all(irrefutable)
                        }
                        _ => false,
                    })
            })
        };
        let mut patterns = Vec::new();
        for &t in &types {
            let TypeBody::Custom(cs) = &p.ty(t).body else {
                continue;
            };
            let prefix = self.ctor_prefix(m, t).unwrap_or_default();
            for (i, c) in cs.iter().enumerate() {
                if covered(CtorRef {
                    ty: t,
                    index: i as u32,
                }) {
                    continue;
                }
                patterns.push(if c.fields.is_empty() {
                    format!("{prefix}{}", c.name)
                } else {
                    format!("{prefix}{}(..)", c.name)
                });
            }
        }
        if types.is_empty() {
            patterns.push("_".into());
        }
        if patterns.is_empty() {
            return None;
        }
        // Before the `}` that ends the clauses, on lines of their own.
        let close = text[..case.span.end as usize].rfind('}')? as u32;
        let indent = clauses
            .last()
            .map(|c| {
                let ls = text[..c.span.start as usize]
                    .rfind('\n')
                    .map_or(0, |x| x + 1);
                text[ls..c.span.start as usize].to_string()
            })
            .filter(|s| s.trim().is_empty())
            .unwrap_or_else(|| "\t".into());
        let line_start = text[..close as usize].rfind('\n').map_or(0, |i| i + 1) as u32;
        let own_line = text[line_start as usize..close as usize].trim().is_empty();
        let body = |pat: &str| {
            let clause = format!("{pat} -> panic(\"todo\")");
            if lisp {
                lisp_clause(&clause).unwrap_or(clause)
            } else {
                clause
            }
        };
        let mut new = String::new();
        let at = if own_line && !lisp {
            for pat in &patterns {
                new.push_str(&format!("{indent}{}\n", body(pat)));
            }
            line_start
        } else {
            for pat in &patterns {
                new.push(' ');
                new.push_str(&body(pat));
            }
            new.push(' ');
            close
        };
        Some(MissingClauses {
            patterns,
            edit: Edit {
                source: self.source_of(m),
                span: Span::new(at, at),
                text: new,
            },
        })
    }

    /// The annotation `: Type` that can be written after the unannotated
    /// name of a binding or a parameter at `offset`, with the type that
    /// inference found: only when that type can be written in the module,
    /// with no type variables.
    pub fn annotation_fix(&self, m: ModuleId, offset: u32) -> Option<CodeFix> {
        let (owner, span, target) = self.target_at_offset(m, offset)?;
        if owner != m {
            return None;
        }
        let binder = self.unannotated_binder(m, offset)?;
        if binder.span != span {
            return None;
        }
        let ty = match target {
            Target::Def(d) => self.def_type(d)?,
            Target::Local(v) => self.local_type(m, v, span.start)?,
            _ => return None,
        };
        if !self.writable(m, &ty) {
            return None;
        }
        let edits = if self.is_lisp(m) {
            let name = &self.written_text(m)[span.start as usize..span.end as usize];
            vec![Edit {
                source: self.source_of(m),
                span,
                text: format!("(: {name} {})", lisp_type(&ty)?),
            }]
        } else {
            vec![Edit {
                source: self.source_of(m),
                span: Span::new(span.end, span.end),
                text: format!(": {ty}"),
            }]
        };
        Some(CodeFix {
            title: format!("Add the type annotation `{ty}`"),
            edits,
            preferred: false,
        })
    }

    /// The binder of a binding or parameter pattern that is only a name.
    fn unannotated_binder(&self, m: ModuleId, offset: u32) -> Option<Binder> {
        let mut found = None;
        let mut take = |p: &Pattern| {
            if let PatternKind::Var(b) = &p.kind
                && contains(b.span, offset)
            {
                found = Some(b.clone());
            }
        };
        visit_module(self.program, m, &mut |n| match n {
            Node::Binding(b) => take(&b.pattern),
            Node::Expr(Expr {
                kind: ExprKind::Fn(f),
                ..
            }) => {
                for param in &f.params {
                    if let Param::Pattern { pattern, .. } = param {
                        take(pattern);
                    }
                }
            }
            _ => {}
        });
        found
    }

    /// A shown type names only types that module `m` can name, and no type
    /// variables.
    fn writable(&self, m: ModuleId, ty: &str) -> bool {
        let p = self.program;
        let module = p.module(m);
        let mut names: HashSet<String> = module
            .types
            .iter()
            .chain(&module.imported_types)
            .map(|t| p.ty(*t).name.clone())
            .collect();
        if let Some(prelude) = p.prelude {
            names.extend(
                p.module(prelude)
                    .types
                    .iter()
                    .map(|t| p.ty(*t).name.clone()),
            );
        }
        names.extend(BuiltinType::ALL.iter().map(|b| b.name().to_string()));
        let mut word = String::new();
        let mut words = Vec::new();
        let mut quoted = false;
        for c in ty.chars().chain(std::iter::once(' ')) {
            if c == '"' {
                quoted = !quoted;
            }
            if !quoted && (c.is_ascii_alphanumeric() || c == '_' || c == '.') {
                word.push(c);
            } else if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        }
        words
            .iter()
            .filter(|w| w.starts_with(|c: char| c.is_ascii_uppercase()))
            .all(|w| names.contains(w))
            && !ty.contains('…')
    }

    /// The inferred types of the unannotated names of bindings and
    /// parameters in `range`, but not of names bound to literals.
    pub fn inlay_hints(&self, m: ModuleId, range: Span) -> Vec<InlayHint> {
        let p = self.program;
        let text = self.written_text(m);
        let mut binders: Vec<(Binder, bool)> = Vec::new();
        visit_module(p, m, &mut |n| match n {
            Node::Binding(b) => {
                let literal = matches!(
                    b.value.kind,
                    ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_) | ExprKind::Fn(_)
                ) || matches!(&b.value.kind, ExprKind::Name(ValueRef::Ctor(_)));
                if let PatternKind::Var(binder) = &b.pattern.kind
                    && !literal
                {
                    binders.push((binder.clone(), false));
                }
            }
            Node::Expr(Expr {
                kind: ExprKind::Fn(f),
                ..
            }) => {
                for param in &f.params {
                    if let Param::Pattern { pattern, .. } = param
                        && let PatternKind::Var(binder) = &pattern.kind
                    {
                        binders.push((binder.clone(), true));
                    }
                }
            }
            _ => {}
        });
        let mut out = Vec::new();
        for (b, parameter) in binders {
            if b.span.end as usize > text.len()
                || b.span.start < range.start
                || b.span.end > range.end
            {
                continue;
            }
            if b.name.starts_with('%')
                || text.get(b.span.start as usize..b.span.end as usize) != Some(b.name.as_str())
            {
                continue;
            }
            let ty = match self.def_of_var(m, b.var) {
                Some(d) => self.def_type(d),
                None => self.local_type(m, b.var, b.span.start),
            };
            let Some(ty) = ty else { continue };
            if ty.contains('\n') || ty.len() > 80 {
                continue;
            }
            out.push(InlayHint {
                offset: b.span.end,
                label: format!(": {ty}"),
                parameter,
            });
        }
        out.sort_by_key(|h| h.offset);
        out.dedup_by_key(|h| h.offset);
        out
    }
}

/// A type shown in the syntax of §3, in the syntax of the Lisp surface: as
/// the printer of that surface writes it in an annotation.
fn lisp_type(ty: &str) -> Option<String> {
    let (module, diags) = ilang_syntax::parse_module(&format!("x: {ty} = 0\n"));
    if !diags.is_empty() {
        return None;
    }
    let printed = ilang_syntax::lisp::print_module(&module);
    let rest = printed.trim().strip_prefix("(= (: x ")?;
    Some(rest.strip_suffix(") 0)")?.to_string())
}

/// A clause `p -> e` of the syntax of §6, in the syntax of the Lisp surface,
/// as its printer writes it.
pub(crate) fn lisp_clause(clause: &str) -> Option<String> {
    let (module, diags) =
        ilang_syntax::parse_module(&format!("f = (s) {{ s |> {{ {clause}  _ -> 0 }} }}\n"));
    if !diags.is_empty() {
        return None;
    }
    let printed = ilang_syntax::lisp::print_module(&module);
    let start = printed.find("(->")?;
    let b = printed.as_bytes();
    let mut depth = 0i32;
    let mut quoted = false;
    for (i, &c) in b.iter().enumerate().skip(start) {
        match c {
            b'"' => quoted = !quoted,
            b'\\' if quoted => {}
            b'(' | b'[' | b'{' if !quoted => depth += 1,
            b')' | b']' | b'}' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return Some(printed[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// A pattern of the syntax of §7, in the syntax of the Lisp surface.
pub(crate) fn lisp_pattern(pattern: &str) -> Option<String> {
    let clause = lisp_clause(&format!("{pattern} -> 0"))?;
    let inner = clause.strip_prefix("(-> ")?.strip_suffix(" 0)")?;
    Some(inner.to_string())
}

/// A pattern that matches every value of its type.
fn irrefutable(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Var(_) | PatternKind::Discard => true,
        PatternKind::Named { pattern, .. } => irrefutable(pattern),
        PatternKind::Tuple(ps) => ps.iter().all(irrefutable),
        _ => false,
    }
}

/// A span with the line end after it, when only spaces are between.
fn with_line_end(text: &str, span: Span) -> Span {
    let rest = &text[span.end as usize..];
    let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let mut end = span.end as usize + spaces;
    if text[end..].starts_with("\r\n") {
        end += 2;
    } else if text[end..].starts_with('\n') {
        end += 1;
    }
    // And one empty line after it.
    if text[end..].starts_with('\n') {
        end += 1;
    }
    Span::new(span.start, end as u32)
}

/// An item with its documentation comment before it and its line end.
fn with_doc_and_line_end(text: &str, item: Span, ast: &ast::Module) -> Span {
    let mut start = item.start as usize;
    let has_doc = ast
        .items
        .iter()
        .any(|i| i.span == item && !i.doc.is_empty());
    if has_doc {
        let mut ls = text[..start].rfind('\n').map_or(0, |i| i + 1);
        loop {
            if ls == 0 {
                break;
            }
            let prev = text[..ls - 1].rfind('\n').map_or(0, |i| i + 1);
            let line = text[prev..ls - 1].trim_start();
            if line.starts_with("///") {
                start = prev;
                ls = prev;
            } else {
                break;
            }
        }
    }
    with_line_end(text, Span::new(start as u32, item.end))
}
