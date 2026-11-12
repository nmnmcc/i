//! The converter from the current tree to the tree of syntax three
//! (docs/design/syntax-3.md §7.3, the purely syntactic rows, and the services
//! amendment: `^ E ! N` becomes `<- (..N, Fail(E))`, environment parameters
//! become the service group of the head, `! T = e` becomes a provided block,
//! and binding-head type parameters move to the function value).
//!
//! Forms that need the resolver are converted as far as the text allows and
//! reported in a [`Note`], so the conversion never fails.

use super::ast as n;
use crate::ast as o;
use crate::lexer::Lexed;
use crate::span::Span;
use crate::token::{StringPart, Token, TokenKind as T};
use crate::walk::{Visitor, Walk};
use std::collections::{HashMap, HashSet};

/// The labels of the fields of a constructor, or of the parameters of a
/// function, in order.
pub type Labels = Vec<Option<String>>;

/// What the files that were read define: the facts of other modules that the
/// converter needs and the text of one file does not show.
#[derive(Clone, Debug, Default)]
pub struct Signatures {
    /// For each constructor name, the distinct lists of field labels with
    /// which it is defined.
    pub ctors: HashMap<String, Vec<Labels>>,
    /// The same, for each module (the last part of its path) and constructor.
    pub module_ctors: HashMap<(String, String), Vec<Labels>>,
    /// For each module and top-level function, the labels of its parameters.
    pub module_fns: HashMap<(String, String), Vec<Labels>>,
}

/// Something the converter could not do from the text alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub span: Span,
    /// A short stable name of the form, for counting.
    pub kind: &'static str,
    pub message: String,
}

fn add(list: &mut Vec<Labels>, labels: Labels) {
    if !list.contains(&labels) {
        list.push(labels);
    }
}

/// The label of a parameter (spec §6.11): the last name in the `=` chain at
/// the top of its pattern.
fn param_label(p: &o::Pattern) -> Option<String> {
    match &p.kind {
        o::PatternKind::Var(n) => Some(n.clone()),
        o::PatternKind::Typed { binder, .. } if !binder.discard => Some(binder.name.clone()),
        o::PatternKind::Named { name, pattern } => {
            param_label(pattern).or_else(|| Some(name.name.clone()))
        }
        _ => None,
    }
}

/// Names with their labels.
type NamedLabels = Vec<(String, Labels)>;

/// The constructors and the top-level functions of a module.
fn definitions(module: &o::Module) -> (NamedLabels, NamedLabels) {
    let (mut ctors, mut fns) = (Vec::new(), Vec::new());
    for item in &module.items {
        match &item.kind {
            o::ItemKind::TypeDef(o::TypeDef {
                body: o::TypeBody::Custom(cs),
                ..
            }) => {
                for c in cs {
                    let labels = c
                        .fields
                        .iter()
                        .map(|f| f.label.as_ref().map(|l| l.name.clone()))
                        .collect();
                    ctors.push((c.name.name.clone(), labels));
                }
            }
            o::ItemKind::Binding(b) => {
                let name = match &b.pattern.kind {
                    o::PatternKind::Var(n) => n,
                    o::PatternKind::Typed { binder, .. } => &binder.name,
                    _ => continue,
                };
                let mut value = &b.value;
                while let o::ExprKind::Block(block) = &value.kind
                    && let [o::Stmt::Expr(inner)] = block.stmts.as_slice()
                {
                    value = inner;
                }
                if let o::ExprKind::Fn(f) = &value.kind {
                    let labels = f
                        .params
                        .iter()
                        .filter_map(|p| match p {
                            o::Param::Pattern(p) => Some(param_label(p)),
                            o::Param::Env { .. } => None,
                        })
                        .collect();
                    fns.push((name.clone(), labels));
                }
            }
            o::ItemKind::Declaration { name, ty, .. } => {
                if let o::TypeKind::Fn { params, .. } = &ty.kind {
                    let labels = params
                        .iter()
                        .map(|p| p.label.as_ref().map(|l| l.name.clone()))
                        .collect();
                    fns.push((name.name.clone(), labels));
                }
            }
            _ => {}
        }
    }
    (ctors, fns)
}

/// Adds what `module` defines to `table`. `stem` is the last part of the
/// module's path: the name of its file without the extension.
pub fn collect(module: &o::Module, stem: &str, table: &mut Signatures) {
    let (ctors, fns) = definitions(module);
    for (name, labels) in ctors {
        add(table.ctors.entry(name.clone()).or_default(), labels.clone());
        add(
            table
                .module_ctors
                .entry((stem.to_string(), name))
                .or_default(),
            labels,
        );
    }
    for (name, labels) in fns {
        add(
            table
                .module_fns
                .entry((stem.to_string(), name))
                .or_default(),
            labels,
        );
    }
}

/// Converts a module. `lexed` is the lexing of its normal-surface source,
/// used to find interpolated strings; `None` for a Lisp-surface file.
pub fn convert(
    module: &o::Module,
    lexed: Option<&Lexed>,
    signatures: &Signatures,
) -> (n::Module, Vec<Note>) {
    let (ctors, fns) = definitions(module);
    let mut local: HashMap<String, Vec<Labels>> = HashMap::new();
    for (name, labels) in ctors {
        add(local.entry(name).or_default(), labels);
    }
    let mut local_fns: HashMap<String, Vec<Labels>> = HashMap::new();
    for (name, labels) in fns {
        add(local_fns.entry(name).or_default(), labels);
    }
    // The module that each imported name and each module name stands for.
    let mut modules = HashMap::new();
    let mut imported = HashMap::new();
    for i in &module.imports {
        let Some(stem) = i.path.last().map(|p| p.name.clone()) else {
            continue;
        };
        if let Some(alias) = &i.alias {
            modules.insert(alias.name.clone(), stem.clone());
        } else if i.names.is_empty() {
            modules.insert(stem.clone(), stem.clone());
        }
        for n in &i.names {
            imported.insert(n.name.clone(), stem.clone());
        }
    }
    let mut interps = HashMap::new();
    if let Some(lexed) = lexed {
        collect_interps(&lexed.tokens, &mut interps);
    }
    let mut cx = Cx {
        local,
        local_fns,
        modules,
        imported,
        global: signatures,
        interps,
        fresh: 0,
        notes: Vec::new(),
        vars: Vec::new(),
        absorbed: Vec::new(),
    };
    let mut module = cx.module(module);
    regroup(&mut module);
    (module, cx.notes)
}

struct Interp {
    /// The number of operands of the `<>` chain that the parser makes.
    parts: usize,
    texts: HashSet<(u32, u32)>,
}

fn collect_interps(tokens: &[Token], out: &mut HashMap<(u32, u32), Interp>) {
    for token in tokens {
        if let T::Interpolated(parts) = &token.kind {
            let mut count = 0;
            let mut texts = HashSet::new();
            for part in parts {
                match part {
                    StringPart::Text(s, span) if !s.is_empty() => {
                        count += 1;
                        texts.insert((span.start, span.end));
                    }
                    StringPart::Text(..) => {}
                    StringPart::Expr { tokens, .. } => {
                        count += 1;
                        collect_interps(tokens, out);
                    }
                }
            }
            if count == 1 {
                count = 2;
                texts.insert((token.span.start, token.span.start));
            }
            out.insert(
                (token.span.start, token.span.end),
                Interp {
                    parts: count,
                    texts,
                },
            );
        }
    }
}

enum Lookup {
    /// Every known definition has only fields without labels, or the
    /// constructor is not known.
    Positional,
    Fields(Vec<Option<String>>),
    /// Several definitions with different labels fit.
    Ambiguous,
}

struct Cx<'a> {
    /// The constructors and the top-level functions of this module.
    local: HashMap<String, Vec<Labels>>,
    local_fns: HashMap<String, Vec<Labels>>,
    /// The module (the last part of its path) of each module name and of
    /// each imported name of this file.
    modules: HashMap<String, String>,
    imported: HashMap<String, String>,
    global: &'a Signatures,
    /// The number of fresh locals `_syn3_N` made so far.
    fresh: usize,
    interps: HashMap<(u32, u32), Interp>,
    notes: Vec<Note>,
    /// The type variables in scope, for telling a row variable from a
    /// service type.
    vars: Vec<String>,
    /// The type parameters that are used only in `^` parts beside a row
    /// variable, and so are dropped.
    absorbed: Vec<String>,
}

fn id(i: &o::Ident) -> n::Ident {
    n::Ident::new(i.name.clone(), i.span)
}

fn opt_id(i: &Option<o::Ident>) -> Option<n::Ident> {
    i.as_ref().map(id)
}

fn binder(b: &o::Binder) -> n::Binder {
    n::Binder {
        name: b.name.clone(),
        discard: b.discard,
        span: n::Sp(b.span),
    }
}

