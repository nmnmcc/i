//! Build ordinary source and test modules for a single-file program. The
//! compiler gets no special entry or test-module rule from this harness.
use ilang_syntax::ast::{ItemKind, PatternKind, TypeBody};
use std::path::Path;

pub fn write(dir: &Path, stem: &str, source: &str) {
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    let mut tests = Vec::new();
    let mut exports = Vec::new();
    let mut edits = Vec::new();
    if diagnostics.is_empty() {
        for item in &module.items {
            if let ItemKind::TypeDef(def) = &item.kind
                && !def.alias_pattern
            {
                exports.push((
                    def.name.name.clone(),
                    matches!(def.body, TypeBody::Custom(_)),
                ));
            }
            let ItemKind::Binding(binding) = &item.kind else {
                continue;
            };
            let name = match &binding.pattern.kind {
                PatternKind::Var(name) => name,
                PatternKind::Typed { binder, .. } => &binder.name,
                _ => continue,
            };
            // A default entry marked as a test remains a source placement
            // error, just as it would in an ordinary src/main.i.
            if name == "main" {
                continue;
            }
            if item
                .attributes
                .iter()
                .any(|attr| attr.module.is_none() && attr.name.name == "main")
            {
                continue;
            }
            for attr in &item.attributes {
                if attr.module.is_none()
                    && attr.name.name == "test"
                    && attr.args.is_empty()
                    && attr.call.is_none()
                {
                    tests.push(name.clone());
                    exports.push((name.clone(), false));
                    edits.push((
                        attr.span.start as usize,
                        attr.span.end as usize,
                        " ".repeat((attr.span.end - attr.span.start) as usize),
                    ));
                }
            }
        }
    }
    let mut text = source.to_string();
    if !tests.is_empty() {
        exports.retain(|(name, _)| !module.exports.iter().any(|e| e.name.name == *name));
        let items = exports
            .iter()
            .map(|(name, ctors)| {
                if *ctors {
                    format!("{name}(..)")
                } else {
                    name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        if !items.is_empty() {
            if let Some(span) = module.exports_span {
                edits.push((
                    span.end as usize - 1,
                    span.end as usize - 1,
                    format!(", {items}"),
                ));
            } else {
                let at = module.imports_span.map_or(0, |s| s.end as usize);
                edits.push((at, at, format!("({items}) ")));
            }
        }
        edits.sort_by_key(|(start, end, _)| (*start, *end));
        for (start, end, replacement) in edits.into_iter().rev() {
            text.replace_range(start..end, &replacement);
        }
        let mut alias = "single_file_source".to_string();
        while tests.contains(&alias) {
            alias.push('_');
        }
        let mut wrappers = format!("({alias} = spec/{stem})\n");
        for name in tests {
            wrappers.push_str(&format!("@test\n{name} = () {{ {alias}.{name}() }}\n"));
        }
        std::fs::create_dir_all(dir.join("test")).unwrap();
        std::fs::write(dir.join("test").join(format!("{stem}.i")), wrappers).unwrap();
    }
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join(format!("{stem}.i")), text).unwrap();
}
