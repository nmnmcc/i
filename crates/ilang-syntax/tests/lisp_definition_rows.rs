//! The definition answers in the item rows of syntax-3.md §6.8 (step 2c of
//! docs/design/syn3-final-ast.md, question 1): a sum answers
//! `(= T (| C1 C2))` and each member is its own item `(= C (, …))`. The
//! `_rows` functions are tested with the rows given, so these tests do not
//! depend on `NEW_ROWS`.
#[path = "oracle.rs"]
mod ilang_syntax;
use ilang_syntax::lisp::definitions::{
    builtin_definition_names, builtin_definition_rows, expanded_definition_rows,
    freshen_duplicate_heads_rows, freshen_parameters, tree_definition,
};
use ilang_syntax::lisp::parse;
use ilang_syntax::lisp::syntax::{Node, Scope, ScopeIdentity, Syntax};
use ilang_syntax::syn3;
use ilang_syntax::syn3::ast::{ItemKind, TypeKind, TypeRhs};
use std::collections::BTreeSet;

fn root(source: &str) -> Syntax {
    let document = parse(source).unwrap();
    Syntax::from_datum(
        &document[0],
        &Scope {
            module: 7,
            marks: BTreeSet::new(),
        },
    )
}

fn parts(s: &Syntax) -> &[Syntax] {
    match s.node() {
        Node::Round(xs) | Node::Curly(xs) | Node::Square(xs) => xs,
        _ => panic!("expected a compound, found {}", s.show()),
    }
}

fn parameter(s: &Syntax) -> Option<(u64, u32)> {
    match s.scope_identity() {
        Some(ScopeIdentity::DefinitionParameter {
            answer, parameter, ..
        }) => Some((*answer, *parameter)),
        _ => None,
    }
}

const NEW: [(&str, &str); 9] = [
    ("Bool", "(= Bool (| True False))"),
    ("True", "(= True (,))"),
    ("False", "(= False (,))"),
    ("Nil", "(= Nil (,))"),
    ("Result", "(= (Result A E) (| (Ok A) (Error E)))"),
    ("Ok", "(= (Ok A) (, A))"),
    ("Error", "(= (Error E) (, E))"),
    (
        "AssertionFailure",
        "(= AssertionFailure (, (= message String)))",
    ),
    (
        "Assert",
        "(= Assert (, (= fail (-> (, AssertionFailure) Never))))",
    ),
];

const OLD: [(&str, &str); 5] = [
    ("Bool", "(Bool {True False})"),
    ("Nil", "(Nil {Nil})"),
    ("Result", "((Result A E) {(Ok A) (Error E)})"),
    (
        "AssertionFailure",
        "(AssertionFailure {(AssertionFailure (: message String))})",
    ),
    (
        "Assert",
        "(Assert {(Assert (: fail (-> ((: failure AssertionFailure)) Never)))})",
    ),
];

#[test]
fn every_builtin_definition_has_its_row() {
    let like = root("x");
    assert_eq!(
        builtin_definition_names(true),
        NEW.map(|(name, _)| name).as_slice()
    );
    for (name, row) in NEW {
        let answer = builtin_definition_rows(name, like.scope(), like.span(), 3, true).unwrap();
        assert_eq!(answer.show(), row, "{name}");
    }
    assert_eq!(
        builtin_definition_names(false),
        OLD.map(|(name, _)| name).as_slice()
    );
    for (name, row) in OLD {
        let answer = builtin_definition_rows(name, like.scope(), like.span(), 3, false).unwrap();
        assert_eq!(answer.show(), row, "{name}");
    }
    for rows in [false, true] {
        assert!(builtin_definition_rows("Int", like.scope(), like.span(), 3, rows).is_none());
    }
    // A member of a sum is a type of its own only in the rows of §6.8.
    for member in ["True", "False", "Ok", "Error"] {
        assert!(builtin_definition_rows(member, like.scope(), like.span(), 3, false).is_none());
    }
}