fn is_name(s: &str) -> bool {
    let rest = s.strip_prefix('_').unwrap_or(s);
    rest.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn named(name: &str, args: Vec<n::Type>, span: Span) -> n::Type {
    n::Type::new(
        n::TypeKind::Named {
            module: None,
            name: n::Ident::new(name, span),
            args,
        },
        span,
    )
}

impl Cx<'_> {
    fn note(&mut self, span: Span, kind: &'static str, message: impl Into<String>) {
        self.notes.push(Note {
            span,
            kind,
            message: message.into(),
        });
    }

    fn label(&mut self, l: &o::Ident) -> n::Ident {
        if is_name(&l.name) {
            return id(l);
        }
        self.note(
            l.span,
            "string-label",
            format!(
                "the label `{}` is not a name; syntax three has no string labels",
                l.name
            ),
        );
        let mut s: String = l
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        if !s.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
            s.insert(0, 'l');
        }
        n::Ident::new(s, l.span)
    }

    /// The definitions of the constructor `[module.]name` that this file
    /// can mean: its own, those of the module it imports the name from, or
    /// every one that was read.
    fn ctor_candidates(&self, module: Option<&o::Ident>, name: &str) -> Option<&Vec<Labels>> {
        let stem = match module {
            Some(m) => self.modules.get(&m.name),
            None => {
                if let Some(own) = self.local.get(name) {
                    return Some(own);
                }
                self.imported.get(name)
            }
        };
        stem.and_then(|stem| {
            self.global
                .module_ctors
                .get(&(stem.clone(), name.to_string()))
        })
        .or_else(|| self.global.ctors.get(name))
    }

    fn lookup(
        &self,
        module: Option<&o::Ident>,
        name: &str,
        fits: impl Fn(&[Option<String>]) -> bool,
    ) -> Lookup {
        let Some(candidates) = self.ctor_candidates(module, name) else {
            return Lookup::Positional;
        };
        if candidates.iter().all(|c| c.iter().all(Option::is_none)) {
            return Lookup::Positional;
        }
        let fitting: Vec<_> = candidates.iter().filter(|c| fits(c)).collect();
        match (candidates.len(), fitting.as_slice()) {
            (1, _) => Lookup::Fields(candidates[0].clone()),
            (_, [one]) => Lookup::Fields((*one).clone()),
            _ => Lookup::Ambiguous,
        }
    }

    /// The labels of the parameters of the function that `callee` names,
    /// when exactly one definition that was read takes `count` arguments
    /// and has every label of `labels`.
    fn fn_labels(&self, callee: &o::Expr, count: usize, labels: &[&str]) -> Option<Labels> {
        let candidates = match &callee.kind {
            o::ExprKind::Name { module: None, name } => match self.local_fns.get(&name.name) {
                Some(own) => Some(own),
                None => self.imported.get(&name.name).and_then(|stem| {
                    self.global
                        .module_fns
                        .get(&(stem.clone(), name.name.clone()))
                }),
            },
            o::ExprKind::Field { target, name } => match &target.kind {
                o::ExprKind::Name {
                    module: None,
                    name: m,
                } => self.modules.get(&m.name).and_then(|stem| {
                    self.global
                        .module_fns
                        .get(&(stem.clone(), name.name.clone()))
                }),
                _ => None,
            },
            _ => None,
        }?;
        let fitting: Vec<&Labels> = candidates
            .iter()
            .filter(|c| {
                c.len() == count
                    && labels
                        .iter()
                        .all(|l| c.iter().any(|f| f.as_deref() == Some(*l)))
            })
            .collect();
        match fitting.as_slice() {
            [one] => Some((*one).clone()),
            _ => None,
        }
    }

    /// §7.3: a call of a function with arguments by label becomes
    /// positional, in parameter order. `piped` is set when the call is the
    /// right side of `|>`, which gives it its first argument. Returns `None`
    /// when the function is not known from the text.
    fn positional_call(
        &mut self,
        callee: &o::Expr,
        args: &[o::Arg],
        piped: bool,
        span: Span,
    ) -> Option<n::ExprKind> {
        let offset = usize::from(piped);
        let given: Vec<&str> = args
            .iter()
            .filter_map(|a| a.label.as_ref().map(|l| l.name.as_str()))
            .collect();
        let params = self.fn_labels(callee, args.len() + offset, &given)?;
        // The index of the argument that goes to each parameter.
        let mut slots: Vec<Option<usize>> = vec![None; params.len() - offset];
        let mut next = 0;
        for (i, a) in args.iter().enumerate() {
            let slot = match &a.label {
                None => {
                    let slot = next;
                    next += 1;
                    slot
                }
                Some(l) => params
                    .iter()
                    .position(|p| p.as_deref() == Some(l.name.as_str()))?
                    .checked_sub(offset)?,
            };
            if slots.get(slot)?.is_some() {
                return None;
            }
            slots[slot] = Some(i);
        }
        let order: Vec<usize> = slots.into_iter().collect::<Option<_>>()?;
        let moved = order.windows(2).any(|w| w[0] > w[1]);
        let indirect: Vec<usize> = (0..args.len())
            .filter(|&i| !is_direct(&args[i].value))
            .collect();
        let callee = Box::new(self.expr(callee));
        if !moved || indirect.len() < 2 {
            let items = order
                .iter()
                .map(|&i| n::DItem::Pos(self.expr(&args[i].value)))
                .collect();
            return Some(n::ExprKind::Call {
                callee,
                args: items,
            });
        }
        if piped {
            // The piped value is worked out before the arguments; locals
            // before the pipe would change that.
            return None;
        }
        // The order changes and two or more arguments are not direct
        // values: each of those is bound, in written order, to a fresh
        // local in a group block, so the order of evaluation stays.
        let mut stmts = Vec::new();
        let mut names: HashMap<usize, String> = HashMap::new();
        for &i in &indirect {
            let name = format!("_syn3_{}", self.fresh);
            self.fresh += 1;
            let at = args[i].value.span;
            stmts.push(n::Stmt::Bind(n::Binding {
                pattern: n::Pattern::new(n::PatternKind::Var(name.clone()), at),
                value: self.expr(&args[i].value),
                span: n::Sp(Span::default()),
            }));
            names.insert(i, name);
        }
        let items = order
            .iter()
            .map(|&i| {
                n::DItem::Pos(match names.get(&i) {
                    Some(name) => n::Expr::new(
                        n::ExprKind::Name {
                            module: None,
                            name: n::Ident::new(name.clone(), args[i].value.span),
                        },
                        args[i].value.span,
                    ),
                    None => self.expr(&args[i].value),
                })
            })
            .collect();
        stmts.push(n::Stmt::Expr(n::Expr::new(
            n::ExprKind::Call {
                callee,
                args: items,
            },
            span,
        )));
        Some(n::ExprKind::Block(n::Block {
            stmts,
            span: n::Sp(Span::default()),
        }))
    }

    // ---- Module and items ----

    fn module(&mut self, m: &o::Module) -> n::Module {
        // The constructors of each type of this module that are not the type
        // itself, for exports.
        let mut members: HashMap<&str, Vec<&o::Ident>> = HashMap::new();
        for item in &m.items {
            if let o::ItemKind::TypeDef(def) = &item.kind
                && let o::TypeBody::Custom(ctors) = &def.body
            {
                let others: Vec<&o::Ident> = ctors
                    .iter()
                    .map(|c| &c.name)
                    .filter(|c| c.name != def.name.name)
                    .collect();
                if !others.is_empty() {
                    members.insert(&def.name.name, others);
                }
            }
        }
        let mut exports: Vec<n::Export> = Vec::new();
        let exported: HashSet<&str> = m.exports.iter().map(|e| e.name.name.as_str()).collect();
        for e in &m.exports {
            exports.push(n::Export {
                name: id(&e.name),
                constructors: e.constructors,
                span: n::Sp(e.span),
            });
            if !e.constructors
                && let Some(others) = members.get(e.name.name.as_str())
            {
                // §7.3: `T` alone becomes `T, C1, C2` (types only).
                for c in others {
                    if !exported.contains(c.name.as_str()) {
                        exports.push(n::Export {
                            name: n::Ident::new(c.name.clone(), e.span),
                            constructors: false,
                            span: n::Sp(e.span),
                        });
                    }
                }
            }
        }
        let mut items = Vec::new();
        for item in &m.items {
            self.item(item, &mut items);
        }
        n::Module {
            doc: m.doc.clone(),
            imports: m
                .imports
                .iter()
                .map(|i| n::Import {
                    alias: opt_id(&i.alias),
                    names: i.names.iter().map(id).collect(),
                    path: i.path.iter().map(id).collect(),
                    span: n::Sp(i.span),
                })
                .collect(),
            exports,
            items,
        }
    }

    fn attribute(&mut self, a: &o::Attribute) -> n::Attribute {
        let args = if let Some((args, _)) = &a.call {
            Some(args.iter().map(|a| self.arg(a)).collect())
        } else if a.args.is_empty() {
            None
        } else {
            Some(
                a.args
                    .iter()
                    .map(|(s, span)| {
                        n::DItem::Pos(n::Expr::new(n::ExprKind::String(s.clone()), *span))
                    })
                    .collect(),
            )
        };
        n::Attribute {
            module: opt_id(&a.module),
            name: id(&a.name),
            args,
            span: n::Sp(a.span),
        }
    }

    fn arg(&mut self, a: &o::Arg) -> n::DItem {
        let value = self.expr(&a.value);
        match &a.label {
            Some(l) => n::DItem::Label(self.label(l), value),
            None => n::DItem::Pos(value),
        }
    }

    fn item(&mut self, item: &o::Item, out: &mut Vec<n::Item>) {
        let attributes: Vec<n::Attribute> =
            item.attributes.iter().map(|a| self.attribute(a)).collect();
        let make = |kind, doc: Vec<String>, attributes| n::Item {
            doc,
            attributes,
            kind,
            span: n::Sp(item.span),
        };
        match &item.kind {
            o::ItemKind::TypeDef(def) => {
                let saved = self.vars.len();
                self.vars.extend(def.params.iter().map(|p| p.name.clone()));
                let defs = self.type_def(def, item.span);
                self.vars.truncate(saved);
                let last = defs.len() - 1;
                let mut attributes = Some(attributes);
                // The members come first, with an empty span, so the comments
                // before the definition stay before the sum. When the
                // definition is the first item and has no documentation, the
                // comments before it are the file's header: the first member
                // takes the span, so they stay first. The documentation and
                // attributes stay with the sum.
                let header = out.is_empty() && item.doc.is_empty();
                for (i, def) in defs.into_iter().enumerate() {
                    if i == last {
                        let mut sum = make(
                            n::ItemKind::TypeDef(def),
                            item.doc.clone(),
                            attributes.take().unwrap(),
                        );
                        if last > 0 && header {
                            sum.span = n::Sp(Span::default());
                        }
                        out.push(sum);
                    } else {
                        out.push(n::Item {
                            doc: Vec::new(),
                            attributes: Vec::new(),
                            kind: n::ItemKind::TypeDef(def),
                            span: n::Sp(if i == 0 && header {
                                item.span
                            } else {
                                Span::default()
                            }),
                        });
                    }
                }
            }
            o::ItemKind::Binding(b) => {
                let b = self.binding(b);
                out.push(make(n::ItemKind::Bind(b), item.doc.clone(), attributes));
            }
            o::ItemKind::Declaration {
                name,
                type_params,
                ty,
            } => {
                let saved = self.vars.len();
                self.vars.extend(type_params.iter().map(|p| p.name.clone()));
                let mut ty = self.ty(ty);
                self.vars.truncate(saved);
                if !type_params.is_empty() && !self.put_generics(&mut ty, type_params) {
                    self.note(
                        name.span,
                        "type-params",
                        "type parameters of a declaration whose type is not a function type are dropped",
                    );
                }
                out.push(make(
                    n::ItemKind::Decl { name: id(name), ty },
                    item.doc.clone(),
                    attributes,
                ));
            }
            o::ItemKind::MacroItem { module, name, args } => {
                let args = args.iter().map(|a| self.arg(a)).collect();
                out.push(make(
                    n::ItemKind::Macro {
                        module: opt_id(module),
                        name: id(name),
                        args,
                    },
                    item.doc.clone(),
                    attributes,
                ));
            }
        }
    }

    fn put_generics(&mut self, ty: &mut n::Type, params: &[o::Ident]) -> bool {
        match &mut ty.kind {
            n::TypeKind::Fn(f) => {
                f.generics = params.iter().map(id).collect();
                true
            }
            _ => false,
        }
    }

    /// §7.3 rows 1 to 5. The last definition keeps the name of the type.
    fn type_def(&mut self, def: &o::TypeDef, span: Span) -> Vec<n::TypeDef> {
        let params: Vec<n::Type> = def
            .params
            .iter()
            .map(|p| named(&p.name, Vec::new(), p.span))
            .collect();
        let head: Vec<n::Type> = if def.head.is_empty() {
            params.clone()
        } else {
            def.head.iter().map(|t| self.ty(t)).collect()
        };
        let ctors = match &def.body {
            o::TypeBody::Alias(ty) => {
                return vec![n::TypeDef {
                    name: id(&def.name),
                    head,
                    rhs: n::TypeRhs::Type(self.ty(ty)),
                }];
            }
            o::TypeBody::Custom(ctors) => ctors,
        };
        if ctors.is_empty() {
            return vec![n::TypeDef {
                name: id(&def.name),
                head,
                rhs: n::TypeRhs::Opaque,
            }];
        }
        let data = |cx: &mut Self, c: &o::Constructor| -> Vec<n::TItem> {
            c.fields
                .iter()
                .map(|f| {
                    let ty = cx.ty(&f.ty);
                    match &f.label {
                        Some(l) => n::TItem::Label(cx.label(l), ty),
                        None => n::TItem::Pos(ty),
                    }
                })
                .collect()
        };
        if let [only] = ctors.as_slice()
            && only.name.name == def.name.name
        {
            return vec![n::TypeDef {
                name: id(&def.name),
                head,
                rhs: n::TypeRhs::Data(data(self, only)),
            }];
        }
        let mut out = Vec::new();
        let mut members = Vec::new();
        for c in ctors {
            if c.name.name == def.name.name {
                self.note(
                    c.span,
                    "constructor-name-clash",
                    format!(
                        "the constructor `{}` has the name of its type, which has other constructors; rename one by hand",
                        c.name.name
                    ),
                );
            }
            // Each constructor keeps only the type parameters its fields use.
            let mut used = HashSet::new();
            for f in &c.fields {
                type_names(&f.ty, &mut used);
            }
            let own: Vec<n::Type> = def
                .params
                .iter()
                .filter(|p| used.contains(p.name.as_str()))
                .map(|p| named(&p.name, Vec::new(), p.span))
                .collect();
            members.push(named(&c.name.name, own.clone(), c.span));
            out.push(n::TypeDef {
                name: id(&c.name),
                head: own,
                rhs: n::TypeRhs::Data(data(self, c)),
            });
        }
        let rhs = if members.len() == 1 {
            members.pop().unwrap()
        } else {
            n::Type::new(n::TypeKind::Union(members), span)
        };
        out.push(n::TypeDef {
            name: id(&def.name),
            head,
            rhs: n::TypeRhs::Type(rhs),
        });
        out
    }

    // ---- Statements ----

    fn binding(&mut self, b: &o::Binding) -> n::Binding {
        if let Some(annotated) = annotated(b) {
            return self.binding(&annotated);
        }
        let saved = (self.vars.len(), self.absorbed.len());
        self.vars
            .extend(b.type_params.iter().map(|p| p.name.clone()));
        // §7.3: a type parameter all of whose uses go away is dropped: uses
        // in the `^` and `!` parts of the function literal itself, and whole
        // `^ E` parts of function types that have a row variable.
        let mut kept: Vec<&o::Ident> = Vec::new();
        if !b.type_params.is_empty() {
            let mut copy = o::Binding {
                pattern: b.pattern.clone(),
                value: b.value.clone(),
                span: b.span,
                type_params: Vec::new(),
            };
            let mut count = Count(HashMap::new());
            copy.walk(&mut count);
            let mut gone: HashMap<&str, usize> = HashMap::new();
            let mut carried: HashMap<&str, usize> = HashMap::new();
            signature_uses(b, &self.vars, &mut gone, &mut carried);
            // A row variable carries the errors only when every function
            // type of the signature that has it also has a `^` part: beside
            // `(R) -> Nil ! N`, the `E` of `() -> R ^ E ! N` stays written.
            if bare_row(b, &self.vars) {
                carried.clear();
            }
            for p in &b.type_params {
                let total = count.0.get(p.name.as_str()).copied().unwrap_or(0);
                let own = gone.get(p.name.as_str()).copied().unwrap_or(0);
                let by_row = carried.get(p.name.as_str()).copied().unwrap_or(0);
                if total > 0 && total == own + by_row {
                    if by_row > 0 {
                        self.absorbed.push(p.name.clone());
                    }
                } else {
                    kept.push(p);
                }
            }
        }
        let mut pattern = self.pattern(&b.pattern, false);
        let mut value = self.expr(&b.value);
        self.vars.truncate(saved.0);
        self.absorbed.truncate(saved.1);
        if !kept.is_empty() {
            let generics: Vec<n::Ident> = kept.iter().map(|p| id(p)).collect();
            let mut placed = false;
            // On the written type when there is one (`id: <A>(A) -> A = …`),
            // else on the function literal.
            if let n::PatternKind::Typed { ty, .. } = &mut pattern.kind
                && let n::TypeKind::Fn(f) = &mut ty.kind
            {
                f.generics = generics.clone();
                placed = true;
            }
            if !placed && let n::ExprKind::Chain(fs) = &mut value.kind {
                for f in fs {
                    f.generics = generics.clone();
                }
                placed = true;
            }
            if !placed {
                self.note(
                    b.span,
                    "type-params",
                    "type parameters of a binding whose value is not a function and whose type is not a function type are dropped",
                );
            }
        }
        n::Binding {
            pattern,
            value,
            span: n::Sp(b.span),
        }
    }

    fn block(&mut self, b: &o::Block) -> n::Block {
        n::Block {
            stmts: self.stmts(&b.stmts, b.span),
            span: n::Sp(b.span),
        }
    }

    /// `! T = e` followed by the rest `r` of its block becomes `{ r } <- (e)`.
    fn stmts(&mut self, stmts: &[o::Stmt], span: Span) -> Vec<n::Stmt> {
        let mut out = Vec::new();
        for (i, s) in stmts.iter().enumerate() {
            match s {
                o::Stmt::Bind(b) => out.push(n::Stmt::Bind(self.binding(b))),
                o::Stmt::Expr(e) => out.push(n::Stmt::Expr(self.expr(e))),
                o::Stmt::Env {
                    value,
                    span: env_span,
                    ..
                } => {
                    // §7.3: consecutive statements whose values are direct
                    // values are one provide; others nest, because a later
                    // value may need an earlier service.
                    let mut provided = vec![n::DItem::Pos(self.expr(value))];
                    let mut i = i;
                    while is_direct(value)
                        && let Some(o::Stmt::Env { value: next, .. }) = stmts.get(i + 1)
                        && is_direct(next)
                    {
                        provided.push(n::DItem::Pos(self.expr(next)));
                        i += 1;
                    }
                    let rest_span = Span::new(env_span.end, span.end.max(env_span.end));
                    let mut rest = self.stmts(&stmts[i + 1..], span);
                    let target = if rest.is_empty() {
                        self.note(
                            *env_span,
                            "env-statement-last",
                            "an environment statement at the end of a block provides to nothing",
                        );
                        n::Expr::new(n::ExprKind::Data(Vec::new()), rest_span)
                    } else if matches!(rest.as_slice(), [n::Stmt::Expr(_)]) {
                        let Some(n::Stmt::Expr(e)) = rest.pop() else {
                            unreachable!()
                        };
                        e
                    } else {
                        n::Expr::new(
                            n::ExprKind::Block(n::Block {
                                stmts: rest,
                                span: n::Sp(Span::default()),
                            }),
                            Span::new(rest_span.start, rest_span.start),
                        )
                    };
                    let services = n::Expr::new(n::ExprKind::Data(provided), *env_span);
                    out.push(n::Stmt::Expr(n::Expr::new(
                        n::ExprKind::Provide {
                            target: Box::new(target),
                            services: Box::new(services),
                        },
                        Span::new(env_span.start, env_span.start),
                    )));
                    return out;
                }
            }
        }
        out
    }

    // ---- Expressions ----

    fn interp(&mut self, e: &o::Expr) -> Option<n::ExprKind> {
        let info = self.interps.get(&(e.span.start, e.span.end))?;
        let count = info.parts;
        let mut operands = Vec::new();
        let mut cur = e;
        for _ in 1..count {
            let o::ExprKind::Binary {
                op: o::BinOp::Concat,
                lhs,
                rhs,
            } = &cur.kind
            else {
                return None;
            };
            operands.push(&**rhs);
            cur = lhs;
        }
        operands.push(cur);
        operands.reverse();
        let mut parts = Vec::new();
        for operand in operands {
            let text = match &operand.kind {
                o::ExprKind::String(s)
                    if self.interps[&(e.span.start, e.span.end)]
                        .texts
                        .contains(&(operand.span.start, operand.span.end)) =>
                {
                    Some(s.clone())
                }
                _ => None,
            };
            match text {
                Some(s) if s.is_empty() => {}
                Some(s) => parts.push(n::InterpPart::Text(s)),
                None => parts.push(n::InterpPart::Expr(self.expr(operand))),
            }
        }
        Some(n::ExprKind::Interp(parts))
    }

    fn function(&mut self, f: &o::Function, span: Span) -> n::Function {
        let mut params = Vec::new();
        let mut services = Vec::new();
        for p in &f.params {
            match p {
                o::Param::Pattern(p) => params.push(n::PItem {
                    key: None,
                    choices: vec![self.pattern(p, true)],
                }),
                // §7.3: the service pattern, in parameter order.
                o::Param::Env { name, ty } => services.push(n::SItem {
                    rest: false,
                    name: id(name),
                    ty: self.ty(ty),
                }),
            }
        }
        let mut ret = None;
        if let Some(r) = &f.ret {
            ret = Some(self.ty(&r.value));
            if r.err.is_some() || r.env.is_some() {
                // §7.3: the needs of a function literal are left to
                // inference; a written need that is wider than what the body
                // needs is no longer stated.
                self.note(
                    r.value.span,
                    "result-needs",
                    "the `^` and `!` parts of a function literal are dropped: its needs are inferred",
                );
            }
        }
        n::Function {
            generics: Vec::new(),
            params,
            guard: None,
            services: (!services.is_empty()).then_some(services),
            ret,
            body: self.block(&f.body),
            span: n::Sp(span),
        }
    }

    fn clause(&mut self, c: &o::Clause) -> n::Function {
        let mut choices: Vec<n::Pattern> =
            c.patterns.iter().map(|p| self.pattern(p, true)).collect();
        // G21: a type test is the last alternative.
        let typed = |p: &n::Pattern| match &p.kind {
            n::PatternKind::Typed { .. } => true,
            n::PatternKind::And(parts) => {
                matches!(
                    parts.last().map(|p| &p.kind),
                    Some(n::PatternKind::Typed { .. })
                )
            }
            _ => false,
        };
        let count = choices.iter().filter(|p| typed(p)).count();
        if count == 1 {
            let at = choices.iter().position(typed).unwrap();
            let p = choices.remove(at);
            choices.push(p);
        } else if count > 1 {
            self.note(
                c.span,
                "typed-alternatives",
                "two alternatives with a type test cannot be written in one parameter item: a type takes the `|` after it",
            );
        }
        let body = match &c.body.kind {
            o::ExprKind::Block(b) => self.block(b),
            _ => n::Block {
                stmts: vec![n::Stmt::Expr(self.expr(&c.body))],
                span: n::Sp(c.body.span),
            },
        };
        n::Function {
            generics: Vec::new(),
            params: vec![n::PItem { key: None, choices }],
            guard: c.guard.as_ref().map(|g| self.expr(g)),
            services: None,
            ret: None,
            body,
            span: n::Sp(c.span),
        }
    }

    fn core_never(&mut self, span: Span) -> n::Expr {
        self.note(
            span,
            "empty-match",
            "the empty match becomes `core.never`; the module must import `i/core`",
        );
        n::Expr::new(
            n::ExprKind::Field {
                target: Box::new(n::Expr::new(
                    n::ExprKind::Name {
                        module: None,
                        name: n::Ident::new("core", span),
                    },
                    span,
                )),
                name: n::Ident::new("never", span),
            },
            span,
        )
    }

    fn call(&mut self, callee: &o::Expr, args: &[o::Arg], piped: bool, span: Span) -> n::ExprKind {
        let ctor = match &callee.kind {
            o::ExprKind::Name { module, name }
                if name
                    .name
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase) =>
            {
                Some((module.as_ref(), name))
            }
            _ => None,
        };
        let items = match ctor {
            Some((module, name)) => self.ctor_args(module, name, args, span),
            None => {
                if let Some(a) = args.iter().find(|a| a.label.is_some()) {
                    if let Some(kind) = self.positional_call(callee, args, piped, span) {
                        return kind;
                    }
                    self.note(
                        a.value.span,
                        "labelled-call",
                        "a call with an argument by label of a function whose parameters the text does not show: the labels are kept as labelled items; the stage 4 migrator (`labels`) makes the call positional",
                    );
                }
                args.iter().map(|a| self.arg(a)).collect()
            }
        };
        n::ExprKind::Call {
            callee: Box::new(self.expr(callee)),
            args: items,
        }
    }

    /// The data of a call of a constructor.
    fn ctor_args(
        &mut self,
        module: Option<&o::Ident>,
        name: &o::Ident,
        args: &[o::Arg],
        span: Span,
    ) -> Vec<n::DItem> {
        let count = args.len();
        let labels: Vec<&str> = args
            .iter()
            .filter_map(|a| a.label.as_ref().map(|l| l.name.as_str()))
            .collect();
        let lookup = self.lookup(module, &name.name, |c| {
            c.len() >= count
                && labels
                    .iter()
                    .all(|l| c.iter().any(|f| f.as_deref() == Some(*l)))
        });
        let fields = match lookup {
            Lookup::Fields(fields) => fields,
            Lookup::Positional => Vec::new(),
            Lookup::Ambiguous => {
                if args.iter().any(|a| a.label.is_none()) {
                    self.note(
                        span,
                        "constructor-ambiguous",
                        format!(
                            "`{}` is defined with different labels in several modules; its arguments by position are kept",
                            name.name
                        ),
                    );
                }
                Vec::new()
            }
        };
        let mut out = Vec::new();
        let mut keyed = false;
        for (i, a) in args.iter().enumerate() {
            let value = self.expr(&a.value);
            match (&a.label, fields.get(i)) {
                (Some(l), _) => {
                    keyed = true;
                    out.push(n::DItem::Label(self.label(l), value));
                }
                (None, Some(Some(l))) => {
                    keyed = true;
                    out.push(n::DItem::Label(
                        n::Ident::new(l.clone(), a.value.span),
                        value,
                    ));
                }
                (None, _) => {
                    if keyed {
                        self.note(
                            a.value.span,
                            "position-after-label",
                            "a field without a label comes after a field with a label",
                        );
                    }
                    out.push(n::DItem::Pos(value));
                }
            }
        }
        out
    }

    fn expr(&mut self, e: &o::Expr) -> n::Expr {
        if matches!(
            e.kind,
            o::ExprKind::Binary {
                op: o::BinOp::Concat,
                ..
            }
        ) && let Some(kind) = self.interp(e)
        {
            return n::Expr::new(kind, e.span);
        }
        let kind = match &e.kind {
            o::ExprKind::Quote(d) => {
                n::ExprKind::Quote(n::Quoted(Box::new(super::lisp::convert_quote(d))))
            }
            o::ExprKind::Int(v) => n::ExprKind::Int(v.clone()),
            o::ExprKind::Float(v) => n::ExprKind::Float(*v),
            o::ExprKind::String(s) => n::ExprKind::String(s.clone()),
            o::ExprKind::Name { module, name } => n::ExprKind::Name {
                module: opt_id(module),
                name: id(name),
            },
            o::ExprKind::Tuple(items) => {
                n::ExprKind::Data(items.iter().map(|i| n::DItem::Pos(self.expr(i))).collect())
            }
            o::ExprKind::Record { base, fields } => {
                let mut items = Vec::new();
                if let Some(base) = base {
                    items.push(n::DItem::Spread(self.expr(base)));
                }
                for (l, v) in fields {
                    let value = self.expr(v);
                    items.push(n::DItem::Label(self.label(l), value));
                }
                n::ExprKind::Data(items)
            }
            o::ExprKind::List { items, rest } => n::ExprKind::List {
                items: items.iter().map(|i| self.expr(i)).collect(),
                rest: rest.as_ref().map(|r| Box::new(self.expr(r))),
            },
            o::ExprKind::BitArray(segments) => n::ExprKind::BitArray(
                segments
                    .iter()
                    .map(|s| n::Segment {
                        value: self.expr(&s.value),
                        format: self.format(&s.format),
                    })
                    .collect(),
            ),
            o::ExprKind::Block(b) => n::ExprKind::Block(self.block(b)),
            o::ExprKind::Fn(f) => n::ExprKind::Chain(vec![self.function(f, e.span)]),
            o::ExprKind::Case { subject, clauses } => {
                if clauses.is_empty() {
                    let callee = self.core_never(e.span);
                    n::ExprKind::Call {
                        callee: Box::new(callee),
                        args: vec![n::DItem::Pos(self.expr(subject))],
                    }
                } else {
                    let lhs = self.expr(subject);
                    let chain = clauses.iter().map(|c| self.clause(c)).collect();
                    let from = clauses[0].span.start;
                    n::ExprKind::Pipe {
                        lhs: Box::new(lhs),
                        rhs: Box::new(n::Expr::new(
                            n::ExprKind::Chain(chain),
                            Span::new(from, e.span.end),
                        )),
                    }
                }
            }
            o::ExprKind::Clauses(clauses) => {
                if clauses.is_empty() {
                    return self.core_never(e.span);
                }
                n::ExprKind::Chain(clauses.iter().map(|c| self.clause(c)).collect())
            }
            o::ExprKind::Call { callee, args } => self.call(callee, args, false, e.span),
            o::ExprKind::Field { target, name } => n::ExprKind::Field {
                target: Box::new(self.expr(target)),
                name: id(name),
            },
            o::ExprKind::TupleIndex { target, index, .. } => n::ExprKind::Index {
                target: Box::new(self.expr(target)),
                index: index.clone(),
            },
            o::ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                let mut items = vec![n::DItem::Spread(self.expr(base))];
                for (l, v) in fields {
                    let value = self.expr(v);
                    items.push(n::DItem::Label(self.label(l), value));
                }
                n::ExprKind::Call {
                    callee: Box::new(self.expr(constructor)),
                    args: items,
                }
            }
            o::ExprKind::Unary { op, expr } => n::ExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(expr)),
            },
            o::ExprKind::Binary {
                op: o::BinOp::Pipe,
                lhs,
                rhs,
            } => {
                let lhs = Box::new(self.expr(lhs));
                // `a |> f(b)` is `f(a, b)`: the labels of `f` count from
                // its second parameter.
                let rhs = match &rhs.kind {
                    o::ExprKind::Call { callee, args } => {
                        n::Expr::new(self.call(callee, args, true, rhs.span), rhs.span)
                    }
                    _ => self.expr(rhs),
                };
                n::ExprKind::Pipe {
                    lhs,
                    rhs: Box::new(rhs),
                }
            }
            o::ExprKind::Binary { op, lhs, rhs } => n::ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
        };
        let out = n::Expr::new(kind, e.span);
        if let n::ExprKind::Pipe { rhs, .. } = &out.kind
            && !super::pipe_target(rhs)
        {
            self.note(
                e.span,
                "pipe-target",
                "the right side of `|>` is not a name, a call or a chain",
            );
        }
        out
    }

    fn format(&mut self, f: &Option<o::Format>) -> Option<n::Format> {
        f.as_ref().map(|f| n::Format {
            name: id(&f.name),
            arg: f.arg.as_ref().map(|a| Box::new(self.expr(a))),
        })
    }

    // ---- Patterns ----

    /// `item` is set directly inside a data pattern, a constructor pattern
    /// or a parameter group, where a leading `name =` is a key (P7): the
    /// alias names go after the pattern.
    fn pattern(&mut self, p: &o::Pattern, item: bool) -> n::Pattern {
        let kind = match &p.kind {
            o::PatternKind::Quote { datum, .. } => {
                n::PatternKind::Quote(n::Quoted(Box::new(super::lisp::convert_quote(datum))))
            }
            o::PatternKind::Var(s) => n::PatternKind::Var(s.clone()),
            o::PatternKind::Discard(s) => n::PatternKind::Discard(s.clone()),
            o::PatternKind::Typed { binder: b, ty } => n::PatternKind::Typed {
                binder: binder(b),
                ty: self.ty(ty),
            },
            o::PatternKind::Named { .. } => {
                let mut names = Vec::new();
                let mut cur = p;
                while let o::PatternKind::Named { name, pattern } = &cur.kind {
                    names.push(n::Pattern::new(
                        n::PatternKind::Var(name.name.clone()),
                        name.span,
                    ));
                    cur = pattern;
                }
                let inner = self.pattern(cur, false);
                let mut parts = Vec::new();
                if !item {
                    parts.extend(names);
                    push_parts(&mut parts, inner);
                } else if matches!(inner.kind, n::PatternKind::Var(_)) {
                    // `(x = label)` becomes `(_ = x = label)`.
                    parts.push(n::Pattern::new(
                        n::PatternKind::Discard("_".into()),
                        Span::new(p.span.start, p.span.start),
                    ));
                    parts.extend(names);
                    parts.push(inner);
                } else {
                    push_parts(&mut parts, inner);
                    parts.extend(names);
                }
                n::PatternKind::And(parts)
            }
            o::PatternKind::Int { negative, value } => n::PatternKind::Int {
                negative: *negative,
                value: value.clone(),
            },
            o::PatternKind::Float { negative, value } => n::PatternKind::Float {
                negative: *negative,
                value: *value,
            },
            o::PatternKind::String(s) => n::PatternKind::String(s.clone()),
            o::PatternKind::Prefix {
                prefix,
                suffix,
                rest,
            } => n::PatternKind::Prefix {
                prefix: prefix.clone(),
                suffix: suffix.clone(),
                rest: binder(rest),
            },
            o::PatternKind::Constructor {
                module,
                name,
                args,
                spread,
            } => {
                let items = self.ctor_pattern(module.as_ref(), name, args, *spread, p.span);
                n::PatternKind::Ctor {
                    module: opt_id(module),
                    name: id(name),
                    args: items,
                }
            }
            o::PatternKind::Tuple(items) => n::PatternKind::Data(
                items
                    .iter()
                    .map(|i| n::DPItem::Pos(self.pattern(i, true)))
                    .collect(),
            ),
            o::PatternKind::List { items, rest } => n::PatternKind::List {
                items: items.iter().map(|i| self.pattern(i, false)).collect(),
                rest: rest.as_ref().map(|r| r.as_ref().map(binder)),
            },
            o::PatternKind::BitArray(segments) => n::PatternKind::BitArray(
                segments
                    .iter()
                    .map(|s| n::Segment {
                        value: self.pattern(&s.value, false),
                        format: self.format(&s.format),
                    })
                    .collect(),
            ),
        };
        n::Pattern::new(kind, p.span)
    }

    /// Spec §7.1: fields without labels are matched by position, the others
    /// by the last name in the chain of `=` of each part.
    fn ctor_pattern(
        &mut self,
        module: Option<&o::Ident>,
        name: &o::Ident,
        args: &[o::Pattern],
        spread: bool,
        span: Span,
    ) -> Option<Vec<n::DPItem>> {
        if args.is_empty() && !spread {
            return None;
        }
        fn label_of(p: &o::Pattern) -> Option<&str> {
            match &p.kind {
                o::PatternKind::Var(s) => Some(s),
                o::PatternKind::Typed { binder, .. } if !binder.discard => Some(&binder.name),
                o::PatternKind::Named { pattern, .. } => label_of(pattern),
                _ => None,
            }
        }
        let count = args.len();
        let lookup = self.lookup(module, &name.name, |c| {
            let free = c.iter().filter(|f| f.is_none()).count();
            (if spread {
                c.len() >= count
            } else {
                c.len() == count
            }) && args
                .iter()
                .skip(free)
                .all(|a| label_of(a).is_some_and(|l| c.iter().any(|f| f.as_deref() == Some(l))))
        });
        let fields = match lookup {
            Lookup::Fields(fields) => fields,
            Lookup::Positional => Vec::new(),
            Lookup::Ambiguous => {
                self.note(
                    span,
                    "constructor-ambiguous",
                    format!(
                        "`{}` is defined with different labels in several modules; its pattern parts are kept by position",
                        name.name
                    ),
                );
                Vec::new()
            }
        };
        let free = if fields.is_empty() {
            count
        } else {
            fields.iter().filter(|f| f.is_none()).count()
        };
        let mut items = Vec::new();
        for (i, a) in args.iter().enumerate() {
            if i < free {
                items.push(n::DPItem::Pos(self.pattern(a, true)));
                continue;
            }
            let Some(label) =
                label_of(a).filter(|l| fields.iter().any(|f| f.as_deref() == Some(*l)))
            else {
                self.note(
                    a.span,
                    "constructor-pattern-label",
                    format!(
                        "this part of the pattern of `{}` names no label of it",
                        name.name
                    ),
                );
                items.push(n::DPItem::Pos(self.pattern(a, true)));
                continue;
            };
            let key = n::Ident::new(label, a.span);
            // The names before the label in the chain bind the part; with
            // none, the label's own name does (no punning).
            let mut names = Vec::new();
            let mut cur = a;
            while let o::PatternKind::Named { name, pattern } = &cur.kind {
                names.push(name);
                cur = pattern;
            }
            let ty = match &cur.kind {
                o::PatternKind::Typed { ty, .. } => Some(self.ty(ty)),
                _ => None,
            };
            let mut binders: Vec<(String, Span)> =
                names.iter().map(|n| (n.name.clone(), n.span)).collect();
            if binders.is_empty() {
                binders.push((label.to_string(), cur.span));
            }
            let mut parts: Vec<n::Pattern> = Vec::new();
            for (j, (text, at)) in binders.into_iter().enumerate() {
                parts.push(match (&ty, j) {
                    (Some(ty), 0) => n::Pattern::new(
                        n::PatternKind::Typed {
                            binder: n::Binder {
                                name: text,
                                discard: false,
                                span: n::Sp(at),
                            },
                            ty: ty.clone(),
                        },
                        at,
                    ),
                    _ => n::Pattern::new(n::PatternKind::Var(text), at),
                });
            }
            let pattern = if parts.len() == 1 {
                parts.pop().unwrap()
            } else {
                // A typed part is put last, where its type ends at `,` or `)`.
                if matches!(parts[0].kind, n::PatternKind::Typed { .. }) {
                    let first = parts.remove(0);
                    parts.push(first);
                }
                n::Pattern::new(n::PatternKind::And(parts), a.span)
            };
            items.push(n::DPItem::Key(key, pattern));
        }
        if spread {
            items.push(n::DPItem::Rest(None));
        }
        Some(items)
    }

    // ---- Types ----

    /// The needs for `^ err ! env` (§7.3): `Fail(err)`, the members of
    /// `env` one by one, and `..N` for a row variable.
    fn row(&mut self, err: Option<&o::TypeExpr>, env: Option<&o::TypeExpr>) -> Vec<n::TItem> {
        let mut items = Vec::new();
        let mut spreads = Vec::new();
        if let Some(env) = env {
            let members: Vec<&o::TypeExpr> = match &env.kind {
                o::TypeKind::Union(ms) => ms.iter().collect(),
                _ => vec![env],
            };
            for m in members {
                if is_var(&self.vars, m) {
                    spreads.push(n::TItem::Spread(self.ty(m)));
                } else if let o::TypeKind::Diff(parts) = &m.kind
                    && is_var(&self.vars, &parts[0])
                {
                    self.note(
                        m.span,
                        "row-difference",
                        "a difference on a row variable is dropped: in `(..N, C(…))`, `N` has no entry for `C`",
                    );
                    spreads.push(n::TItem::Spread(self.ty(&parts[0])));
                } else if matches!(&m.kind, o::TypeKind::Named { name, args, module: None }
                    if name.name == "Never" && args.is_empty())
                {
                    // `! Never` disappears.
                } else if matches!(m.kind, o::TypeKind::Named { .. }) {
                    items.push(n::TItem::Pos(self.ty(m)));
                } else {
                    self.note(
                        m.span,
                        "need-not-named",
                        "a member of an environment that is not a named type cannot be written as a need; it is dropped",
                    );
                }
            }
        }
        if spreads.len() > 1 {
            self.note(
                env.map_or(Span::default(), |e| e.span),
                "two-row-variables",
                "a need has at most one row variable; the others are dropped",
            );
            spreads.truncate(1);
        }
        let mut out = Vec::new();
        if let Some(err) = err {
            let never = matches!(&err.kind, o::TypeKind::Named { module: None, name, args }
                if name.name == "Never" && args.is_empty());
            // A type parameter used only in `^` parts is carried by the row
            // variable beside it.
            let absorbed = !spreads.is_empty()
                && matches!(&err.kind, o::TypeKind::Named { module: None, name, args }
                    if args.is_empty() && self.absorbed.contains(&name.name));
            if !never && !absorbed {
                let inner = self.ty(err);
                out.push(n::TItem::Pos(named("Fail", vec![inner], err.span)));
            }
        }
        out.extend(items);
        out.extend(spreads);
        out
    }

    fn ty(&mut self, t: &o::TypeExpr) -> n::Type {
        let kind = match &t.kind {
            o::TypeKind::Lit(v) => n::TypeKind::Lit(v.clone()),
            o::TypeKind::Named { module, name, args } => n::TypeKind::Named {
                module: opt_id(module),
                name: id(name),
                args: args.iter().map(|a| self.ty(a)).collect(),
            },
            o::TypeKind::ValueOf { module, name } => n::TypeKind::ValueOf {
                module: opt_id(module),
                name: id(name),
            },
            o::TypeKind::Call { module, name, args } => n::TypeKind::Call {
                module: opt_id(module),
                name: id(name),
                args: args.iter().map(|a| self.ty(a)).collect(),
            },
            o::TypeKind::Hole => n::TypeKind::Hole,
            o::TypeKind::Tuple(items) => {
                n::TypeKind::Data(items.iter().map(|i| n::TItem::Pos(self.ty(i))).collect())
            }
            o::TypeKind::Record {
                base,
                fields,
                removed,
            } => {
                if base.is_some() || !removed.is_empty() {
                    self.note(
                        t.span,
                        "open-record",
                        "a record type with a spread or a removed label is an open record; data is closed in syntax three",
                    );
                }
                let mut items = Vec::new();
                for (l, ty) in fields {
                    if l.parameter {
                        self.note(l.span, "open-record", "a label parameter is removed");
                    }
                    let ty = self.ty(ty);
                    items.push(n::TItem::Label(self.label(&l.ident), ty));
                }
                n::TypeKind::Data(items)
            }
            o::TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                // Function types lose their documentation labels.
                let items = params
                    .iter()
                    .map(|p| n::TItem::Pos(self.ty(&p.ty)))
                    .collect();
                let mut row = self.row(err.as_deref(), env.as_deref());
                let services = if row.is_empty() {
                    None
                } else if matches!(row.as_slice(), [n::TItem::Spread(_)]) {
                    let Some(n::TItem::Spread(ty)) = row.pop() else {
                        unreachable!()
                    };
                    Some(ty)
                } else {
                    Some(n::Type::new(n::TypeKind::Data(row), t.span))
                };
                n::TypeKind::Fn(Box::new(n::FnType {
                    generics: Vec::new(),
                    param: n::Type::new(n::TypeKind::Data(items), t.span),
                    services,
                    ret: self.ty(ret),
                }))
            }
            o::TypeKind::Union(parts) => {
                n::TypeKind::Union(parts.iter().map(|p| self.ty(p)).collect())
            }
            o::TypeKind::Inter(parts) => {
                n::TypeKind::Inter(parts.iter().map(|p| self.ty(p)).collect())
            }
            o::TypeKind::Diff(parts) => {
                n::TypeKind::Diff(parts.iter().map(|p| self.ty(p)).collect())
            }
        };
        n::Type::new(kind, t.span)
    }
}

