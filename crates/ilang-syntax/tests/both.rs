//! A1 design D3 and §6.1: Both is Old plus error-only extensions, with
//! exactly E0113 and J1 as the named exceptions.
mod common;

use ilang_syntax::{Code, parse_module, parse_module_old, parser::parse_module_uncached};

fn agrees(name: &str, source: &str) -> (bool, bool) {
    let (both, bd) = parse_module(source);
    // D15 and §3.13 apply to every diagnostic-free Both tree, including
    // new forms that Old mode rejects and accepted mutations of them.
    if bd.is_empty() {
        assert_eq!(
            ilang_syntax::print::unprintable(&both),
            None,
            "{name}\n{source}"
        );
        let printed = ilang_syntax::print::print_module(&both);
        let (again, diagnostics) = parse_module(&printed);
        assert!(
            diagnostics.is_empty(),
            "{name}\n{source}\n{printed}\n{diagnostics:?}"
        );
        assert_eq!(
            common::shape(&both),
            common::shape(&again),
            "{name}\n{source}\n{printed}"
        );
    }
    let (old, od) = parse_module_old(source);
    if !od.is_empty() {
        return (false, false);
    }
    let same = common::shape(&old) == common::shape(&both);
    let ambiguous = bd.iter().any(|d| d.code == Code::DualAmbiguous);
    if !same || bd.iter().any(|d| d.code != Code::DualAmbiguous) {
        // D3 J1: an unqualified lower TypeCall replaces the end of an old
        // type. The actual AST, not source text, must contain it.
        let debug = format!("{both:?}");
        assert!(
            debug.contains("Call { module: None, name: Ident"),
            "unlisted mismatch: {name}\n{source}\nold: {old:?}\nBoth: {both:?}\n{bd:?}"
        );
        assert!(
            ilang_syntax::format::format(source).is_err(),
            "J1 must refuse formatting: {name}"
        );
        return (ambiguous, true);
    }
    (ambiguous, false)
}

#[test]
fn both_agrees_with_old() {
    let sources = common::corpus();
    assert!(sources.len() > 500);
    let baseline = common::baseline_paths();
    eprintln!("comparing {} corpus sources", sources.len());
    let mut exceptions = (0, 0);
    for (name, source) in sources {
        let (ambiguous, joined) = agrees(&name, &source);
        if baseline.contains(name.split(':').next().unwrap()) {
            exceptions.0 += usize::from(ambiguous);
            exceptions.1 += usize::from(joined);
        }
    }
    assert_eq!(exceptions, (0, 0), "D4 and J1 have no corpus hits at base");
}

#[test]
fn seeded_mutants_agree() {
    let sources = common::corpus();
    let mut seed = 0xA1_2026_u64;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as usize
    };
    let mut count = 0;
    assert!(
        sources.len() <= 25_000,
        "increase the mutation budget to cover every source"
    );
    // About 50,000 parses. Every corpus source gets a turn before cycling.
    for (name, source) in sources.iter().cycle().take(25_000) {
        let tokens = ilang_syntax::lex(source).tokens;
        let at = tokens[random() % tokens.len()].span.start as usize;
        let mutation = random() % 5;
        let mut mutant = source.clone();
        match mutation {
            0 => mutant.insert(at, '\n'),
            1 => {
                if let Some(pos) = mutant[..at].rfind('\n') {
                    mutant.remove(pos);
                }
            }
            2 => mutant.insert_str(at, "-> "),
            3 => {
                if let Some(pos) = mutant[at..].find('{') {
                    mutant.replace_range(at + pos..at + pos + 1, "(");
                }
            }
            _ => {
                if let Some(pos) = mutant[at..].find('(') {
                    let pos = at + pos;
                    if pos > 0 && mutant.as_bytes()[pos - 1] == b'\n' {
                        mutant.replace_range(pos - 1..pos, " ");
                    } else {
                        mutant.insert(pos, '\n');
                    }
                }
            }
        }
        agrees(name, &mutant);
        count += 2;
    }
    assert_eq!(count, 50_000);
}

#[test]
fn cache_does_not_change_the_parse() {
    for (name, source) in common::corpus() {
        let (cached, cd) = parse_module(&source);
        let (plain, pd) = parse_module_uncached(&source);
        assert_eq!(cd, pd, "{name}");
        assert_eq!(common::shape(&cached), common::shape(&plain), "{name}");
    }
}