/// Each row is an item of the Lisp surface of syntax three, of the kind that
/// §6.8 gives it, and the Lisp writer prints it back the same.
#[test]
fn every_builtin_row_is_an_item_of_the_lisp_surface() {
    let like = root("x");
    for (name, row) in NEW {
        let answer = builtin_definition_rows(name, like.scope(), like.span(), 3, true).unwrap();
        let (module, errors) = syn3::lisp::from_lisp(&answer.show());
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let [item] = module.items.as_slice() else {
            panic!("{name}: one item")
        };
        let ItemKind::TypeDef(def) = &item.kind else {
            panic!("{name}: a type definition")
        };
        assert_eq!(def.name.name, name);
        match (&def.rhs, name) {
            (TypeRhs::Type(ty), "Bool" | "Result") => {
                assert!(matches!(&ty.kind, TypeKind::Union(members) if members.len() == 2));
            }
            (TypeRhs::Data(_), "Bool" | "Result") | (TypeRhs::Type(_) | TypeRhs::Opaque, _) => {
                panic!("{name}: wrong kind of right side")
            }
            (TypeRhs::Data(items), _) => {
                let count = match name {
                    "True" | "False" | "Nil" => 0,
                    _ => 1,
                };
                assert_eq!(items.len(), count, "{name}");
            }
        }
        assert_eq!(syn3::lisp::to_lisp(&module).trim(), row, "{name}");
    }
}

#[test]
fn builtin_rows_keep_builtin_names_and_fresh_parameter_binders() {
    let like = root("x");
    let result = builtin_definition_rows("Result", like.scope(), like.span(), 3, true).unwrap();
    let item = parts(&result);
    let head = parts(&item[1]);
    assert!(
        matches!(head[0].scope_identity(), Some(ScopeIdentity::Builtin { name }) if name == "Result")
    );
    assert_eq!(parameter(&head[1]), Some((3, 0)));
    assert_eq!(parameter(&head[2]), Some((3, 1)));
    assert_eq!(item[1].definition_head(), Some(3));
    let members = parts(&item[2]);
    let ok = parts(&members[1]);
    let error = parts(&members[2]);
    assert!(
        matches!(ok[0].scope_identity(), Some(ScopeIdentity::Builtin { name }) if name == "Ok")
    );
    assert_eq!(head[1].scope_identity(), ok[1].scope_identity());
    assert_eq!(head[2].scope_identity(), error[1].scope_identity());

    // A member's parameter is the first of its own head.
    for (member, letter) in [("Ok", "A"), ("Error", "E")] {
        let answer = builtin_definition_rows(member, like.scope(), like.span(), 4, true).unwrap();
        let item = parts(&answer);
        let head = parts(&item[1]);
        assert_eq!(parameter(&head[1]), Some((4, 0)), "{member}");
        assert!(matches!(head[1].node(), Node::Up(name) if name == letter));
        assert_eq!(item[1].definition_head(), Some(4));
        let data = parts(&item[2]);
        assert_eq!(head[1].scope_identity(), data[1].scope_identity());
    }

    // Separate answers do not share binders; a head without parameters is
    // not a binding head.
    let second = builtin_definition_rows("Result", like.scope(), like.span(), 5, true).unwrap();
    assert_ne!(
        head[1].scope_identity(),
        parts(&parts(&second)[1])[1].scope_identity()
    );
    let assert = builtin_definition_rows("Assert", like.scope(), like.span(), 6, true).unwrap();
    assert_eq!(parts(&assert)[1].definition_head(), None);
    let never = &parts(&parts(&parts(&parts(&assert)[2])[1])[2])[2];
    assert!(
        matches!(never.scope_identity(), Some(ScopeIdentity::Builtin { name }) if name == "Never")
    );
}

