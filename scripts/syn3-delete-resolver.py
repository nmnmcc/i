"""Step-four preparation. Apply only after the green step-three checkpoint."""
from pathlib import Path
import re
import subprocess

ROOT = Path.cwd()


def snapshot(path):
    return subprocess.check_output(['git', 'show', 'HEAD:' + path], text=True)


def item_end(text, start):
    """Find an item's closing brace, ignoring Rust strings and comments."""
    i = start
    level = 0
    opened = False
    while i < len(text):
        if text.startswith('//', i):
            end = text.find('\n', i)
            i = len(text) if end < 0 else end + 1
            continue
        if text.startswith('/*', i):
            nesting = 1
            i += 2
            while nesting:
                if text.startswith('/*', i):
                    nesting += 1
                    i += 2
                elif text.startswith('*/', i):
                    nesting -= 1
                    i += 2
                else:
                    i += 1
            continue
        raw = re.match(r'r(#+)?"', text[i:])
        if raw:
            delimiter = '"' + (raw.group(1) or '')
            i = text.index(delimiter, i + len(raw.group())) + len(delimiter)
            continue
        char = re.match(r"'(?:\\u\{[0-9a-fA-F]+\}|\\.|[^'\\])'", text[i:])
        if char:
            i += len(char.group())
            continue
        if text[i] == '"':
            i += 1
            while text[i] != '"':
                i += 2 if text[i] == '\\' else 1
            i += 1
            continue
        if text[i] == '{':
            opened = True
            level += 1
        elif text[i] == '}':
            level -= 1
            if opened and level == 0:
                return i + 1
        i += 1
    raise ValueError('Unterminated item at ' + str(start))


def remove_function(text, name, indent=''):
    pattern = re.compile(r'^' + indent + r'(?:pub(?:\([^)]*\))? )?fn ' + name + r'\b', re.M)
    match = pattern.search(text)
    if match is None:
        raise ValueError('Missing function ' + name)
    begin = match.start()
    # Remove the item's own docs/attributes too; an empty impl must parse.
    while begin:
        previous = text.rfind('\n', 0, begin - 1) + 1
        line = text[previous:begin].strip()
        if not line or line.startswith('///') or line.startswith('#['):
            begin = previous
        else:
            break
    end = item_end(text, match.start())
    return text[:begin] + text[end:]


def make_legacy(source):
    source = source.replace('type Parsed = crate::LoweredParsed;', 'type Parsed = BaselineParsed;')
    begin = source.index('mod three;')
    end = source.index('fn resolve_lowered(', begin)
    source = source[:begin] + source[end:]
    begin = source.index('#[derive(Clone, Copy, Debug, PartialEq, Eq)]\nenum Entry')
    end = source.index("struct Resolver<'a>", begin)
    source = source[:begin] + source[end:]
    begin = source.index('#[derive(Clone, Debug)]\npub(crate) struct ExpansionScope')
    source = source[:begin]
    source = source.replace('use ilang_syntax::ast;', 'use migration_syntax::ast;')
    source = source.replace('ilang_syntax::walk::', 'migration_syntax::walk::')
    source = source.replace('ilang_syntax::syn3::lower::', 'migration_syntax::syn3::lower::')
    source = source.replace('ilang_syntax::lisp::parse_pattern', 'migration_syntax::lisp::parse_pattern')
    source += '''
#[derive(Clone)]
struct BaselineParsed {
    package: PackageId,
    path: String,
    source: ilang_package::SourceId,
    kind: ModuleKind,
    ast: ast::Module,
    scoped: Option<ilang_syntax::lisp::scoped::ScopedSource>,
    compiler_generated: bool,
    typed_macros: Option<std::sync::Arc<dyn crate::typed_macro::Expander>>,
}
pub(super) fn resolve_transition(
    graph: &PackageGraph,
    parsed: &[crate::Parsed],
    modules: &[ast::Module],
) -> (Program, Vec<Diagnostic>) {
    ilang_syntax::syn3::deep::deep(|| {
        let parsed: Vec<_> = parsed.iter().zip(modules).map(|(p, m)| BaselineParsed {
            package: p.package, path: p.path.clone(), source: p.source, kind: p.kind,
            ast: m.clone(), scoped: p.scoped.clone(),
            compiler_generated: p.compiler_generated, typed_macros: p.typed_macros.clone(),
        }).collect();
        resolve_lowered(graph, &parsed)
    })
}
'''
    # Scope and expansion tables belong to HIR; reuse their production types.
    at = source.index('type Parsed =')
    source = source[:at] + 'use super::*;\n' + source[at:]
    return source


