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
        let (_, diags) = ilang_syntax::parse_module(std::str::from_utf8(&source).unwrap());
        writeln!(output, "{}", diags.len()).unwrap();
        for d in diags {
            let message: String = d.message.bytes().map(|b| format!("{b:02x}")).collect();
            writeln!(output, "{} {message}", d.code.as_str()).unwrap();
        }
        line.clear();
    }
}