#[test]
fn copies_of_a_builtin_row_are_freshened_as_a_unit() {
    let like = root("x");
    let original = builtin_definition_rows("Result", like.scope(), like.span(), 3, true).unwrap();
    let copy = freshen_parameters(&original, 3, 8);
    let item = parts(&copy);
    let head = parts(&item[1]);
    assert_eq!(item[1].definition_head(), Some(8));
    assert_eq!(parameter(&head[1]), Some((8, 0)));
    assert_eq!(
        head[2].scope_identity(),
        parts(&parts(&item[2])[2])[1].scope_identity()
    );

    let ok = builtin_definition_rows("Ok", like.scope(), like.span(), 50, true).unwrap();
    let attributed = root("(@ deprecated x)");
    let wrapped = attributed.replace_children(vec![
        parts(&attributed)[0].clone(),
        parts(&attributed)[1].clone(),
        ok.clone(),
    ]);
    let items = Syntax::make(Node::Curly(vec![ok.clone(), wrapped, original]), &like).unwrap();
    let mut next = 51;
    let output = freshen_duplicate_heads_rows(&items, true, || {
        next += 1;
        next - 1
    });
    let items = parts(&output);
    let first = parts(&items[0]);
    let second = parts(&parts(&items[1])[2]);
    for (item, id) in [(first, 50), (second, 51)] {
        assert_eq!(item[1].definition_head(), Some(id));
        assert_eq!(parameter(&parts(&item[1])[1]), Some((id, 0)));
        assert_eq!(
            parts(&item[1])[1].scope_identity(),
            parts(&item[2])[1].scope_identity()
        );
    }
    // Another definition keeps its identity.
    assert_eq!(parts(&items[2])[1].definition_head(), Some(3));
    assert_eq!(next, 52);
}

#[test]
fn expanded_rows_give_the_parameters_of_the_head_their_identity() {
    let item = root("(= (Pair A B) (, A (= right (List B)) (-> (, A) (<- B (, (Fail A) .. B)))))");
    let answer = expanded_definition_rows(&item, 41, true).unwrap();
    assert_eq!(answer.show(), item.show());
    let item = parts(&answer);
    let head = parts(&item[1]);
    assert_eq!(item[1].definition_head(), Some(41));
    assert!(head[0].scope_identity().is_none());
    assert_eq!(parameter(&head[1]), Some((41, 0)));
    assert_eq!(parameter(&head[2]), Some((41, 1)));
    let data = parts(&item[2]);
    assert_eq!(parameter(&data[1]), Some((41, 0)));
    let list = parts(&parts(&data[2])[2]);
    assert!(list[0].scope_identity().is_none());
    assert_eq!(parameter(&list[1]), Some((41, 1)));
    let function = parts(&data[3]);
    assert_eq!(parameter(&parts(&function[1])[1]), Some((41, 0)));
    let result = parts(&function[2]);
    assert_eq!(parameter(&result[1]), Some((41, 1)));
    let needs = parts(&result[2]);
    assert_eq!(parameter(&parts(&needs[1])[1]), Some((41, 0)));
    assert_eq!(parameter(&needs[3]), Some((41, 1)));
}

#[test]
fn expanded_rows_identify_only_parameters_with_the_same_scope() {
    let prototype = root("(= (Box A) (, A A))");
    let item = parts(&prototype);
    let data = parts(&item[2]);
    let foreign = data[2].flip_mark(9);
    let changed = prototype.replace_children(vec![
        item[0].clone(),
        item[1].clone(),
        item[2].replace_children(vec![data[0].clone(), data[1].clone(), foreign]),
    ]);
    let answer = expanded_definition_rows(&changed, 12, true).unwrap();
    let data = parts(&parts(&answer)[2]);
    assert_eq!(parameter(&data[1]), Some((12, 0)));
    assert_eq!(parameter(&data[2]), None);

    // The parameters of a polymorphic function type are its own; a
    // qualified name is not a parameter.
    let answer =
        expanded_definition_rows(&root("(= (F A) (, (-> [A] (, A) A) (. m A) A))"), 13, true)
            .unwrap();
    let data = parts(&parts(&answer)[2]);
    let function = parts(&data[1]);
    assert_eq!(parameter(&parts(&function[1])[0]), None);
    assert_eq!(parameter(&parts(&function[2])[1]), None);
    assert_eq!(parameter(&function[3]), None);
    assert_eq!(parameter(&parts(&data[2])[2]), None);
    assert_eq!(parameter(&data[3]), Some((13, 0)));
}