/// Puts a group `{ e }` where the grammar of syntax three reads an operand
/// of a stronger level than `e` has: today's `|>` is stronger than `<>` and
/// the comparisons, and is the weakest operator now (§7.3, stage 5's row).
/// A group is a block in the tree, so the converter writes it.
pub fn regroup(module: &mut n::Module) {
    for item in &mut module.items {
        for a in &mut item.attributes {
            if let Some(args) = &mut a.args {
                group_items(args);
            }
        }
        match &mut item.kind {
            n::ItemKind::Bind(b) => {
                group_pattern(&mut b.pattern);
                group_expr(&mut b.value);
            }
            n::ItemKind::Macro { args, .. } => group_items(args),
            n::ItemKind::TypeDef(_) | n::ItemKind::Decl { .. } => {}
        }
    }
}

fn group_items(items: &mut [n::DItem]) {
    for item in items {
        match item {
            n::DItem::Pos(e) | n::DItem::Label(_, e) | n::DItem::Spread(e) => group_expr(e),
        }
    }
}

fn group_block(b: &mut n::Block) {
    for s in &mut b.stmts {
        match s {
            n::Stmt::Bind(b) => {
                group_pattern(&mut b.pattern);
                group_expr(&mut b.value);
            }
            n::Stmt::Expr(e) => group_expr(e),
        }
    }
}

