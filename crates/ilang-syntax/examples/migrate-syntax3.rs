//! The syntax three migrator (docs/design/syntax-3.md §7).
//!
//! ```text
//! cargo run -p ilang-syntax --example migrate-syntax3 -- <subcommand> [path …]
//! cargo run -p ilang-syntax --example migrate-syntax3 -- --check <subcommand> [path …]
//! ```
//!
//! A subcommand rewrites, in place, every i source the old-syntax check reads
//! (`.i` files, `i` fences in Markdown, i source in Rust string literals),
//! limited to the given paths when there are any, and moves the entries of
//! `scripts/old-syntax-check.json` to the new text. It prints the forms it
//! leaves for manual work. `--check` changes nothing and prints the number of
//! forms of the subcommand that remain; it exits 1 when that is not 0.
//!
//! Each subcommand is a module in `migrate-syntax3/` with a `NAME` and a
//! `run(&Source) -> Outcome`, and one row in `SUBCOMMANDS` at the bottom.

#[path = "migrate-syntax3/common.rs"]
mod common;
#[path = "migrate-syntax3/types.rs"]
mod types;

use common::{Migrate, Outcome};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.first().is_some_and(|a| a == "--check");
    if check {
        args.remove(0);
    }
    let Some(name) = args.first().cloned() else {
        usage();
    };
    let Some((_, migrate)) = SUBCOMMANDS.iter().find(|(n, _)| *n == name) else {
        usage();
    };
    let root = common::root();
    let mut remaining = 0;
    let mut changed_files = 0;
    let mut changed_sources = 0;
    let mut manual = Vec::new();
    let mut moves = Vec::new();
    for (path, (host, sources)) in common::sources(&root, &args[1..]) {
        let mut changes = Vec::new();
        for s in &sources {
            let Outcome {
                text,
                found,
                remaining: r,
                manual: m,
            } = migrate(s);
            manual.extend(m);
            if check {
                remaining += found;
                continue;
            }
            remaining += r;
            if let Some(text) = text {
                moves.push((
                    common::identity(&s.path, &s.kind, &s.text),
                    common::identity(&s.path, &s.kind, &text),
                ));
                changes.push((s, text));
            }
        }
        if !changes.is_empty() {
            changed_files += 1;
            changed_sources += changes.len();
            let new = common::rewrite_host(&host, &changes);
            std::fs::write(root.join(&path), new).unwrap();
        }
    }
    common::rekey(&root, &moves);
    for m in &manual {
        eprintln!("manual: {m}");
    }
    if check {
        println!("{name}: {remaining} remaining");
        if remaining > 0 {
            std::process::exit(1);
        }
    } else {
        println!(
            "{name}: {changed_sources} sources in {changed_files} files rewritten; {remaining} left for manual work"
        );
    }
}

fn usage() -> ! {
    let names: Vec<_> = SUBCOMMANDS.iter().map(|(n, _)| *n).collect();
    eprintln!("usage: migrate-syntax3 [--check] <{}> [path …]", names.join("|"));
    std::process::exit(2)
}

/// One row per subcommand.
const SUBCOMMANDS: &[(&str, Migrate)] = &[
    (types::NAME, types::run),
];
