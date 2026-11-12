//! Layout properties for both source surfaces, using the CLI conversion path's
//! comment-preserving encoder and the existing canonical core-tree comparison.
use ilang_syntax::{
    format::{TAB_WIDTH, WIDTH},
    lexer::lex_lisp,
    lisp, shape,
};
use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if path.is_dir() {
            if !name.starts_with('.') && !matches!(name, "target" | "node_modules" | "binaryen") {
                files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "i" || e == "is") {
            out.push(path);
        }
    }
}

fn check(source: &str) -> String {
    let (before, errors) = lisp::parse_source(source);
    assert!(errors.is_empty(), "{errors:?}\n{source}");
    let out = lisp::format(source).unwrap();
    let (after, errors) = lisp::parse_source(&out);
    assert!(errors.is_empty(), "{errors:?}\n{out}");
    assert_eq!(shape::canon(&before), shape::canon(&after), "{out}");
    assert_eq!(lisp::format(&out).unwrap(), out, "not idempotent");
    let comments = |s: &str| {
        lex_lisp(s)
            .comments
            .into_iter()
            .map(|c| {
                if c.text.starts_with("///") {
                    c.text
                } else {
                    c.text.trim_end().into()
                }
            })
            .collect::<Vec<String>>()
    };
    assert_eq!(comments(source), comments(&out), "comments changed");
    // Each overlong line must contain an indivisible token/comment spanning
    // its available width. Delimiters and reader prefixes may hug that token.
    let lexed = lex_lisp(&out);
    let mut start = 0;
    for line in out.split_inclusive('\n') {
        let line = line.trim_end_matches('\n');
        let width = line
            .chars()
            .map(|c| if c == '\t' { TAB_WIDTH } else { 1 })
            .sum::<usize>();
        if width > WIDTH {
            use ilang_syntax::token::TokenKind as T;
            let atoms = lexed
                .tokens
                .iter()
                .filter(|t| {
                    let a = t.span.start as usize;
                    let b = t.span.end as usize;
                    a < start + line.len()
                        && b > start
                        && !matches!(
                            t.kind,
                            T::LParen
                                | T::RParen
                                | T::LBracket
                                | T::RBracket
                                | T::LBrace
                                | T::RBrace
                                | T::LtLt
                                | T::GtGt
                                | T::Quote
                                | T::Tilde
                                | T::DotDot
                        )
                })
                .count();
            let comment = lexed.comments.iter().any(|c| {
                c.span.start as usize >= start && (c.span.start as usize) < start + line.len()
            });
            assert!(
                atoms == 1 && !comment || atoms == 0 && comment,
                "splittable line of width {width}: {line:?}\n{out}"
            );
        }
        start += line.len() + 1;
    }
    out
}

#[test]
fn repository_surfaces_have_pretty_lisp_layout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    files(&root, &mut paths);
    paths.sort();
    let mut native = 0;
    let mut converted = 0;
    for path in paths {
        let relative = path.strip_prefix(&root).unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        if path.extension().unwrap() == "is" {
            eprintln!("Lisp {}", relative.display());
            check(&source);
            native += 1;
        } else if ["std", "lib", "examples", "tests/spec"]
            .iter()
            .any(|dir| relative.starts_with(dir))
        {
            let (module, errors) = ilang_syntax::parse_module(&source);
            if !errors.is_empty() {
                continue;
            } // Syntax-error fixtures have no core tree.
            let (lisp, _) = lisp::print_with_comments(&source, &module);
            eprintln!("Converted {}", relative.display());
            let out = check(&lisp);
            assert_eq!(
                shape::canon(&module),
                shape::canon(&lisp::parse_source(&out).0)
            );
            converted += 1;
        }
    }
    eprintln!("checked {native} Lisp files and {converted} converted normal files");
    assert!(
        native >= 3 && converted > 500,
        "{native} native, {converted} converted"
    );
}

#[test]
fn comments_stay_between_the_same_datums_and_blank_lines_collapse() {
    let source = "//// module\n/// first\n(= f (-> (x) _ {(+ x // operand\n 1) // statement\n})) // item\n\n\n// before next\n\n/// second\n(= g `(~ // prefix\n x ~ // splice\n ..[a b]))\n// end\n";
    let out = check(source);
    let tokens = |s: &str| {
        let lexed = lex_lisp(s);
        let mut pieces: Vec<_> = lexed
            .tokens
            .into_iter()
            .filter(|t| t.span.start != t.span.end)
            .map(|t| {
                (
                    t.span.start,
                    s[t.span.start as usize..t.span.end as usize].to_owned(),
                )
            })
            .collect();
        pieces.extend(lexed.comments.into_iter().map(|c| (c.span.start, c.text)));
        pieces.sort_by_key(|p| p.0);
        pieces.into_iter().map(|p| p.1).collect::<Vec<_>>()
    };
    assert_eq!(tokens(source), tokens(&out));
    assert!(!out.contains("\n\n\n"), "{out}");
    assert!(out.contains("\n\n// before next\n\n/// second\n"), "{out}");
    // A line comment before closing brackets moves them to the next line.
    assert!(out.contains("// statement\n     }))"), "{out}");
}

#[test]
fn heads_lists_blocks_and_prefixes() {
    assert_eq!(
        check("(= f (-> (x) _ {(+ x 1)}))\n"),
        "(= f (-> (x) _ {(+ x 1)}))\n"
    );
    let out = check(
        "(= f (-> (x) _ {(= a `[one two three four five six seven eight nine ten eleven twelve thirteen fourteen]) (| x {(-> 0 `(~..a)) (-> _ `(~x))})}))\n",
    );
    assert!(
        out.starts_with("(= f\n  (-> (x) _\n    {(= a\n       `[one\n         two\n"),
        "{out}"
    );
    assert!(
        out.ends_with("fourteen])\n     (| x {(-> 0 `(~..a)) (-> _ `(~x))})}))\n"),
        "{out}"
    );
    assert!(out.contains("`["));
    assert!(out.contains("~..a"));
    let datum = lisp::parse("`(long ~name ~..[one two])").unwrap();
    assert_eq!(lisp::show(&datum[0]), "`(long ~name ~..[one two])");
}

#[test]
fn lisp_convention_gathers_closers_and_aligns_arguments() {
    let out = check(
        "(= main (-> () _ {(print_all_of_these first_argument second_argument third_argument fourth)}))\n",
    );
    assert_eq!(
        out,
        "(= main\n  (-> () _\n    {(print_all_of_these first_argument\n                         second_argument\n                         third_argument\n                         fourth)}))\n"
    );
    let out = check("(= x ((. items add) first_argument_is_long second_argument_is_long third_argument_is_long))\n");
    assert_eq!(
        out,
        "(= x\n  ((. items add)\n   first_argument_is_long\n   second_argument_is_long\n   third_argument_is_long))\n"
    );
    for line in out.lines() {
        assert!(!line.trim_start().starts_with([')', ']', '}']) && !line.contains('\t'));
    }
}

#[test]
fn literals_and_long_comments_remain_indivisible() {
    let source = format!(
        "(= text \"{}\") // {}\n(= number 0x1234)\n",
        "字".repeat(100),
        "comment ".repeat(20)
    );
    let out = check(&source);
    assert!(out.contains(&format!("\"{}\"", "字".repeat(100))));
    assert!(out.contains("0x1234"));
}
