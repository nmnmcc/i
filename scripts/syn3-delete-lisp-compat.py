"""Retain native expansion/scoped APIs and move old test readers to oracle."""
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
    base = ROOT / 'crates/ilang-syntax'
    path = base / 'src/lisp/expansion.rs'
    original = path.read_text()
    private = base / 'examples/legacy/src/lisp'
    # Share MacroDefinition/MacroArgument identities; only the old reader is private.
    begin = original.index('pub fn definitions(')
    end = helper.item_end(original, begin)
    (private / 'expansion.rs').write_text('use super::syntax::SyntaxError;\npub use syntax3::lisp::expansion::{MacroDefinition, MacroArgument, ExpansionSession};\n' + original[begin:end] + '\n')
    path.write_text(helper.remove_function(original, 'definitions', '') + '\npub use super::expansion3::definitions;\n')
    path = private / 'mod.rs'
    path.write_text(path.read_text() + '\npub mod expansion;\n')
    path = base / 'src/lisp/expansion3.rs'
    text = path.read_text().replace('crate::syn3::lower::', 'migration_syntax::syn3::lower::')
    text = text.replace('super::super::expansion::definitions(&lowered, lisp)', 'migration_syntax::lisp::expansion::definitions(&lowered, lisp)')
    path.write_text(text)
    (base / 'src/lisp/quote_lower.rs').write_text('//! Native quote planning.\npub use super::quote_lower3::*;\n')
    # Retain the old public Rust reader solely for unchanged old-node test
    # assertions. Its native readers still use the real production row parser.
    scoped = (base / 'src/lisp/scoped.rs').read_text()
    scoped = scoped.replace('crate::syn3::facts::Transition', 'crate::syn3::lower::Transition')
    for kind, lower, record in [('expression', 'expr', 'expression'), ('pattern', 'pattern', 'pattern')]:
        before = '        self.transition\n            .capture_' + kind + '(&tree)\n            .map_err(|error| self.error(error))?;'
        after = ('        let mut old = crate::syn3::lower::lower_' + lower + '(&tree)\n'
            '            .map_err(|error| self.error(error))?;\n'
            '        self.transition.record_' + record + '(&mut old);\n'
            '        self.transition.' + kind + '(tree.span.0);')
        assert before in scoped
        scoped = scoped.replace(before, after)
    before = '        self.transition\n            .capture_items(&tree)\n            .map_err(|error| self.error(error))?;'
    after = ('        let mut old = crate::syn3::lower::lower_items(&tree)\n'
        '            .map_err(|error| self.error(error))?;\n'
        '        self.transition.record_items(&mut old);')
    assert before in scoped
    scoped = scoped.replace(before, after)
    scoped = scoped.replace('crate::syn3::lower::Transition::validate_type',
        'syntax3::syn3::facts::Transition::validate_type')
    (private / 'scoped.rs').write_text(scoped)
    path = private / 'mod.rs'
    path.write_text(path.read_text() + '\npub mod scoped;\n')
    path = base / 'src/lisp/scoped.rs'
    text = path.read_text().replace('use crate::ast::{Expr, Item, Pattern, TypeExpr};\n', '')
    for name in ['expression_metered', 'pattern_metered', 'ty_metered', 'items_metered', 'expression', 'pattern', 'ty', 'items']:
        text = helper.remove_function(text, name, '    ')
    # Compatibility method names denote the one native tree after deletion.
    offset = text.index('\n}\n\n#[cfg(test)]')
    text = text[:offset] + '''
    pub fn expression(&mut self, syntax: &Syntax, call_site: &Syntax) -> Result<crate::syn3::ast::Expr, ScopedError> {
        self.expression3(syntax, call_site)
    }
''' + text[offset:]
    # Existing origin assertions use the same byte ranges carried by the new span wrapper.
    begin = text.index('#[cfg(test)]')
    tests = text[begin:]
    tests = tests.replace('first.span', 'first.span.0').replace('second.span', 'second.span.0')
    text = text[:begin] + tests
    path.write_text(text)

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; --apply requires the exact green step-three proof.')
