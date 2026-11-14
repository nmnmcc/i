//! Every ```i block of `docs/spec.md` parses as syntax three without errors,
//! and its print parses to the same tree.

#[path = "oracle.rs"]
mod ilang_syntax;
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
    use ilang_syntax::syn3;
    let spec = include_str!("../../../docs/spec.md");
    let blocks = blocks(spec);
    assert!(blocks.len() >= 11, "found {} blocks", blocks.len());
    for (line, src) in blocks {
        let (module, diags) = syn3::parse_module(&src);
        assert!(
            diags.is_empty(),
            "spec.md block at line {line}: {diags:?}\n{src}"
        );
        let printed = syn3::print_module(&module);
        let (again, diags) = syn3::parse_module(&printed);
        assert!(
            diags.is_empty(),
            "print of block at line {line}: {diags:?}\n{printed}"
        );
        assert_eq!(module, again, "block at line {line}:\n{printed}");
    }
}
