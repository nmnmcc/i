//! Scope-aware name lookup for syntax introduced by macro expansion.
//!
//! Local names require exactly the same module and mark set. Marked top-level
//! definitions are considered only in the caller; otherwise lookup falls back
//! to the definition module, and made names observe its export boundary.
use ilang_syntax::lisp::syntax::{Node, Provenance, Scope, ScopeIdentity, Syntax};

/// A local or expansion-created binding, in lexical order (outermost first).
pub struct Binding<T> {
    pub text: String,
    pub scope: Scope,
    pub identity: Option<ScopeIdentity>,
    pub target: T,
}

/// A top-level or imported name visible in the lookup module.
pub struct ModuleName<T> {
    pub target: T,
    pub exported: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Lookup<T> {
    Found(T),
    NotExported,
    Missing,
}

/// Resolve one identifier using meta §5 rules 1–3. Labels never use this
/// function: they compare by written text instead.
pub fn lookup<T: Clone>(
    name: &Syntax,
    caller_module: u32,
    locals: &[Binding<T>],
    generated: &[Binding<T>],
    module_name: impl FnOnce(u32, &str) -> Option<ModuleName<T>>,
    special: impl FnOnce(&ScopeIdentity) -> Option<T>,
    builtin: impl FnOnce(&str) -> Option<T>,
) -> Lookup<T> {
    let text = match name.node() {
        Node::Name(text) | Node::Up(text) => text.as_str(),
        _ => panic!("hygienic lookup requires a name atom"),
    };
    let scope = name.scope();
    if let Some(identity) = name.scope_identity() {
        return match identity {
            ScopeIdentity::DefinitionParameter { .. } => locals
                .iter()
                .rev()
                .find(|b| b.identity.as_ref() == Some(identity))
                .map_or(Lookup::Missing, |b| Lookup::Found(b.target.clone())),
            ScopeIdentity::Builtin { .. } | ScopeIdentity::TypeVariable { .. } => {
                special(identity).map_or(Lookup::Missing, Lookup::Found)
            }
        };
    }
    if let Some(local) = locals
        .iter()
        .rev()
        .find(|b| b.identity.is_none() && b.text == text && b.scope == *scope)
    {
        return Lookup::Found(local.target.clone());
    }
    let made_from_answer = matches!(name.provenance(), Provenance::Made { from_answer: true });
    if !scope.marks.is_empty()
        && !made_from_answer
        && let Some(binding) = generated
            .iter()
            .rev()
            .find(|b| b.identity.is_none() && b.text == text && b.scope == *scope)
    {
        return Lookup::Found(binding.target.clone());
    }
    if let Some(found) = module_name(scope.module, text) {
        let made = matches!(name.provenance(), Provenance::Made { .. });
        if made && (!scope.marks.is_empty() || scope.module != caller_module) && !found.exported {
            return Lookup::NotExported;
        }
        return Lookup::Found(found.target);
    }
    builtin(text).map_or(Lookup::Missing, Lookup::Found)
}
