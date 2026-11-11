//! Manifest representation of target provider plans (§2.6.1).
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
fn table<'a>(v: &'a Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Option<&'a DeTable<'a>> {
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
fn list(v: &Spanned<DeValue>, d: &mut Vec<Diagnostic>, what: &str) -> Vec<String> {
    let DeValue::Array(a) = v.get_ref() else {
        error(d, at(v.span()), format!("{what} must be a list"));
        return vec![];
    };
    a.iter().filter_map(|x| qualified(x, d, what)).collect()
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
    let mut args = vec![];
    let mut required = false;
    let mut has_args = false;
    let mut has_required = false;
    for (k, v) in t.iter() {
        match k.get_ref().as_ref() {
            "factory" => factory = qualified(v, d, "factory"),
            "host" => host = boolean(v, d, "host"),
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
                required = boolean(v, d, "seed_required").unwrap_or(false);
            }
            _ => error(d, at(k.span()), "unknown provider cell key"),
        }
    }
    if has_required && !args.contains(&Argument::Seed) {
        error(d, at(v.span()), "seed_required requires a seed argument");
    }
    if host.is_some() && (factory.is_some() || has_args || has_required) {
        error(
            d,
            at(v.span()),
            "host and factory variants cannot be combined",
        );
        return None;
    }
    if let Some(h) = host {
        if !h {
            error(d, at(v.span()), "host must be true");
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
    error(d, at(v.span()), "provider cell requires factory or host");
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