#[test]
fn expanded_rows_keep_the_three_kinds_of_right_side_apart() {
    let show = |source: &str| {
        expanded_definition_rows(&root(source), 20, true)
            .unwrap()
            .show()
    };
    // Labelled type, opaque type, name (§6.8, "Files and items").
    assert_eq!(show("(= U (, T1 (= x T2)))"), "(= U (, T1 (= x T2)))");
    assert_eq!(show("(= (U A) (, A))"), "(= (U A) (, A))");
    assert_eq!(show("(= U (,))"), "(= U (,))");
    assert_eq!(show("(= (U A) Opaque)"), "(= (U A) Opaque)");
    assert_eq!(show("(= (U A) (List A))"), "(= (U A) (List A))");
    assert_eq!(show("(= T (| C1 C2))"), "(= T (| C1 C2))");
    // The group of the item form stays: a name whose right side is data.
    assert_eq!(show("(= U {(, T1 T2)})"), "(= U {(, T1 T2)})");
    assert_eq!(show("(= U {{(, T1 T2)}})"), "(= U {(, T1 T2)})");
    assert_eq!(show("(= U {Opaque})"), "(= U {Opaque})");
    // Type groups are removed everywhere else.
    assert_eq!(show("(= (U A) {(List {A})})"), "(= (U A) (List A))");
    assert_eq!(
        show("(= U (, {(, T1)} (= x {T2})))"),
        "(= U (, (, T1) (= x T2)))"
    );
    // An alias pattern keeps its head as written and binds nothing.
    let answer = expanded_definition_rows(&root("(= (Elem (List A)) A)"), 21, true).unwrap();
    assert_eq!(answer.show(), "(= (Elem (List A)) A)");
    assert_eq!(parts(&answer)[1].definition_head(), None);
    assert_eq!(parameter(&parts(&answer)[2]), None);
}

#[test]
fn expanded_rows_keep_attributes() {
    let item = root("(@ (deprecated \"old\") (@ doc (= (Box A) (, A))))");
    let answer = expanded_definition_rows(&item, 90, true).unwrap();
    assert_eq!(answer.show(), item.show());
    let inner = parts(&parts(&parts(&answer)[2])[2]);
    assert_eq!(inner[1].definition_head(), Some(90));
    assert_eq!(parameter(&parts(&inner[1])[1]), Some((90, 0)));
    assert_eq!(parameter(&parts(&inner[2])[1]), Some((90, 0)));
}

#[test]
fn expanded_rows_refuse_what_is_not_a_type_definition() {
    for source in [
        "(= x 1)",
        "(: x Int)",
        "(f 1)",
        "(Bool {True False})",
        "((Result A E) {(Ok A) (Error E)})",
        "(= U)",
        "U",
    ] {
        let error = expanded_definition_rows(&root(source), 1, true).unwrap_err();
        assert_eq!(
            error.message, "definition expects a type definition",
            "{source}"
        );
    }
}

/// The right side is checked without recursion; the check accepts what the
/// type reader of the Lisp surface accepts.
#[test]
fn expanded_rows_accept_the_types_of_the_lisp_surface() {
    let types = [
        ("Int", true),
        ("1", true),
        ("(- 1)", true),
        ("\"a\"", true),
        ("x", true),
        ("_", true),
        ("(. m T)", true),
        ("(. m x)", true),
        ("(List A)", true),
        ("((. m List) A)", true),
        ("(f A)", true),
        ("(f)", true),
        ("((. m f) A)", true),
        ("((. m f))", true),
        ("(,)", true),
        ("(, A (= x B))", true),
        ("(| A B)", true),
        ("(& A B)", true),
        ("(- A B C)", true),
        ("(-> (,) R)", true),
        ("(-> (, T1 (= x T2)) R)", true),
        ("(-> (, T) (<- R (, Db Log)))", true),
        ("(-> (, T) (<- R N))", true),
        ("(-> (, T) (<- R (, .. N Db)))", true),
        ("(-> [A N] (, A) (<- A N))", true),
        ("(-> (| (, T) (, T T)) R)", true),
        ("{A}", true),
        ("()", false),
        ("(T)", false),
        ("((. m T))", false),
        ("(| A)", false),
        ("(& A)", false),
        ("(- A)", false),
        ("(. m)", false),
        ("(. M T)", false),
        ("(, .. A)", false),
        ("(, (= x))", false),
        ("(, (= X A))", false),
        ("(-> (, A))", false),
        ("(-> (, A) B C)", false),
        ("(-> [a] (, A) B)", false),
        ("(-> (, A) (<- R))", false),
        ("(-> (, A) (<- R (, ..)))", false),
        ("(+ A B)", false),
        ("[A]", false),
        ("{A B}", false),
        ("{}", false),
        ("1.5", false),
        ("((f) A)", false),
        ("(1 A)", false),
        ("(: x A)", false),
        ("(! R N)", false),
        ("(^ R E)", false),
    ];
    for (ty, valid) in types {
        let read = syn3::lisp::type_from_datum(&parse(ty).unwrap()[0]);
        assert_eq!(read.is_ok(), valid, "the type reader on {ty}: {read:?}");
        let answer = expanded_definition_rows(&root(&format!("(= (T A) {ty})")), 1, true);
        assert_eq!(answer.is_ok(), valid, "the definition check on {ty}");
        let in_head = expanded_definition_rows(&root(&format!("(= (T {ty}) Int)")), 1, true);
        assert_eq!(in_head.is_ok(), valid, "the head check on {ty}");
    }
}

