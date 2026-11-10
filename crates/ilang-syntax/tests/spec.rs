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
