"""Prepare removal of old definition-table nodes, preserving oracle tests."""
from pathlib import Path
import importlib.util
import json
import re
import subprocess

ROOT = Path.cwd()
spec = importlib.util.spec_from_file_location('resolver_delete', ROOT / 'scripts/syn3-delete-resolver.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


def apply():
    proof = json.loads((ROOT / 'target/fa-night/step3-green.json').read_text())
    assert proof['commit'] == subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    base = ROOT / 'crates/ilang-syntax'
    path = base / 'src/lisp/definitions.rs'
    text = path.read_text()
    for name in ['source_definition', 'source_definition_from_syntax', 'expanded_definition_rows']:
        text = helper.remove_function(text, name)
    text += '''
/// Answer a written native type definition with fresh parameter identities.
pub fn source_definition(item: &crate::syn3::ast::Item, source: &str, scope: &Scope, answer: u64) -> Syntax {
    let span = item.span.0;
    let lisp = source[span.start as usize..span.end as usize].starts_with('(');
    let tree = super::source_tree::SourceTree::read(scope, source, lisp);
    let datum = tree.item_datum(span).expect("written definition has its tree item");
    tree_definition_from_datum(&datum, scope, answer)
}
pub fn source_definition_from_syntax(_item: &crate::syn3::ast::Item, syntax: &Syntax, answer: u64) -> Syntax {
    expanded_definition(syntax, answer).expect("a written type definition remains valid")
}
/// Definition answers use the final item rows. The obsolete flag remains only
/// until callers of the Rust helper finish their API migration.
pub fn expanded_definition_rows(item: &Syntax, answer: u64, _new_rows: bool) -> Result<Syntax, SyntaxError> {
    expanded_definition_new(item, answer)
}
'''
    assert 'crate::ast::' not in text
    path.write_text(text)
    # The old definition reader lives exclusively in the standalone oracle.
    private = base / 'examples/legacy/src/lisp'
    original = subprocess.check_output(['git', 'show', 'HEAD:crates/ilang-syntax/src/lisp/definitions.rs'], text=True)
    (private / 'definitions.rs').write_text(original)
    path = private / 'mod.rs'
    path.write_text(path.read_text() + '\npub mod definitions;\n')
    path = base / 'src/lisp/syntax.rs'
    text = path.read_text()
    for name in ['with_definition_head', 'with_node', 'replace_scope_identity']:
        text = text.replace('    pub(super) fn ' + name + '(',
            '    #[doc(hidden)]\n    pub fn ' + name + '(', 1)
    path.write_text(text)
    path = base / 'src/lisp/definition_table.rs'
    text = path.read_text().replace('use crate::ast::{Item, ItemKind};\n', '')
    text = text.replace('TypeQuery, builtin_definition, freshen_parameters, source_definition_from_syntax, type_query,',
        'TypeQuery, builtin_definition, freshen_parameters, type_query,')
    begin = text.index('    Written {')
    end = helper.item_end(text, begin)
    if text[end:end + 1] == ',':
        end += 1
    text = text[:begin] + text[end:]
    text = text.replace('    Event(super::SyntaxCharge),\n', '')
    text = text.replace('    lisp_document: Option<Vec<super::Datum>>,\n', '')
    text = text.replace('    source_encoder: Option<super::SourceEncoder>,\n', '')
    text = helper.remove_function(text, 'new', '    ')
    begin = text.index('impl Entry {') + len('impl Entry {')
    text = text[:begin] + '''
    fn new(definition: Definition) -> Self {
        if let Definition::Tree { item, .. } = &definition {
            assert!(matches!(item.kind, crate::syn3::ast::ItemKind::TypeDef(_)),
                "definition table accepts type definitions only");
        }
        Self { definition }
    }
''' + text[begin:]
    text = re.sub(r'Definition::Written\s*\{[^}]*\}\s*\|\s*', '', text)
    begin = text.index('            Definition::Written {')
    arrow = text.index('=>', begin)
    end = helper.item_end(text, arrow)
    if text[end:end + 1] == ',':
        end += 1
    text = text[:begin] + text[end:]
    assert 'Definition::Written' not in text
    assert 'SourceEncoder' not in text and 'SyntaxCharge' not in text
    path.write_text(text)
    path = ROOT / 'crates/ilang-host/src/syntax.rs'
    text = path.read_text()
    begin = text.index('        ilang_syntax::lisp::definition_table::DefinitionCost::Event(event)')
    arrow = text.index('=>', begin)
    end = helper.item_end(text, arrow)
    if text[end:end + 1] == ',':
        end += 1
    path.write_text(text[:begin] + text[end:])


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; --apply requires the exact green step-three proof.')
