//! The parts of `definition_world` that read the tree, on the tree of
//! syntax three: what a type definition binds, whether its constructor is
//! hidden outside its module, and the type parameters of an item.
//!
//! In the tree of syntax three a sum `T = C1 | C2` and its members
//! `C1 = (…)`, `C2 = (…)` are three items, and each is a definition of its
//! own (syntax-3.md §6.8; the lowering makes one definition of them). The
//! table of the definitions (`lisp::definition_table`) and the rows of the
//! answers are not part of this module.
use ilang_syntax::syn3::ast::{
    Binding, ExprKind, Ident, Item, ItemKind, PatternKind, TItem, Type, TypeDef, TypeKind, TypeRhs,
};

/// `T(head) = value` where the right side is the type of a value and the
/// head is not empty: an alias pattern, which binds the names of its head.
pub fn alias_pattern(def: &TypeDef) -> bool {
    def.alias_pattern
}

/// The type names that a definition binds: its own name, or for an alias
/// pattern the plain names of its head.
pub fn type_binders(def: &TypeDef) -> Vec<&Ident> {
    let mut binders = Vec::new();
    if alias_pattern(def) {
        for part in &def.head {
            pattern_binders(part, &mut binders);
        }
    } else {
        binders.push(&def.name);
    }
    binders
}

/// The plain names of a type pattern: a name with no module and no
/// argument, inside named types and positional data.
pub fn pattern_binders<'a>(ty: &'a Type, output: &mut Vec<&'a Ident>) {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match &ty.kind {
            TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => output.push(name),
            TypeKind::Named { args, .. } => pending.extend(args.iter().rev()),
            // The old tree's tuple: data with positional items only, and
            // at least one.
            TypeKind::Data(items)
                if !items.is_empty() && items.iter().all(|i| matches!(i, TItem::Pos(_))) =>
            {
                pending.extend(items.iter().rev().map(|i| match i {
                    TItem::Pos(t) | TItem::Label(_, t) | TItem::Spread(t) => t,
                }))
            }
            _ => {}
        }
    }
}

/// A definition with a constructor of its own (data, or opaque): its
/// definition is given to another module only when the export shows the
/// constructors, `T(..)`.
pub fn has_own_constructors(def: &TypeDef) -> bool {
    !matches!(def.rhs, TypeRhs::Type(_))
}

/// The plain names of a head: the type parameters of a definition that is
/// not an alias pattern. `None` when a part is not a plain name.
pub fn head_parameters(head: &[Type]) -> Option<Vec<&Ident>> {
    head.iter()
        .map(|t| match &t.kind {
            TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => Some(name),
            _ => None,
        })
        .collect()
}

/// The type parameters of a binding: the `<A>` lists of the branches of
/// its chain, each name once per list that adds it, then those of its
/// function type.
pub fn binding_type_parameters(binding: &Binding) -> Vec<&Ident> {
    let mut generics: Vec<&Ident> = Vec::new();
    if let ExprKind::Chain(fs) = &binding.value.kind {
        for f in fs {
            let before = generics.len();
            for g in &f.generics {
                if !generics[..before].iter().any(|x| x.name == g.name) {
                    generics.push(g);
                }
            }
        }
    }
    if let PatternKind::Typed { ty, .. } = &binding.pattern.kind
        && let TypeKind::Fn(f) = &ty.kind
    {
        for g in &f.generics {
            if !generics.iter().any(|x| x.name == g.name) {
                generics.push(g);
            }
        }
    }
    generics
}

/// The type parameters that an item declares.
pub fn type_parameters(item: &Item) -> Vec<&Ident> {
    match &item.kind {
        ItemKind::TypeDef(def) if alias_pattern(def) => Vec::new(),
        ItemKind::TypeDef(def) => head_parameters(&def.head).unwrap_or_default(),
        ItemKind::Bind(binding) => binding_type_parameters(binding),
        ItemKind::Decl { ty, .. } => match &ty.kind {
            TypeKind::Fn(f) => f.generics.iter().collect(),
            _ => Vec::new(),
        },
        ItemKind::Macro { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilang_syntax::ast as old;

    /// The answers on the tree of syntax three and on its lowering.
    #[test]
    fn answers_agree_with_the_lowered_tree() {
        let source = "\
T(A, B) = (A, B)
U = Opaque
V(A) = List(A)
W((A, List(B)), C) = value
f = <A, B>(a: A) { a } | <B, C>(b: B) { b }
g: <A, N>(A) <- N -> A = (a) { a }
h: <A>(A) -> A
k = 1
";
        let (module, errors) = ilang_syntax::syn3::parse_module(source);
        assert!(errors.is_empty(), "{errors:?}");
        let (lowered, _) = ilang_syntax::syn3::lower::lower(&module);
        assert_eq!(module.items.len(), lowered.items.len());
        let names = |ids: Vec<&Ident>| ids.iter().map(|i| i.name.clone()).collect::<Vec<_>>();
        let old_names = |ids: &[old::Ident]| {
            ids.iter()
                .map(|i| i.name.clone())
                // The row parameter that the lowering adds.
                .filter(|n| !n.contains('%'))
                .collect::<Vec<_>>()
        };
        for (new, old) in module.items.iter().zip(&lowered.items) {
            let old_parameters = match &old.kind {
                old::ItemKind::TypeDef(def) => old_names(&def.params),
                old::ItemKind::Binding(b) => old_names(&b.type_params),
                old::ItemKind::Declaration { type_params, .. } => old_names(type_params),
                old::ItemKind::MacroItem { .. } => Vec::new(),
            };
            assert_eq!(names(type_parameters(new)), old_parameters);
            if let (ItemKind::TypeDef(new), old::ItemKind::TypeDef(old)) = (&new.kind, &old.kind) {
                assert_eq!(alias_pattern(new), old.alias_pattern, "{}", new.name.name);
                assert_eq!(
                    has_own_constructors(new),
                    matches!(old.body, old::TypeBody::Custom(_)),
                    "{}",
                    new.name.name
                );
            }
        }
        let ItemKind::TypeDef(w) = &module.items[3].kind else {
            panic!()
        };
        assert_eq!(names(type_binders(w)), ["A", "B", "C"]);
        let ItemKind::TypeDef(t) = &module.items[0].kind else {
            panic!()
        };
        assert_eq!(names(type_binders(t)), ["T"]);
    }
}
