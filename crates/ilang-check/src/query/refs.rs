//! References and rename: every place that names the same thing as a name,
//! found by its identity in the HIR ([`Target`]), never by its text.
//!
//! As in the Gleam language server, the places are found by walking the
//! resolved tree of every module (`Query::walk`), and a rename edits each of
//! them. A rename is refused for what the package does not define (a
//! dependency, the library `i`, a built-in name), for a name that macro
//! expansion produced, for a new name of the wrong kind, and for a new name
//! that another name already has where it would be seen.

use super::*;

/// A place that names a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference {
    pub location: Location,
    /// The place that defines the target.
    pub definition: bool,
    /// The binder of a constructor pattern part that is also the label of
    /// a field (`User(name, ..)`): a name of the field and of a local.
    pub label_binder: bool,
}

/// A change of the text of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub source: SourceId,
    pub span: Span,
    pub text: String,
}

/// Why a rename cannot be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameError(pub String);

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What may be renamed at a place.
pub(crate) struct Renamable {
    pub owner: ModuleId,
    pub span: Span,
    pub target: Target,
}

impl Query<'_> {
    /// The target at a place, with the module whose HIR has it and the span
    /// of the name in the file of `m`.
    pub fn target_at_offset(&self, m: ModuleId, offset: u32) -> Option<(ModuleId, Span, Target)> {
        let (owner, span, _, target) = self.target_at(m, offset)?;
        Some((owner, span, self.canonical(owner, target)))
    }

    /// A local name that a top-level pattern binds is the top-level name.
    pub(crate) fn canonical(&self, m: ModuleId, target: Target) -> Target {
        match target {
            Target::Local(v) => match self.def_of_var(m, v) {
                Some(d) => Target::Def(d),
                None => Target::Local(v),
            },
            t => t,
        }
    }

    /// The modules of the packages that are not dependencies: the package
    /// being checked and its tests.
    pub fn own_modules(&self) -> Vec<ModuleId> {
        (0..self.program.modules.len() as u32)
            .map(ModuleId)
            .filter(|m| self.is_own(*m))
            .collect()
    }

    pub(crate) fn is_own(&self, m: ModuleId) -> bool {
        Some(self.program.module(m).package) == self.graph.root
    }

    fn same(&self, m: ModuleId, found: &Target, target: &Target) -> bool {
        match (found, target) {
            (Target::Field(a), Target::Field(b)) => a.iter().any(|x| b.contains(x)),
            (found, target) => self.canonical(m, found.clone()) == *target,
        }
    }

    /// Every place in `modules` that names `target`, and the place that
    /// defines it, in the order of the modules and of the text. A local
    /// name is only looked for in its own module.
    pub fn references(&self, target: &Target, owner: ModuleId, modules: &[ModuleId]) -> Vec<Reference> {
        let target = &self.canonical(owner, target.clone());
        let definition = self.definition(owner, target);
        let only: Vec<ModuleId>;
        let modules = match target {
            Target::Local(_) | Target::TyVar(_) => {
                only = vec![owner];
                &only
            }
            _ => modules,
        };
        let mut out: Vec<Reference> = Vec::new();
        for &m in modules {
            let w = self.walk(m);
            let labels = w.labels.iter().map(|(s, t)| (s, t, true));
            for (span, found, label_binder) in w.occ.iter().map(|(s, t)| (s, t, false)).chain(labels) {
                if span.start >= span.end || !self.same(m, found, target) {
                    continue;
                }
                let Some(location) = self.written_place(m, *span) else {
                    continue;
                };
                let r = Reference {
                    location,
                    definition: definition.is_some_and(|d| d.source == location.source && d.span == location.span),
                    label_binder,
                };
                if !out.iter().any(|o| o.location == r.location) {
                    out.push(r);
                }
            }
        }
        // A module is defined by its file, not by a name in it.
        if let Some(d) = definition
            && !matches!(target, Target::Module(_))
            && !out.iter().any(|r| r.definition)
            && d.span.start < d.span.end
        {
            out.push(Reference {
                location: d,
                definition: true,
                label_binder: false,
            });
        }
        out.sort_by_key(|r| (r.location.module, r.location.span.start));
        out
    }

    /// The place in a written file of a span in the HIR of a module: the
    /// span itself, or for expanded code, the written argument of a macro
    /// call that it came from. `None` for what a macro made.
    pub(crate) fn written_place(&self, m: ModuleId, span: Span) -> Option<Location> {
        let p = self.program;
        match p.origin(m, span) {
            None => Some(Location {
                module: m,
                source: p.module(m).source,
                span,
            }),
            Some(origin) if origin.template.is_none() => {
                let module = self.module_of_source(origin.source)?;
                Some(Location {
                    module,
                    source: origin.source,
                    span: origin.span,
                })
            }
            Some(_) => None,
        }
    }

    /// The places in the module `m` that name the same as the name at
    /// `offset`, for highlighting: the definition is a write.
    pub fn highlights(&self, m: ModuleId, offset: u32) -> Vec<Reference> {
        let Some((owner, _, target)) = self.target_at_offset(m, offset) else {
            return Vec::new();
        };
        if matches!(target, Target::Literal(_)) {
            return Vec::new();
        }
        let source = self.source_of(m);
        self.references(&target, owner, &[m])
            .into_iter()
            .filter(|r| r.location.source == source)
            .collect()
    }

    /// The name at a place, when it may be renamed: its span, and its text.
    pub fn prepare_rename(&self, m: ModuleId, offset: u32) -> Result<Option<(Span, String)>, RenameError> {
        let Some(r) = self.renamable(m, offset)? else {
            return Ok(None);
        };
        let text = self.written_text(m);
        let name = text
            .get(r.span.start as usize..r.span.end as usize)
            .unwrap_or_default()
            .to_string();
        Ok(Some((r.span, name)))
    }

    pub(crate) fn renamable(&self, m: ModuleId, offset: u32) -> Result<Option<Renamable>, RenameError> {
        let Some((owner, span, target)) = self.target_at_offset(m, offset) else {
            return Ok(None);
        };
        let p = self.program;
        let refuse = |s: String| Err(RenameError(s));
        match &target {
            Target::Literal(_) => return Ok(None),
            Target::Type(TypeRef::Builtin(b)) => {
                return refuse(format!("`{}` is a built-in type and cannot be renamed", b.name()));
            }
            _ => {}
        }
        if owner != m || span.end as usize > self.written_text(m).len() {
            return refuse("this name was produced by a macro expansion and cannot be renamed".into());
        }
        let defined = match &target {
            Target::Module(t) => Some(*t),
            Target::Local(_) | Target::TyVar(_) => Some(owner),
            _ => self.definition(owner, &target).map(|l| l.module),
        };
        let Some(defined) = defined else {
            return refuse("this name has no definition that can be renamed".into());
        };
        if let Target::Module(_) = target {
            // The name of a module in this module: its import.
        } else if !self.is_own(defined) {
            let package = &self.graph.packages[p.module(defined).package];
            let module = &p.module(defined).path;
            return if Some(defined) == p.prelude || p.module(defined).package == self.graph.library {
                refuse(format!("`{module}` is in the library `i`: its names are built in and cannot be renamed"))
            } else {
                refuse(format!(
                    "this name is defined in `{module}` of the dependency `{}`, which cannot be renamed from here",
                    package.name
                ))
            };
        }
        if let Some(d) = self.definition(owner, &target)
            && !matches!(target, Target::Module(_))
            && (p.origin(d.module, d.span).is_some_and(|o| o.template.is_some())
                || d.span.end as usize > self.written_text(d.module).len())
        {
            return refuse("this name was produced by a macro expansion and cannot be renamed".into());
        }
        Ok(Some(Renamable { owner, span, target }))
    }

    /// The edits that rename the name at a place to `new_name`, in every
    /// module of the package.
    pub fn rename(&self, m: ModuleId, offset: u32, new_name: &str) -> Result<Vec<Edit>, RenameError> {
        let Some(r) = self.renamable(m, offset)? else {
            return Err(RenameError("there is no name here to rename".into()));
        };
        let upper = matches!(r.target, Target::Ctor(_) | Target::Type(_) | Target::TyVar(_));
        let valid = if upper { is_up_name(new_name) } else { ilang_package::is_name(new_name) };
        if !valid {
            let kind = if upper {
                "an UpName (a capital letter, then letters and digits)"
            } else {
                "a name (a lower-case letter or `_`, then lower-case letters, digits and `_`)"
            };
            return Err(RenameError(format!("`{new_name}` is not a valid new name: it must be {kind}")));
        }
        let old = self.written_text(m)[r.span.start as usize..r.span.end as usize].to_string();
        if old == new_name {
            return Ok(Vec::new());
        }
        if let Target::Module(t) = r.target {
            return self.rename_module(m, t, new_name);
        }
        let refs = self.references(&r.target, r.owner, &self.own_modules());
        self.check_conflicts(&r, &refs, new_name)?;
        let mut edits = Vec::new();
        for reference in refs {
            let loc = reference.location;
            let lisp = self.is_lisp(loc.module);
            let text = &self.written_text(loc.module)[loc.span.start as usize..loc.span.end as usize];
            let edit = match (&r.target, reference.label_binder) {
                // `User(name, ..)` binds `name` and matches the field
                // `name`: keep the local, give the new label.
                (Target::Field(_), true) => {
                    if lisp {
                        format!("(= {text} {new_name})")
                    } else {
                        format!("{text} = {new_name}")
                    }
                }
                // Keep the label, give the new local name.
                (Target::Local(_), _) if self.is_label_binder(loc.module, loc.span) => {
                    if lisp {
                        format!("(= {new_name} {text})")
                    } else {
                        format!("{new_name} = {text}")
                    }
                }
                _ => new_name.to_string(),
            };
            edits.push(Edit {
                source: loc.source,
                span: loc.span,
                text: edit,
            });
        }
        Ok(edits)
    }

    fn is_label_binder(&self, m: ModuleId, span: Span) -> bool {
        self.walk(m).labels.iter().any(|(s, _)| *s == span)
    }

    /// Renames the name that an import gives to a module, in that module.
    fn rename_module(&self, m: ModuleId, t: ModuleId, new_name: &str) -> Result<Vec<Edit>, RenameError> {
        let source = self.source_of(m);
        if self.visible_top_level(m).iter().any(|n| n == new_name) {
            return Err(RenameError(format!(
                "`{new_name}` is already a name in this module: the module would be covered by it"
            )));
        }
        let ast = self.ast(m);
        let mut edits = Vec::new();
        for item in &ast.imports {
            let alias = item.alias.as_ref().or(item.path.last());
            if alias.and_then(|a| self.import_module(m, &a.name)) != Some(t) {
                continue;
            }
            match &item.alias {
                Some(a) => edits.push(Edit {
                    source,
                    span: a.span,
                    text: new_name.to_string(),
                }),
                None if self.is_lisp(m) => {
                    let text = &self.written_text(m)[item.span.start as usize..item.span.end as usize];
                    edits.push(Edit {
                        source,
                        span: item.span,
                        text: format!("(= {new_name} {text})"),
                    });
                }
                None => edits.push(Edit {
                    source,
                    span: Span::new(item.span.start, item.span.start),
                    text: format!("{new_name} = "),
                }),
            }
        }
        let w = self.walk(m);
        let path_spans: Vec<Span> = ast
            .imports
            .iter()
            .filter_map(|i| Some(i.path.first()?.span.to(i.path.last()?.span)))
            .collect();
        for (span, target) in &w.occ {
            if *target == Target::Module(t)
                && !path_spans.contains(span)
                && !ast.imports.iter().any(|i| i.alias.as_ref().is_some_and(|a| a.span == *span))
                && span.end as usize <= self.written_text(m).len()
            {
                edits.push(Edit {
                    source,
                    span: *span,
                    text: new_name.to_string(),
                });
            }
        }
        // A local name with the new name where the module is used would
        // cover it.
        for e in &edits {
            if self.local_names_at(m, e.span.start).iter().any(|b| b.name == new_name) {
                return Err(RenameError(format!(
                    "`{new_name}` is a local name where the module is used: it would cover the module"
                )));
            }
        }
        edits.sort_by_key(|e| e.span.start);
        edits.dedup();
        Ok(edits)
    }

    /// The names at the top level of a module: its own, its imports, and
    /// the names that the imports put in.
    pub(crate) fn visible_top_level(&self, m: ModuleId) -> Vec<String> {
        let p = self.program;
        let module = p.module(m);
        let mut out: Vec<String> = module.defs.iter().map(|d| d.name.clone()).collect();
        for &t in &module.types {
            if let TypeBody::Custom(cs) = &p.ty(t).body {
                out.extend(cs.iter().map(|c| c.name.clone()));
            }
        }
        out.extend(module.import_names.iter().map(|(n, _)| n.clone()));
        for item in &self.ast(m).imports {
            out.extend(item.names.iter().map(|n| n.name.clone()));
        }
        out
    }

    /// The types at the top level of a module: its own and the imported.
    fn visible_types(&self, m: ModuleId) -> Vec<String> {
        let p = self.program;
        let module = p.module(m);
        module
            .types
            .iter()
            .chain(&module.imported_types)
            .map(|t| p.ty(*t).name.clone())
            .collect()
    }

    /// The local names in scope at an offset of a module.
    pub(crate) fn local_names_at(&self, m: ModuleId, offset: u32) -> Vec<Binder> {
        let mut scope = Vec::new();
        Scope {
            offset,
            out: &mut scope,
            direct: Vec::new(),
        }
        .module(self.program.module(m));
        scope
    }

    fn check_conflicts(&self, r: &Renamable, refs: &[Reference], new: &str) -> Result<(), RenameError> {
        let p = self.program;
        let conflict = |what: String| Err(RenameError(format!("cannot rename to `{new}`: {what}")));
        match &r.target {
            Target::Local(v) => {
                // A name of the same name in scope where it is used: the
                // new name would be covered by it, or cover it.
                for reference in refs {
                    let loc = reference.location;
                    if loc.module != r.owner {
                        continue;
                    }
                    let at = self.local_names_at(loc.module, loc.span.start);
                    if at.iter().any(|b| b.name == new && b.var != *v) {
                        return conflict("a local name with that name is in scope where this one is used".into());
                    }
                    if !reference.definition && self.visible_top_level(loc.module).iter().any(|n| n == new) {
                        let covered = self.local_names_at(loc.module, loc.span.start).iter().all(|b| b.var != *v);
                        if covered {
                            return conflict("a top-level name with that name would be used instead".into());
                        }
                    }
                }
                // A use of another name with the new name, where the
                // renamed name is in scope: it would be covered.
                self.check_capture(r.owner, *v, new)
            }
            Target::Def(d) => {
                let mut modules = vec![d.module];
                for &m in &self.own_modules() {
                    if self.ast(m).imports.iter().any(|i| i.names.iter().any(|n| n.name == p.def(*d).name))
                        && refs.iter().any(|x| x.location.module == m)
                    {
                        modules.push(m);
                    }
                }
                for m in modules {
                    if self.visible_top_level(m).iter().any(|n| n == new) {
                        return conflict(format!("`{}` already has a top-level name `{new}`", p.module(m).path));
                    }
                }
                for reference in refs {
                    let loc = reference.location;
                    let qualified = self.qualified_at(loc.module, loc.span);
                    if !qualified && self.local_names_at(loc.module, loc.span.start).iter().any(|b| b.name == new) {
                        return conflict("a local name with that name is in scope where this one is used".into());
                    }
                }
                Ok(())
            }
            Target::Ctor(c) => {
                let module = p.ty(c.ty).module;
                if self.visible_top_level(module).iter().any(|n| n == new) {
                    return conflict(format!("`{}` already has a name `{new}`", p.module(module).path));
                }
                for reference in refs {
                    let m = reference.location.module;
                    if m != module && !self.qualified_at(m, reference.location.span)
                        && self.visible_top_level(m).iter().any(|n| n == new)
                    {
                        return conflict(format!("`{}` already has a name `{new}`", p.module(m).path));
                    }
                }
                Ok(())
            }
            Target::Type(TypeRef::Custom(t)) => {
                let module = p.ty(*t).module;
                if self.visible_types(module).iter().any(|n| n == new) {
                    return conflict(format!("`{}` already has a type `{new}`", p.module(module).path));
                }
                if BuiltinType::ALL.iter().any(|b| b.name() == new) {
                    return conflict("it is a built-in type".into());
                }
                for reference in refs {
                    let m = reference.location.module;
                    if m != module && !self.qualified_at(m, reference.location.span)
                        && self.visible_types(m).iter().any(|n| n == new)
                    {
                        return conflict(format!("`{}` already has a type `{new}`", p.module(m).path));
                    }
                }
                Ok(())
            }
            Target::Field(fields) => {
                for (c, i) in fields {
                    let Some(def) = self.ctor_def(*c) else { continue };
                    if def
                        .fields
                        .iter()
                        .enumerate()
                        .any(|(j, f)| j != *i as usize && f.label.as_ref().is_some_and(|l| l.name == new))
                    {
                        return conflict(format!("the constructor `{}` already has a field `{new}`", def.name));
                    }
                }
                Ok(())
            }
            Target::TyVar(v) => {
                let w = self.walk(r.owner);
                let item = self.item_around(r.owner, r.span);
                let clash = w.occ.iter().any(|(s, t)| {
                    matches!(t, Target::TyVar(x) if x != v && self.ty_var_name(*x) == new)
                        && item.is_some_and(|i| i.start <= s.start && s.end <= i.end)
                });
                if clash {
                    return conflict("another type variable has that name here".into());
                }
                Ok(())
            }
            Target::Type(TypeRef::Builtin(_)) | Target::Module(_) | Target::Literal(_) => Ok(()),
        }
    }

    /// The span of the top-level item around a span of a module.
    pub(crate) fn item_around(&self, m: ModuleId, span: Span) -> Option<Span> {
        self.ast(m)
            .items
            .iter()
            .find(|i| i.span.start <= span.start && span.end <= i.span.end)
            .map(|i| i.span)
    }

    /// The name at `span` is written after the name of a module.
    fn qualified_at(&self, m: ModuleId, span: Span) -> bool {
        self.walk(m)
            .occ
            .iter()
            .any(|(s, t)| matches!(t, Target::Module(_)) && s.end < span.start && span.start - s.end <= 2)
    }

    /// The uses of other names spelled `new` where the local `v` is in
    /// scope: renaming `v` to `new` would capture them.
    fn check_capture(&self, m: ModuleId, v: VarId, new: &str) -> Result<(), RenameError> {
        let w = self.walk(m);
        let text = self.text(m);
        for (span, target) in &w.occ {
            let word = text.get(span.start as usize..span.end as usize).unwrap_or_default();
            if word != new || matches!(target, Target::Module(_) | Target::Literal(_)) {
                continue;
            }
            if matches!(target, Target::Local(x) if *x == v) {
                continue;
            }
            if matches!(target, Target::Type(_) | Target::Ctor(_) | Target::TyVar(_) | Target::Field(_)) {
                continue;
            }
            if w.binders.values().any(|b| b == span) {
                continue;
            }
            if self.local_names_at(m, span.start).iter().any(|b| b.var == v) {
                return Err(RenameError(format!(
                    "cannot rename to `{new}`: a use of another `{new}` is in the scope of this name and would be captured"
                )));
            }
        }
        Ok(())
    }
}

/// `UpName` (§1.3).
pub fn is_up_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('A'..='Z')) && chars.all(|c| c.is_ascii_alphanumeric())
}