fn group_format(f: &mut Option<n::Format>) {
    if let Some(arg) = f.as_mut().and_then(|f| f.arg.as_mut()) {
        group_expr(arg);
    }
}

fn group_pattern(p: &mut n::Pattern) {
    match &mut p.kind {
        n::PatternKind::Ctor {
            args: Some(items), ..
        }
        | n::PatternKind::Data(items) => {
            for i in items {
                match i {
                    n::DPItem::Pos(p) | n::DPItem::Key(_, p) => group_pattern(p),
                    n::DPItem::Rest(_) => {}
                }
            }
        }
        n::PatternKind::List { items, .. } | n::PatternKind::And(items) => {
            items.iter_mut().for_each(group_pattern)
        }
        n::PatternKind::BitArray(segments) => {
            for s in segments {
                group_pattern(&mut s.value);
                group_format(&mut s.format);
            }
        }
        _ => {}
    }
}

/// Makes `e` an operand of at least the level `min`.
fn at_least(e: &mut n::Expr, min: i8) {
    group_expr(e);
    let literal = min == super::print::POSTFIX + 1
        && matches!(e.kind, n::ExprKind::Int(_) | n::ExprKind::Float(_));
    if super::print::level(e) < min.min(super::print::POSTFIX) || literal {
        let span = e.span;
        let inner = std::mem::replace(e, n::Expr::new(n::ExprKind::Data(Vec::new()), span.0));
        *e = n::Expr {
            kind: n::ExprKind::Block(n::Block {
                stmts: vec![n::Stmt::Expr(inner)],
                span,
            }),
            span,
        };
    }
}