def native_core(source):
    source = source.replace('type Parsed = crate::LoweredParsed;', 'type Parsed = crate::Parsed;')
    for name in ['resolve_transition', 'resolve_lowered', 'multi_groups', 'ast_binders',
                 'label_of', 'ungroup', 'is_constructor', 'same_name_part',
                 'typed_output_via_lowering', 'typed_output_lowered']:
        source = remove_function(source, name)
    for name in ['collect', 'collect_pattern_binders', 'collect_type', 'attributes',
                 'entry_form', 'imports', 'import_circles', 'type_defs', 'early_types',
                 'host_form', 'resolve_alias_pattern', 'type_pattern', 'ty', 'values',
                 'binding', 'is_direct', 'expr', 'alias_target', 'selection_tables',
                 'piped_table', 'group_call', 'clause_function', 'format', 'block',
                 'function', 'clause', 'resolved_quote', 'pattern']:
        source = remove_function(source, name, '    ')
    source = source.replace('use ilang_syntax::ast;', '')
    source = source.replace('ast::ImportItem', 'SourceImport').replace('ast::ExportItem', 'SourceExport')
    source = source.replace('ast::Ident', 'Ident')
    source = source.replace('ast::LitValue', 'ilang_syntax::syn3::ast::LitValue')
    assert not re.search(r'(?<!:)\bast::', source), [line for line in source.splitlines() if re.search(r'(?<!:)\bast::', line)]
    metadata = '''    let metadata: Vec<_> = parsed
        .iter()
        .map(|p| p.with_lowered_ast(Default::default()))
        .collect();
'''
    assert source.count(metadata) == 2
    source = source.replace(metadata, '').replace('&metadata', 'parsed')
    at = source.index('mod three;')
    declarations = '''
/// Names and spans needed by shared import/export rules; no syntax nodes.
struct SourceImport { alias: Option<Ident>, names: Vec<Ident>, path: Vec<Ident>, span: Span }
struct SourceExport { name: Ident, constructors: bool, span: Span }
#[cfg(test)]
#[allow(dead_code)]
mod legacy;
#[cfg(test)]
mod differential;
'''
    return source[:at] + declarations + source[at:]


def apply():
    import json
    proof = json.loads((ROOT / 'target/fa-night/step3-green.json').read_text())
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    assert proof['commit'] == head, 'Require the exact green step-three commit'
    branch = subprocess.check_output(['git', 'branch', '--show-current'], text=True).strip()
    assert branch.startswith('lane/') and branch.endswith('-delete')
    path = ROOT / 'crates/ilang-check/src/resolve.rs'
    source = snapshot('crates/ilang-check/src/resolve.rs')
    directory = path.parent / 'resolve'
    (directory / 'legacy.rs').write_text(make_legacy(source))
    path.write_text(native_core(source))
    path = directory / 'three.rs'
    text = path.read_text().replace('use ilang_syntax::ast as o;', 'use ilang_syntax::syn3::ast::resolved as o;')
    text = text.replace('o::ImportItem', 'SourceImport').replace('o::ExportItem', 'SourceExport')
    path.write_text(text)
    path = ROOT / 'crates/ilang-check/src/lib.rs'
    text = path.read_text()
    begin = text.index('#[derive(Clone)]\npub(crate) struct LoweredParsed')
    end = text.index('impl Clone for Parsed', begin)
    text = text[:begin] + text[end:]
    begin = text.index('impl Parsed {')
    end = text.index('/// Parses every module', begin)
    text = text[:begin] + text[end:]
    path.write_text(text)
    path = ROOT / 'crates/ilang-check/src/query/tree.rs'
    path.write_text(path.read_text().replace('syn3::lower::', 'migration_syntax::syn3::lower::'))
    path = ROOT / 'crates/ilang-check/tests/resolve3_differential.rs'
    text = path.read_text().replace('use ilang_syntax::syn3;', 'use migration_syntax::syn3;\nuse crate as ilang_check;')
    text = text.replace('resolve::resolve_transition(', 'super::legacy::resolve_transition(')
    text = text.replace('ilang_syntax::lisp::quote_lower::', 'migration_syntax::lisp::quote_lower::')
    (directory / 'differential.rs').write_text(text)
    path.unlink()


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; --apply requires target/fa-night/step3-green.json and lane/*-delete.')