#[test]
fn deeply_nested_rows_use_a_bounded_host_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let prototype = root("(= (Box A) A)");
            let item = parts(&prototype);
            let mut ty = item[2].clone();
            for _ in 0..10_000 {
                ty = Syntax::make(Node::Curly(vec![ty.clone()]), &ty).unwrap();
            }
            let grouped = prototype.replace_children(vec![item[0].clone(), item[1].clone(), ty]);
            let answer = expanded_definition_rows(&grouped, 77, true).unwrap();
            assert_eq!(answer.show(), "(= (Box A) A)");

            let list = root("(List A)");
            let mut ty = item[2].clone();
            for _ in 0..10_000 {
                ty = list.replace_children(vec![parts(&list)[0].clone(), ty]);
            }
            let nested = prototype.replace_children(vec![item[0].clone(), item[1].clone(), ty]);
            drop(expanded_definition_rows(&nested, 78, true).unwrap());

            let wrapper = root("(@ deprecated x)");
            let mut attributed = answer;
            for _ in 0..10_000 {
                attributed = wrapper.replace_children(vec![
                    parts(&wrapper)[0].clone(),
                    parts(&wrapper)[1].clone(),
                    attributed,
                ]);
            }
            drop(expanded_definition_rows(&attributed, 79, true).unwrap());
        })
        .unwrap()
        .join()
        .unwrap();
}

/// A written definition of the new tree: a sum is `(= T (| C1 C2))` and each
/// member answers with its own item.
#[test]
fn written_definitions_of_the_new_tree_answer_in_the_rows() {
    let source = "\
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
Pair(A, B) = (left = A, right = List(B))
Handle = Opaque
Both(A) = { (A, A) }
Name(A) = List(A)
Run(A, N) = (A) <- (..N, Log, Fail(String)) -> A
";
    let (module, errors) = syn3::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let like = root("x");
    let rows: Vec<_> = module
        .items
        .iter()
        .map(|item| tree_definition(item, like.scope(), 17))
        .collect();
    let shown: Vec<_> = rows.iter().map(Syntax::show).collect();
    assert_eq!(
        shown,
        [
            "(= (Some A) (, A))",
            "(= None (,))",
            "(= (Option A) (| (Some A) None))",
            "(= (Pair A B) (, (= left A) (= right (List B))))",
            "(= Handle Opaque)",
            "(= (Both A) {(, A A)})",
            "(= (Name A) (List A))",
            "(= (Run A N) (-> (, A) (<- A (, .. N Log (Fail String)))))",
        ]
    );
    let option = parts(&rows[2]);
    let head = parts(&option[1]);
    assert_eq!(option[1].definition_head(), Some(17));
    assert_eq!(parameter(&head[1]), Some((17, 0)));
    let some = parts(&parts(&option[2])[1]);
    assert_eq!(head[1].scope_identity(), some[1].scope_identity());
    assert!(some[0].scope_identity().is_none());
    let pair = parts(&rows[3]);
    let right = parts(&parts(&parts(&pair[2])[2])[2]);
    assert_eq!(parameter(&right[1]), Some((17, 1)));
    // Every node is at its place in the source.
    assert_eq!(
        &source[head[1].span().start as usize..head[1].span().end as usize],
        "A"
    );
}