fn group_expr(e: &mut n::Expr) {
    use super::print::{LOWEST, PIPE, POSTFIX};
    match &mut e.kind {
        n::ExprKind::Quote(_)
        | n::ExprKind::Int(_)
        | n::ExprKind::Float(_)
        | n::ExprKind::String(_)
        | n::ExprKind::Name { .. } => {}
        n::ExprKind::Interp(parts) => {
            for p in parts {
                if let n::InterpPart::Expr(e) = p {
                    group_expr(e);
                }
            }
        }
        n::ExprKind::Data(items) => group_items(items),
        n::ExprKind::List { items, rest } => {
            items.iter_mut().for_each(group_expr);
            if let Some(r) = rest {
                group_expr(r);
            }
        }
        n::ExprKind::BitArray(segments) => {
            for s in segments {
                group_expr(&mut s.value);
                group_format(&mut s.format);
            }
        }
        n::ExprKind::Block(b) => group_block(b),
        n::ExprKind::Chain(fs) => {
            for f in fs {
                for p in &mut f.params {
                    p.choices.iter_mut().for_each(group_pattern);
                }
                if let Some(g) = &mut f.guard {
                    group_expr(g);
                }
                group_block(&mut f.body);
            }
        }
        n::ExprKind::Call { callee, args } => {
            // The target of a call, `.f` or `.k` is a postfix expression,
            // and not a number.
            at_least(callee, POSTFIX + 1);
            group_items(args);
        }
        n::ExprKind::Field { target, .. } | n::ExprKind::Index { target, .. } => {
            at_least(target, POSTFIX + 1)
        }
        n::ExprKind::Provide { target, services } => {
            at_least(target, LOWEST);
            at_least(services, POSTFIX);
        }
        n::ExprKind::Unary { expr, .. } => at_least(expr, POSTFIX),
        n::ExprKind::Binary { op, lhs, rhs } => {
            let lvl = op.level() as i8;
            at_least(lhs, lvl);
            at_least(rhs, lvl + 1);
        }
        n::ExprKind::Pipe { lhs, rhs } => {
            at_least(lhs, PIPE);
            at_least(rhs, PIPE + 1);
        }
    }
}

