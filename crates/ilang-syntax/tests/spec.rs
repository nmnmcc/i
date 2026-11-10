//! Every ```i block of `docs/spec.md` parses without errors, and its print
//! parses to the same tree.

mod common;

use ilang_syntax::parse_module;
use ilang_syntax::print::print_module;

fn blocks(spec: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut current: Option<(usize, String)> = None;
    for (n, line) in spec.lines().enumerate() {
        match &mut current {
            None if line.trim_end() == "```i" => current = Some((n + 1, String::new())),
            Some(_) if line.trim_end() == "```" => out.push(current.take().unwrap()),
            Some((_, text)) => {
                text.push_str(line);
                text.push('\n');
            }
            None => {}
        }
    }
    out
}

#[test]
fn spec_blocks_parse() {
    let spec = include_str!("../../../docs/spec.md");
    let blocks = blocks(spec);
    assert!(blocks.len() >= 11, "found {} blocks", blocks.len());
    for (line, src) in blocks {
        let (module, diags) = parse_module(&src);
        assert!(
            diags.is_empty(),
            "spec.md block at line {line}: {diags:?}\n{src}"
        );
        assert_eq!(ilang_syntax::print::unprintable(&module), None);
        let printed = print_module(&module);
        let (again, diags) = parse_module(&printed);
        assert!(
            diags.is_empty(),
            "print of block at line {line}: {diags:?}\n{printed}"
        );
        assert_eq!(
            common::shape(&module),
            common::shape(&again),
            "block at line {line}:\n{printed}"
        );
    }
}

#[test]
fn pending_literal_types_have_the_expected_parse_errors() {
    // literal-types §12 step 1 explicitly stages written literal syntax
    // until step 8. old-syntax-check also parses this fence against its
    // T1 entry. Keep implemented snippets under spec_blocks_parse above.
    let spec = include_str!("../../../docs/spec.md");
    let pending: Vec<_> = spec.split("```i pending=T1\n").skip(1).collect();
    assert_eq!(pending.len(), 1, "update this fixture when T1 step 8 lands");
    let source = pending[0].split_once("\n```").unwrap().0;
    let (_, diags) = parse_module(source);
    let actual: Vec<_> = diags.iter().map(|d| (d.code, d.message.as_str())).collect();
    assert_eq!(
        actual,
        [
            (
                ilang_syntax::Code::Unexpected,
                "expected a type, found a string"
            ),
            (
                ilang_syntax::Code::Unexpected,
                "expected a type, found a string"
            ),
        ]
    );
}
