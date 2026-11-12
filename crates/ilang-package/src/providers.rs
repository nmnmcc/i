//! The representation of target provider plans in the package file.
use crate::{Diagnostic, code, is_name};
use ilang_syntax::span::Span;
use std::collections::BTreeMap;
use toml::{
    Spanned,
    de::{DeTable, DeValue},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderPlan {
    pub rows: Vec<ProviderRow>,
    pub adapters: Vec<ProviderAdapter>,
    pub span: Option<Span>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    #[default]
    None,
    Scheduler,
    Runner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Js,
    Nodejs,
    Native,
    Wasi,
    Host,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Argument {
    Seed,
    EntryRootKey,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Factory {
        factory: String,
        arguments: Vec<Argument>,
        seed_required: bool,
    },
    Host,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderRow {
    pub service: String,
    pub role: Role,
    /// Distinguishes an omitted role from an explicit metadata redeclaration.
    pub role_declared: bool,
    pub adapter: Option<String>,
    pub discriminant: BTreeMap<Target, String>,
    pub cells: BTreeMap<Target, Cell>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderAdapter {
    pub function: String,
    pub supplies: Vec<String>,
    pub requires: Vec<String>,
    pub span: Span,
}
fn at(r: std::ops::Range<usize>) -> Span {
    Span::new(r.start as u32, r.end as u32)
}
fn error(d: &mut Vec<Diagnostic>, s: Span, msg: impl Into<String>) {
    d.push(Diagnostic::error(code::MANIFEST_SHAPE, None, s, msg));
}
fn target(s: &str) -> Option<Target> {
    match s {
        "js" => Some(Target::Js),
        "nodejs" => Some(Target::Nodejs),
        "native" => Some(Target::Native),
        "wasi" => Some(Target::Wasi),
        "host" => Some(Target::Host),
        _ => None,
    }
}
fn type_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('A'..='Z')) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn path(s: &str) -> bool {
    let Some((m, n)) = s.rsplit_once('.') else {
        return false;
    };
    let parts: Vec<_> = m.split('/').collect();
    parts.len() >= 2 && parts.iter().all(|p| is_name(p)) && (is_name(n) || type_name(n))
}
fn string(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Option<String> {
    if let DeValue::String(s) = v.get_ref() {
        Some(s.to_string())
    } else {
        error(d, at(v.span()), format!("{what} must be a string"));
        None
    }
}
fn qualified(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Option<String> {
    let s = string(v, d, what)?;
    if !path(&s) {
        error(d, at(v.span()), format!("{what} must be a qualified path"));
        None
    } else {
        Some(s)
    }
}
fn table<'a>(
    v: &'a Spanned<DeValue>,
    d: &mut Vec<Diagnostic>,
    what: &str,
) -> Option<&'a DeTable<'a>> {
    if let DeValue::Table(t) = v.get_ref() {
        Some(t)
    } else {
        error(d, at(v.span()), format!("{what} must be a table"));
        None
    }
}
fn boolean(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Option<bool> {
    if let DeValue::Boolean(b) = v.get_ref() {
        Some(*b)
    } else {
        error(d, at(v.span()), format!("{what} must be a boolean"));
        None
    }
}
// Adapter service types may apply closed type constructors to closed types.
fn closed_type(s: &str) -> bool {
    let s = s.trim();
    if let Some(open) = s.find('(') {
        if !s.ends_with(')') || !(path(&s[..open]) || type_name(&s[..open])) {
            return false;
        }
        let inner = &s[open + 1..s.len() - 1];
        let mut depth = 0usize;
        let mut start = 0;
        for (i, c) in inner.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                }
                ',' if depth == 0 => {
                    if !closed_type(&inner[start..i]) {
                        return false;
                    }
                    start = i + 1;
                }
                _ => {}
            }
        }
        depth == 0 && closed_type(&inner[start..])
    } else {
        path(s) || matches!(s, "Int" | "String" | "Bool" | "Nil" | "Float" | "Bytes")
    }
}
fn list(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Vec<String> {
    let DeValue::Array(a) = v.get_ref() else {
        error(d, at(v.span()), format!("{what} must be a list"));
        return vec![];
    };
    a.iter()
        .filter_map(|item| {
            let s = string(item, d, what)?;
            if closed_type(&s) {
                Some(s)
            } else {
                error(
                    d,
                    at(item.span()),
                    format!("{what} must contain closed service types"),
                );
                None
            }
        })
        .collect()
}

fn cell(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>) -> Option<Cell> {
    if matches!(v.get_ref(), DeValue::String(_)) {
        return qualified(v, d, "factory").map(|factory| Cell::Factory {
            factory,
            arguments: vec![],
            seed_required: false,
        });
    }
    let t = table(v, d, "provider cell")?;
    let mut factory = None;
    let mut host = None;
    let mut has_factory = false;
    let mut has_host = false;
    let mut host_span = at(v.span());
    let mut args = vec![];
    let mut required = false;
    let mut has_args = false;
    let mut has_required = false;
    let mut required_span = at(v.span());
    for (k, v) in t.iter() {
        match k.get_ref().as_ref() {
            "factory" => {
                has_factory = true;
                factory = qualified(v, d, "factory");
            }
            "host" => {
                has_host = true;
                host_span = at(v.span());
                host = boolean(v, d, "host");
            }
            "arguments" => {
                has_args = true;
                if let DeValue::Array(a) = v.get_ref() {
                    for item in a.iter() {
                        let a = match item.get_ref() {
                            DeValue::String(s) if s.as_ref() == "seed" => Some(Argument::Seed),
                            DeValue::String(s) if s.as_ref() == "entry-root-key" => {
                                Some(Argument::EntryRootKey)
                            }
                            _ => None,
                        };
                        match a {
                            Some(a) if !args.contains(&a) => args.push(a),
                            Some(_) => error(d, at(item.span()), "duplicate provider argument"),
                            None => error(d, at(item.span()), "unknown provider argument"),
                        }
                    }
                } else {
                    error(d, at(v.span()), "arguments must be a list");
                }
            }
            "seed_required" => {
                has_required = true;
                required_span = at(v.span());
                required = boolean(v, d, "seed_required").unwrap_or(false);
            }
            _ => error(d, at(k.span()), "unknown provider cell key"),
        }
    }
    if has_required && !args.contains(&Argument::Seed) {
        error(d, required_span, "seed_required requires a seed argument");
    }
    if has_host && (has_factory || has_args || has_required) {
        error(d, host_span, "host and factory variants cannot be combined");
        return None;
    }
    if let Some(h) = host {
        if !h {
            error(d, host_span, "host must be true");
            return None;
        }
        return Some(Cell::Host);
    }
    if let Some(factory) = factory {
        return Some(Cell::Factory {
            factory,
            arguments: args,
            seed_required: required,
        });
    }
    if !has_factory && !has_host {
        error(d, at(v.span()), "provider cell requires factory or host");
    }
    None
}
pub(crate) fn rows(v: &Spanned<DeValue>, plan: &mut ProviderPlan, d: &mut Vec<Diagnostic>) {
    let Some(t) = table(v, d, "providers") else {
        return;
    };
    for (k, v) in t.iter() {
        let service = k.get_ref().to_string();
        if !path(&service) {
            error(d, at(k.span()), "service must be a qualified path");
        }
        let Some(row) = table(v, d, "provider row") else {
            continue;
        };
        let mut r = ProviderRow {
            service,
            role: Role::None,
            role_declared: false,
            adapter: None,
            discriminant: BTreeMap::new(),
            cells: BTreeMap::new(),
            span: at(k.span()),
        };
        let mut role_span = at(k.span());
        let mut adapter_span = at(k.span());
        for (key, val) in row.iter() {
            let name: &str = key.get_ref().as_ref();
            match name {
                "role" => {
                    r.role_declared = true;
                    role_span = at(val.span());
                    if let Some(s) = string(val, d, "role") {
                        r.role = match s.as_str() {
                            "none" => Role::None,
                            "scheduler" => Role::Scheduler,
                            "runner" => Role::Runner,
                            _ => {
                                error(d, at(val.span()), "unknown provider role");
                                Role::None
                            }
                        };
                    }
                }
                "adapter" => {
                    adapter_span = at(val.span());
                    r.adapter = qualified(val, d, "adapter");
                }
                "discriminant" => {
                    if let Some(map) = table(val, d, "discriminant") {
                        for (k, v) in map.iter() {
                            if let Some(t) = target(k.get_ref().as_ref()) {
                                if let Some(s) = qualified(v, d, "discriminant") {
                                    r.discriminant.insert(t, s);
                                }
                            } else {
                                error(d, at(k.span()), "unknown discriminant target");
                            }
                        }
                        if r.discriminant.len() != 5 {
                            error(
                                d,
                                at(val.span()),
                                "discriminant must cover all five targets",
                            );
                        }
                    }
                }
                _ => {
                    if let Some(t) = target(name) {
                        if let Some(c) = cell(val, d) {
                            r.cells.insert(t, c);
                        }
                    } else {
                        error(d, at(key.span()), "unknown provider target or key");
                    }
                }
            }
        }
        if r.role == Role::None && r.adapter.is_some() {
            error(d, adapter_span, "role none must not have an adapter");
        }
        if r.role != Role::None && r.adapter.is_none() {
            error(d, role_span, "provider role requires an adapter");
        }
        plan.rows.push(r);
    }
}
pub(crate) fn adapters(v: &Spanned<DeValue>, plan: &mut ProviderPlan, d: &mut Vec<Diagnostic>) {
    let Some(t) = table(v, d, "provider_adapters") else {
        return;
    };
    for (k, v) in t.iter() {
        let function = k.get_ref().to_string();
        if !path(&function) {
            error(d, at(k.span()), "adapter must be a qualified path");
        }
        let Some(t) = table(v, d, "adapter row") else {
            continue;
        };
        let mut a = ProviderAdapter {
            function,
            supplies: vec![],
            requires: vec![],
            span: at(k.span()),
        };
        for (k, v) in t.iter() {
            match k.get_ref().as_ref() {
                "supplies" => a.supplies = list(v, d, "supplies"),
                "requires" => a.requires = list(v, d, "requires"),
                _ => error(d, at(k.span()), "unknown adapter key"),
            }
        }
        plan.adapters.push(a);
    }
}

/// A selected cell retains the authority of the manifest that declared it.
#[derive(Clone, Debug)]
pub struct SelectedCell<'a> {
    pub cell: &'a Cell,
    pub package: crate::PackageId,
}
/// A row retains its original metadata and the authority for each target cell.
#[derive(Clone, Debug)]
pub struct SelectedRow<'a> {
    pub row: &'a ProviderRow,
    pub package: crate::PackageId,
    pub cells: BTreeMap<Target, SelectedCell<'a>>,
}
impl crate::PackageGraph {
    /// Select explicit root cells over inherited cells. Resolution of names to
    /// nominal identities and access checking belongs to the provider checker.
    pub fn selected_provider_rows(&self) -> Vec<SelectedRow<'_>> {
        let mut rows: Vec<SelectedRow<'_>> = Vec::new();
        let packages = (0..self.packages.len())
            .filter(|id| Some(*id) != self.root)
            .chain(self.root);
        for id in packages {
            for row in &self.packages[id].provider_plan.rows {
                if let Some(existing) = rows.iter_mut().find(|r| r.row.service == row.service) {
                    for (&target, cell) in &row.cells {
                        if Some(id) == self.root {
                            existing
                                .cells
                                .insert(target, SelectedCell { cell, package: id });
                        } else {
                            existing
                                .cells
                                .entry(target)
                                .or_insert(SelectedCell { cell, package: id });
                        }
                    }
                } else {
                    rows.push(SelectedRow {
                        row,
                        package: id,
                        cells: row
                            .cells
                            .iter()
                            .map(|(&target, cell)| (target, SelectedCell { cell, package: id }))
                            .collect(),
                    });
                }
            }
        }
        rows
    }
}