/// A direct value (spec §6.10), as far as the text shows: a literal, a
/// name, a function, or data and constructor calls of direct values.
fn is_direct(e: &o::Expr) -> bool {
    use o::ExprKind as E;
    match &e.kind {
        E::Fn(_) | E::Clauses(_) | E::Int(_) | E::Float(_) | E::String(_) | E::Name { .. } => true,
        E::Block(b) => matches!(b.stmts.as_slice(), [o::Stmt::Expr(e)] if is_direct(e)),
        E::Tuple(items) => items.iter().all(is_direct),
        E::Record { base, fields } => {
            base.as_deref().is_none_or(is_direct) && fields.iter().all(|(_, v)| is_direct(v))
        }
        E::List { items, rest } => items.iter().chain(rest.as_deref()).all(is_direct),
        E::Call { callee, args } => {
            matches!(&callee.kind, E::Name { name, .. }
                if name.name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                && args.iter().all(|a| is_direct(&a.value))
        }
        _ => false,
    }
}

/// Counts every string of a tree; an `UpName` is counted where it is used.
struct Count(HashMap<String, usize>);

impl Visitor for Count {
    fn string(&mut self, s: &mut String) {
        *self.0.entry(s.clone()).or_default() += 1;
    }
}

/// The uses of names in the written signature of a binding that go away:
/// `gone` counts the names in the `^` and `!` parts of the function literal
/// itself, `carried` the whole `^ E` parts of function types with a row
/// variable.
fn signature_uses<'a>(
    b: &'a o::Binding,
    vars: &[String],
    gone: &mut HashMap<&'a str, usize>,
    carried: &mut HashMap<&'a str, usize>,
) {
    fn names<'a>(t: &'a o::TypeExpr, out: &mut HashMap<&'a str, usize>) {
        let mut set = Vec::new();
        all_type_names(t, &mut set);
        for n in set {
            *out.entry(n).or_default() += 1;
        }
    }
    fn fn_types<'a>(t: &'a o::TypeExpr, vars: &[String], carried: &mut HashMap<&'a str, usize>) {
        if let o::TypeKind::Fn { err, env, .. } = &t.kind
            && let Some(err) = err
            && let o::TypeKind::Named {
                module: None,
                name,
                args,
            } = &err.kind
            && args.is_empty()
            && env.as_ref().is_some_and(|env| match &env.kind {
                o::TypeKind::Union(ms) => ms.iter().any(|m| is_var(vars, m)),
                _ => is_var(vars, env),
            })
        {
            *carried.entry(name.name.as_str()).or_default() += 1;
        }
        for_each_type_child(t, &mut |c| fn_types(c, vars, carried));
    }
    fn pattern_types<'a>(p: &'a o::Pattern, f: &mut dyn FnMut(&'a o::TypeExpr)) {
        match &p.kind {
            o::PatternKind::Typed { ty, .. } => f(ty),
            o::PatternKind::Named { pattern, .. } => pattern_types(pattern, f),
            o::PatternKind::Constructor { args, .. } | o::PatternKind::Tuple(args) => {
                for a in args {
                    pattern_types(a, f);
                }
            }
            o::PatternKind::List { items, .. } => {
                for a in items {
                    pattern_types(a, f);
                }
            }
            _ => {}
        }
    }
    pattern_types(&b.pattern, &mut |t| fn_types(t, vars, carried));
    if let o::ExprKind::Fn(f) = &b.value.kind {
        for p in &f.params {
            match p {
                o::Param::Pattern(p) => pattern_types(p, &mut |t| fn_types(t, vars, carried)),
                o::Param::Env { ty, .. } => fn_types(ty, vars, carried),
            }
        }
        if let Some(r) = &f.ret {
            fn_types(&r.value, vars, carried);
            if let Some(t) = &r.err {
                names(t, gone);
            }
            if let Some(t) = &r.env {
                names(t, gone);
            }
        }
    }
}

