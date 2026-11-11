//! Name lookup follows exact scope and module rules for expanded code.
use ilang_check::hygiene::{Binding, Lookup, ModuleName, lookup};
use ilang_syntax::lisp::{
    parse,
    syntax::{Node, Scope, Syntax},
};
use std::collections::BTreeSet;

fn atom(module: u32, marks: &[u64]) -> Syntax {
    Syntax::from_datum(
        &parse("name").unwrap()[0],
        &Scope {
            module,
            marks: marks.iter().copied().collect::<BTreeSet<_>>(),
        },
    )
}
fn binding(syntax: &Syntax, target: u32) -> Binding<u32> {
    Binding {
        text: "name".into(),
        scope: syntax.scope().clone(),
        identity: None,
        target,
    }
}

#[test]
fn nearest_exact_scope_wins_without_capturing_other_marks() {
    let caller = atom(1, &[]);
    let template = atom(2, &[7]);
    let locals = [
        binding(&caller, 10),
        binding(&template, 11),
        binding(&caller, 12),
    ];
    assert_eq!(
        lookup(&caller, 1, &locals, &[], |_, _| None, |_| None, |_| None),
        Lookup::Found(12)
    );
    assert_eq!(
        lookup(&template, 1, &locals, &[], |_, _| None, |_| None, |_| None),
        Lookup::Found(11)
    );
    assert_eq!(
        lookup(
            &atom(1, &[7]),
            1,
            &locals,
            &[],
            |_, _| None,
            |_| None,
            |_| None
        ),
        Lookup::<u32>::Missing
    );
}

#[test]
fn marked_definitions_precede_template_module_and_answer_made_names_do_not() {
    let written = atom(2, &[3]);
    let generated = [binding(&written, 11)];
    let module = |id, _name: &str| {
        assert_eq!(id, 2);
        Some(ModuleName {
            target: 12,
            exported: true,
        })
    };
    assert_eq!(
        lookup(&written, 1, &[], &generated, module, |_| None, |_| None),
        Lookup::Found(11)
    );
    let answer = written.as_answer();
    let made = Syntax::make(Node::Name("name".into()), &answer).unwrap();
    assert_eq!(
        lookup(&made, 1, &[], &generated, module, |_| None, |_| None),
        Lookup::Found(12)
    );
}

#[test]
fn private_template_names_are_allowed_but_foreign_made_names_are_not() {
    let template = atom(2, &[9]);
    let made = Syntax::make(Node::Name("name".into()), &template).unwrap();
    let private = |_, _: &str| {
        Some(ModuleName {
            target: 8,
            exported: false,
        })
    };
    assert_eq!(
        lookup(&template, 1, &[], &[], private, |_| None, |_| None),
        Lookup::Found(8)
    );
    assert_eq!(
        lookup(&made, 1, &[], &[], private, |_| None, |_| None),
        Lookup::NotExported
    );
    let own = atom(1, &[]);
    let own_made = Syntax::make(Node::Name("name".into()), &own).unwrap();
    assert_eq!(
        lookup(&own_made, 1, &[], &[], private, |_| None, |_| None),
        Lookup::Found(8)
    );
}

#[test]
fn compiler_answer_identities_bypass_ordinary_shadowing() {
    use ilang_syntax::lisp::syntax::ScopeIdentity;
    let written = atom(1, &[]);
    let local = binding(&written, 1);
    let builtin = written.with_scope_identity(ScopeIdentity::Builtin {
        name: "name".into(),
    });
    assert_eq!(
        lookup(
            &builtin,
            1,
            &[local],
            &[],
            |_, _| None,
            |_| Some(2),
            |_| Some(3)
        ),
        Lookup::Found(2)
    );
    let id = ScopeIdentity::DefinitionParameter {
        answer: 7,
        parameter: 0,
        name: "name".into(),
    };
    let parameter = written.with_scope_identity(id.clone());
    let mut binder = binding(&written, 4);
    binder.identity = Some(id);
    assert_eq!(
        lookup(
            &parameter,
            1,
            &[binder],
            &[],
            |_, _| None,
            |_| Some(2),
            |_| Some(3)
        ),
        Lookup::Found(4)
    );
    assert_eq!(
        lookup(
            &parameter,
            1,
            &[],
            &[],
            |_, _| None,
            |_| Some(2),
            |_| Some(3)
        ),
        Lookup::Missing
    );
}
