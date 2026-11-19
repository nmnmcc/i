//! Repair advice derived from accessible modules and their exports. These
//! queries never take part in name resolution or type checking.

use super::*;

/// The prelude is not a module that an import can name; its names are in
/// every module. Gives the message for an import of `i/prelude`.
pub(super) fn prelude_import_message(written: &str) -> Option<String> {
    (written == "i/prelude").then(|| {
        format!("there is no module `{written}`: the prelude is built in; its names need no import")
    })
}

/// Shared by ordinary resolution and the macro import-closure diagnostic.
/// Only consult loaded modules; do not resolve or load any additional imports.
pub fn missing_module_message(
    graph: &PackageGraph,
    from: PackageId,
    kind: ModuleKind,
    written: &str,
) -> String {
    if let Some(message) = prelude_import_message(written) {
        return message;
    }
    let mut message = format!("there is no module `{written}`");
    let other = if let Some(rest) = written.strip_prefix("i/") {
        format!("std/{rest}")
    } else if let Some(rest) = written.strip_prefix("std/") {
        format!("i/{rest}")
    } else {
        return message;
    };
    let package = &graph.packages[from];
    let mut alternatives = BTreeSet::new();
    for (target, candidate) in graph.packages.iter().enumerate() {
        for module in &candidate.modules {
            if module.path != other || module.kind == ModuleKind::Test && kind != ModuleKind::Test {
                continue;
            }
            let (_, rest) = module.path.split_once('/').unwrap();
            if target == graph.library {
                alternatives.insert(format!("i/{rest}"));
            } else if target == from {
                alternatives.insert(format!("{}/{rest}", package.name));
            } else {
                for (name, id) in package.dependencies.iter().chain(
                    package
                        .dev_dependencies
                        .iter()
                        .filter(|_| kind == ModuleKind::Test),
                ) {
                    if *id == Some(target) {
                        alternatives.insert(format!("{name}/{rest}"));
                    }
                }
            }
        }
    }
    if !alternatives.is_empty() {
        message.push('.');
    }
    for path in alternatives.into_iter().take(3) {
        message.push_str(&format!(" Import `{path}` instead; that module exists."));
    }
    message
}

impl Resolver<'_> {
    /// Import spellings from the current package, including dependency aliases.
    fn import_paths(&self, target: ModuleId) -> Vec<String> {
        let here = self.p.module(self.cur);
        let module = self.p.module(target);
        if target == self.cur
            || module.kind == ModuleKind::Prelude
            || module.kind == ModuleKind::Test && here.kind != ModuleKind::Test
        {
            return Vec::new();
        }
        let package = &self.graph.packages[here.package];
        let Some((_, rest)) = module.path.split_once('/') else {
            return Vec::new();
        };
        let mut prefixes = BTreeSet::new();
        if module.package == self.graph.library {
            prefixes.insert("i".to_string());
        } else if module.package == here.package {
            prefixes.insert(package.name.clone());
        } else {
            for (name, id) in package.dependencies.iter().chain(
                package
                    .dev_dependencies
                    .iter()
                    .filter(|_| here.kind == ModuleKind::Test),
            ) {
                if *id == Some(module.package) {
                    prefixes.insert(name.clone());
                }
            }
        }
        prefixes
            .into_iter()
            .map(|p| format!("{p}/{rest}"))
            .collect()
    }

    pub(super) fn undefined_name_message(&self, name: &str, module_only: bool) -> String {
        let mut message = if module_only {
            format!("there is no module `{name}`")
        } else {
            format!("`{name}` is not defined")
        };
        let mut candidates = BTreeSet::new();
        for (mi, module) in self.p.modules.iter().enumerate() {
            let target = ModuleId(mi as u32);
            let is_module = module.path.rsplit('/').next() == Some(name);
            let is_export = !module_only
                && (module.defs.iter().any(|d| d.exported && d.name == name)
                    || self.macros_by_name[mi].get(name) == Some(&true)
                    || self.ctors_by_name[mi]
                        .get(name)
                        .is_some_and(|c| self.p.ty(c.ty).constructors_exported));
            if !is_module && !is_export {
                continue;
            }
            for path in self.import_paths(target) {
                let advice = if is_module {
                    format!("Import `{path}` to use the module `{name}`.")
                } else if let Some((alias, _)) = self
                    .p
                    .module(self.cur)
                    .import_names
                    .iter()
                    .find(|(_, m)| *m == target)
                {
                    format!(
                        "Add `{name}` to the names in your existing import of `{path}`, or use `{alias}.{name}`."
                    )
                } else {
                    format!("Import `({name}) = {path}` to use `{name}`.")
                };
                candidates.insert((!is_module, path, advice));
            }
        }
        if !candidates.is_empty() {
            message.push('.');
        }
        for (_, _, advice) in candidates.into_iter().take(3) {
            message.push(' ');
            message.push_str(&advice);
        }
        message
    }

    pub(super) fn missing_module_message(&self, written: &str) -> String {
        let here = self.p.module(self.cur);
        missing_module_message(self.graph, here.package, here.kind, written)
    }

    pub(super) fn same_named_type_hint(&self, written: &Type) -> String {
        let TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } = &written.kind
        else {
            return String::new();
        };
        let original = self.p.ty(*id);
        let mut candidates = BTreeSet::new();
        for ty in &self.p.types {
            if ty.name != original.name || ty.module == original.module || !ty.exported {
                continue;
            }
            for path in self.import_paths(ty.module) {
                let alias = self
                    .p
                    .module(self.cur)
                    .import_names
                    .iter()
                    .find(|(_, m)| *m == ty.module)
                    .map(|(a, _)| a.as_str());
                let advice = if let Some(alias) = alias {
                    format!(
                        "`{path}` also exports a type named `{}`. Use `{alias}.{}` if that is the type you intended.",
                        ty.name, ty.name
                    )
                } else {
                    let alias = path.rsplit('/').next().unwrap();
                    format!(
                        "`{path}` also exports a type named `{}`. Import `{path}` and use `{alias}.{}` if that is the type you intended.",
                        ty.name, ty.name
                    )
                };
                candidates.insert((path, advice));
            }
        }
        if candidates.is_empty() {
            return String::new();
        }
        let mut hint = format!(
            ". `{}` here comes from `{}`.",
            original.name,
            self.module_name(original.module)
        );
        for (_, advice) in candidates.into_iter().take(3) {
            hint.push(' ');
            hint.push_str(&advice);
        }
        hint
    }
}