/// Whether the written signature of a binding has a function type with a
/// row variable and no `^` part.
fn bare_row(b: &o::Binding, vars: &[String]) -> bool {
    fn ty(t: &o::TypeExpr, vars: &[String], found: &mut bool) {
        if let o::TypeKind::Fn {
            err: None,
            env: Some(env),
            ..
        } = &t.kind
            && match &env.kind {
                o::TypeKind::Union(ms) => ms.iter().any(|m| is_var(vars, m)),
                _ => is_var(vars, env),
            }
        {
            *found = true;
        }
        for_each_type_child(t, &mut |c| ty(c, vars, found));
    }
    fn pattern(p: &o::Pattern, vars: &[String], found: &mut bool) {
        match &p.kind {
            o::PatternKind::Typed { ty: t, .. } => ty(t, vars, found),
            o::PatternKind::Named { pattern: inner, .. } => pattern(inner, vars, found),
            o::PatternKind::Constructor { args, .. } | o::PatternKind::Tuple(args) => {
                for a in args {
                    pattern(a, vars, found);
                }
            }
            o::PatternKind::List { items, .. } => {
                for a in items {
                    pattern(a, vars, found);
                }
            }
            _ => {}
        }
    }
    let mut found = false;
    pattern(&b.pattern, vars, &mut found);
    if let o::ExprKind::Fn(f) = &b.value.kind {
        for p in &f.params {
            match p {
                o::Param::Pattern(p) => pattern(p, vars, &mut found),
                o::Param::Env { ty: t, .. } => ty(t, vars, &mut found),
            }
        }
        if let Some(r) = &f.ret {
            ty(&r.value, vars, &mut found);
        }
    }
    found
}

