"""Keep old-tree assertions as adapters over the real native reader/validator."""
from pathlib import Path
import json
import subprocess

ROOT = Path.cwd()


def apply():
    proof = json.loads((ROOT / 'target/fa-night/step3-green.json').read_text())
    assert proof['commit'] == subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    syntax = ROOT / 'crates/ilang-syntax'
    manifest = syntax / 'Cargo.toml'
    manifest.write_text(manifest.read_text() + '\nmigration-syntax = { package = "ilang-syntax-migration", path = "examples/legacy" }\n')
    tests = syntax / 'tests'
    adapter = r'''//! Old node assertions over the production native parser and validation.
#![allow(dead_code)]
#[allow(unused_imports)]
pub use migration_syntax::*;
extern crate ilang_syntax as native;

pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (module, mut errors) = native::parse_module(source);
    let (old, old_errors) = migration_syntax::syn3::lower::lower(&module);
    let (_, validation) = native::syn3::facts::Transition::capture_module(&module, false);
    let records = |ds: &[Diagnostic]| ds.iter().map(|d| (d.code, d.span, d.message.clone())).collect::<Vec<_>>();
    assert_eq!(records(&validation), records(&old_errors), "native validation of {source:?}");
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (old, errors)
}
'''
    for path in tests.glob('*.rs'):
        text = path.read_text()
        if path.name == 'syn3.rs':
            text = text.replace('use ilang_syntax::lexer::{lex, lex3};', 'use ::ilang_syntax::lexer::{lex, lex3};')
        if path.name == 'common.rs':
            path.write_text(text.replace('ilang_syntax::', 'migration_syntax::'))
            continue
        text = text.replace('#![cfg(feature = "legacy")]\n', '')
        text = text.replace('#[path = "../examples/support/legacy/mod.rs"]\nmod legacy;', 'use migration_syntax::legacy;')
        # Inner docs/attributes precede the adapter declaration.
        lines = text.splitlines(keepends=True)
        at = 0
        while at < len(lines) and (lines[at].lstrip().startswith(('//!', '#![')) or not lines[at].strip()):
            at += 1
        lines[at:at] = ['#[path = "oracle.rs"]\nmod ilang_syntax;\n']
        path.write_text(''.join(lines))
    (tests / 'oracle.rs').write_text(adapter)
    # Keep the old `Written` test intent using a native stored source item.
    path = tests / 'lisp_syntax.rs'
    text = path.read_text()
    while 'Definition::Written {' in text:
        begin = text.index('Definition::Written {')
        import importlib.util
        spec = importlib.util.spec_from_file_location('resolver_delete', ROOT / 'scripts/syn3-delete-resolver.py')
        helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(helper)
        end = helper.item_end(text, begin)
        block = text[begin:end]
        # Reuse every existing source/scope/readiness expression; only the
        # representation of its stored item changes.
        import re
        item = re.search(r'item:\s*Box::new\((.*?)\),\n', block).group(1)
        source = re.search(r'source:\s*(.*?),\n', block).group(1)
        scope = re.search(r'scope:\s*(.*?),\n', block).group(1)
        ready = re.search(r'ready:\s*(true|false)', block).group(1)
        replacement = f'native_written_definition(&{item}, {source}, {scope}, {ready})'
        text = text[:begin] + replacement + text[end:]
    text += '''
// Stored definitions use the final rows while the assertion oracle keeps
// the same old item span for its existing source fixtures.
fn native_written_definition(
    item: &migration_syntax::ast::Item, source: String,
    scope: ilang_syntax::lisp::syntax::Scope, ready: bool,
) -> ilang_syntax::lisp::definition_table::Definition {
    let lisp = source[item.span.start as usize..item.span.end as usize].starts_with('(');
    let (module, errors) = if lisp {
        ilang_syntax::syn3::lisp::from_lisp(&source)
    } else { ilang_syntax::syn3::parse_module(&source) };
    assert!(errors.is_empty(), "{errors:?}");
    let native = module.items.into_iter().find(|i| i.span.0 == item.span).expect("same written item");
    let tree = ilang_syntax::lisp::source_tree::SourceTree::read(&scope, &source, lisp);
    let datum = tree.item_datum(item.span).expect("same written datum");
    ilang_syntax::lisp::definition_table::Definition::tree_with_datum(native, datum, scope, ready)
}
'''
    path.write_text(text)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; --apply requires the exact green step-three proof.')
