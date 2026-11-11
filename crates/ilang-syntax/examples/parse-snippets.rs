//! A length-prefixed parser protocol for scripts/old-syntax-check.
use std::io::{self, BufRead, Read, Write};

fn main() {
    let mut input = io::stdin().lock();
    let mut output = io::BufWriter::new(io::stdout().lock());
    let mut line = String::new();
    while input.read_line(&mut line).unwrap() != 0 {
        let len: usize = line.trim().parse().unwrap();
        let mut source = vec![0; len];
        input.read_exact(&mut source).unwrap();
        let (module, diags) = ilang_syntax::parse_module(std::str::from_utf8(&source).unwrap());
        let old_main = module
            .items
            .iter()
            .flat_map(|item| &item.attributes)
            .filter(|a| a.module.is_none() && a.name.name == "main")
            .count();
        writeln!(output, "{}", diags.len() + old_main).unwrap();
        for d in diags {
            let message: String = d.message.bytes().map(|b| format!("{b:02x}")).collect();
            writeln!(output, "{} {message}", d.code.as_str()).unwrap();
        }
        for _ in 0..old_main {
            let message: String = "removed @main attribute"
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect();
            writeln!(output, "E0301 {message}").unwrap();
        }
        line.clear();
    }
}