#[test]
fn definition_table_answers_a_definition_of_the_new_tree() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{
        Definition, DefinitionCost, DefinitionTable, Lookup, QueryResult,
    };
    let source = "Some(A) = (A)\nOption(A) = Some(A) | None\n";
    let (module, errors) = syn3::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let of = root("Option");
    let mut table = DefinitionTable::default();
    table.insert(
        1,
        Definition::tree(module.items[0].clone(), of.scope().clone(), true),
    );
    table.insert(
        2,
        Definition::tree(module.items[1].clone(), of.scope().clone(), false),
    );
    assert!(matches!(
        table.query(&of, Mode::Normal, 5, |_, _| Lookup::Found(2)),
        QueryResult::Deferred { .. }
    ));
    table.set_ready(&2, true);
    let mut charges = 0;
    let answer = table
        .query_metered(
            &of,
            Mode::Normal,
            6,
            |_, _| Lookup::Found(2),
            |cost| {
                assert!(matches!(cost, DefinitionCost::Datum(_)));
                charges += 1;
                Ok(())
            },
        )
        .unwrap();
    let QueryResult::Answer { key: 2, syntax } = answer else {
        panic!("expected an answer")
    };
    assert_eq!(charges, 1);
    assert_eq!(syntax.show(), "(= (Option A) (| (Some A) None))");
    assert_eq!(parameter(&parts(&parts(&syntax)[1])[1]), Some((6, 0)));
    let QueryResult::Answer { syntax, .. } =
        table.query(&root("Some"), Mode::Normal, 7, |_, _| Lookup::Found(1))
    else {
        panic!("expected an answer")
    };
    assert_eq!(syntax.show(), "(= (Some A) (, A))");
    // A refused charge stops the answer.
    let refused = table.query_metered(
        &of,
        Mode::Normal,
        8,
        |_, _| Lookup::Found(2),
        |_| Err("budget".into()),
    );
    assert!(refused.is_err());
    // A finished definition replaces a pending one.
    table.finish(
        &2,
        Definition::tree(module.items[0].clone(), of.scope().clone(), true),
    );
}

/// The integrator's `SourceTree` gives the written item with the spans of
/// its tokens; the table answers and charges from that datum.
#[test]
fn definition_table_answers_from_the_datum_of_the_source_tree() {
    use ilang_syntax::lisp::definition_schedule::Mode;
    use ilang_syntax::lisp::definition_table::{Definition, DefinitionTable, Lookup, QueryResult};
    use ilang_syntax::lisp::source_tree::SourceTree;
    let source = "Pair(A, B) = (left = A, right = List(B))\n";
    let (module, errors) = syn3::parse_module(source);
    assert!(errors.is_empty(), "{errors:?}");
    let of = root("Pair");
    let tree = SourceTree::new(of.scope(), source);
    let item = &module.items[0];
    let datum = tree.item_datum(item.span.0).expect("the written item");
    let mut table = DefinitionTable::default();
    table.insert(
        1,
        Definition::tree_with_datum(item.clone(), datum, of.scope().clone(), true),
    );
    let QueryResult::Answer { syntax, .. } =
        table.query(&of, Mode::Normal, 3, |_, _| Lookup::Found(1))
    else {
        panic!("expected an answer")
    };
    assert_eq!(
        syntax.show(),
        "(= (Pair A B) (, (= left A) (= right (List B))))"
    );
    assert_eq!(syntax.show(), tree_definition(item, of.scope(), 3).show());
    let head = parts(&parts(&syntax)[1]);
    assert_eq!(parameter(&head[2]), Some((3, 1)));
    // The answer is an item of the Lisp surface.
    assert!(syn3::lisp::item_from_datum(&syntax.to_datum()).is_ok());
}