/// Every unqualified `UpName` in a type, once for each place it is written.
fn all_type_names<'a>(t: &'a o::TypeExpr, out: &mut Vec<&'a str>) {
    if let o::TypeKind::Named {
        module: None, name, ..
    } = &t.kind
    {
        out.push(&name.name);
    }
    for_each_type_child(t, &mut |c| all_type_names(c, out));
}

fn for_each_type_child<'a>(t: &'a o::TypeExpr, f: &mut dyn FnMut(&'a o::TypeExpr)) {
    match &t.kind {
        o::TypeKind::Named { args, .. }
        | o::TypeKind::Call { args, .. }
        | o::TypeKind::Tuple(args)
        | o::TypeKind::Union(args)
        | o::TypeKind::Inter(args)
        | o::TypeKind::Diff(args) => args.iter().for_each(f),
        o::TypeKind::Record { base, fields, .. } => {
            if let Some(b) = base {
                f(b);
            }
            for (_, t) in fields {
                f(t);
            }
        }
        o::TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            for p in params {
                f(&p.ty);
            }
            f(ret);
            if let Some(t) = err {
                f(t);
            }
            if let Some(t) = env {
                f(t);
            }
        }
        o::TypeKind::Lit(_) | o::TypeKind::ValueOf { .. } | o::TypeKind::Hole => {}
    }
}

/// Whether `t` is a declared type variable.
fn is_var(vars: &[String], t: &o::TypeExpr) -> bool {
    match &t.kind {
        o::TypeKind::Named {
            module: None,
            name,
            args,
        } => args.is_empty() && vars.contains(&name.name),
        _ => false,
    }
}

fn push_parts(parts: &mut Vec<n::Pattern>, p: n::Pattern) {
    match p.kind {
        n::PatternKind::And(inner) => parts.extend(inner),
        _ => parts.push(p),
    }
}

/// The unqualified `UpName`s in a type.
fn type_names<'a>(t: &'a o::TypeExpr, out: &mut HashSet<&'a str>) {
    let mut all = Vec::new();
    all_type_names(t, &mut all);
    out.extend(all);
}

/// §7.3: "a written `! N` that is wider than what the body needs is kept as a
/// type annotation on the binding". Without the checker the converter cannot
/// compare, so it keeps the annotation where the need is certainly not the
/// body's own: the written needs of the function have a row variable that a
/// parameter's function type needs beside another service, which the
/// function provides to that parameter (a handler or a generator). `f(A, N) = (g: () -> R ! X
/// | N) -> T ! N { … }` becomes `f: <A, N>(() <- (X, ..N) -> R) <- N -> T =
/// (g) { … }`.
fn annotated(b: &o::Binding) -> Option<o::Binding> {
    let o::PatternKind::Var(name) = &b.pattern.kind else {
        return None;
    };
    let o::ExprKind::Fn(f) = &b.value.kind else {
        return None;
    };
    let ret = f.ret.as_ref()?;
    let env = ret.env.as_ref()?;
    let mentions = |t: &o::TypeExpr, v: &str| format!("{t:?}").contains(&format!("name: {v:?}"));
    let row = b.type_params.iter().find(|p| mentions(env, &p.name))?;
    let mut types = Vec::new();
    let mut params = Vec::new();
    let mut provides = false;
    for p in &f.params {
        let o::Param::Pattern(o::Pattern {
            kind: o::PatternKind::Typed { binder, ty },
            span,
        }) = p
        else {
            return None;
        };
        // The parameter needs a service besides the row: the function
        // provides it (`body: () -> Nil ! Yield(A) | N`).
        if let o::TypeKind::Fn {
            env: Some(inner), ..
        } = &ty.kind
            && let o::TypeKind::Union(members) = &inner.kind
            && members.len() >= 2
            && mentions(inner, &row.name)
        {
            provides = true;
        }
        types.push(o::FnParamType {
            label: None,
            ty: ty.clone(),
        });
        let kind = if binder.discard {
            o::PatternKind::Discard(binder.name.clone())
        } else {
            o::PatternKind::Var(binder.name.clone())
        };
        params.push(o::Param::Pattern(o::Pattern { kind, span: *span }));
    }
    if !provides {
        return None;
    }
    let ty = o::TypeExpr {
        kind: o::TypeKind::Fn {
            params: types,
            ret: Box::new(ret.value.clone()),
            err: ret.err.clone().map(Box::new),
            env: Some(Box::new(env.clone())),
        },
        span: b.value.span,
    };
    Some(o::Binding {
        pattern: o::Pattern {
            kind: o::PatternKind::Typed {
                binder: o::Binder {
                    name: name.clone(),
                    discard: false,
                    span: b.pattern.span,
                },
                ty,
            },
            span: b.pattern.span,
        },
        value: o::Expr {
            kind: o::ExprKind::Fn(Box::new(o::Function {
                params,
                ret: None,
                body: f.body.clone(),
            })),
            span: b.value.span,
        },
        span: b.span,
        type_params: b.type_params.clone(),
    })
}
