"""Prepare native source normalization and retain old-reader test adapters."""
from pathlib import Path
import importlib.util
import json
import subprocess

ROOT = Path.cwd()
spec = importlib.util.spec_from_file_location('resolver_delete', ROOT / 'scripts/syn3-delete-resolver.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


def apply():
    proof = json.loads((ROOT / 'target/fa-night/step3-green.json').read_text())
    assert proof['commit'] == subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    path = ROOT / 'crates/ilang-syntax/src/lisp/source_syntax.rs'
    source = path.read_text()
    prefix = source[:source.index('fn symbol(')]
    prefix = prefix.replace('Node, ROWS3, Scope, Syntax', 'Node, Scope, Syntax')
    prefix = prefix.replace('use crate::ast::{Expr, Item, Pattern, TypeExpr};\n', '')
    prefix = prefix.replace('datum.is_none() && ROWS3', 'datum.is_none()')
    start = prefix.index('    if ROWS3 {')
    end = helper.item_end(prefix, start)
    # The old else branch is part of the same function; keep only native rows.
    assert prefix[end:].lstrip().startswith('else')
    other = prefix.index('else', end)
    finish = helper.item_end(prefix, other)
    prefix = prefix[:start] + '    Syntax::from_datum(datum, scope)\n' + prefix[finish:]
    start = source.index('fn symbol(')
    symbol = source[start:helper.item_end(source, start)]
    start = source.index('// ---- The rows of syntax three')
    end = source.index('fn item(', start)
    native = source[start:end]
    tests = source[source.index('#[cfg(test)]'):]
    tests = tests.replace('    use super::*;', '''    use super::*;
    use crate::lisp::syntax::ROWS3;
    use super::{normalize_expression as expr, normalize_type as ty};
    use migration_syntax::lisp::source_syntax::{
        source_expression_syntax, source_item_syntax, source_pattern_syntax, source_type_syntax,
    };''', 1)
    tests = tests.replace('super::super::', 'migration_syntax::lisp::')
    path.write_text(prefix + symbol + '\n\n' + native + tests)
    # Private adapters retain old AST arguments while delegating actual native
    # normalization to the production functions exercised by these tests.
    private = ROOT / 'crates/ilang-syntax/examples/legacy/src/lisp/source_syntax.rs'
    text = private.read_text()
    for kind, name in [('Type', 'type'), ('Pattern', 'pattern'), ('Expr', 'expression'), ('Item', 'item')]:
        text = text.replace('normalize3(s, Mode3::' + kind + ')',
            'syntax3::lisp::source_syntax::normalize_' + name + '(s)')
    private.write_text(text)
    path = ROOT / 'crates/ilang-syntax/src/lisp/syntax.rs'
    text = path.read_text().replace('    pub(super) fn written(',
        '    #[doc(hidden)]\n    pub fn written(', 1)
    path.write_text(text)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; --apply requires the exact green step-three proof.')
