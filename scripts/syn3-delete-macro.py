"""Prepare removal of the inactive old macro hooks after step three is green."""
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
    base = ROOT / 'crates/ilang-macro'
    old_walk = (base / 'src/body_walk.rs').read_text()
    start = old_walk.index('#[derive(Clone, Debug)]\npub struct Local')
    end = old_walk.index('\npub trait Hook', start)
    local = old_walk[start:end]
    path = base / 'src/body_walk3.rs'
    path.write_text(path.read_text().replace('pub use crate::body_walk::Local;', local))
    (base / 'src/body_walk.rs').write_text('//! Native body traversal.\npub use crate::body_walk3::*;\n')
    (base / 'src/call.rs').write_text('//! Native macro call candidates.\npub use crate::call3::*;\n')
    path = base / 'src/body_engine.rs'
    text = path.read_text()
    for name in ['atom', 'resolve', 'expr_syntax', 'ty_syntax']:
        text = helper.remove_function(text, name, '    ')
    begin = text.index("impl Hook for BodyEngine<'_>")
    end = helper.item_end(text, begin)
    text = text[:begin] + text[end:]
    text = text.replace('    body_walk::{Hook, Local},\n    call,', '    body_walk3::Local,')
    text = text.replace('use ilang_syntax::ast::{self, Expr, ExprKind, Ident, Stmt, TypeExpr};\n', '')
    text = text.replace('    source_syntax::{source_expression_syntax, source_type_syntax},\n', '')
    text = text.replace('    Datum, DatumKind,\n', '')
    path.write_text(text)
    manifest = base / 'Cargo.toml'
    manifest.write_text(manifest.read_text() + '\n[dev-dependencies]\nmigration-syntax = { package = "ilang-syntax-migration", path = "../ilang-syntax/examples/legacy" }\n')
    path = base / 'tests/body_walk.rs'
    text = path.read_text().replace('use ilang_macro::body_walk::{self, Hook, Local};',
        'use migration_syntax as ilang_syntax;\nuse migration_syntax::macro_body_walk::{self as body_walk, Hook, Local};')
    path.write_text(text)
    path = base / 'tests/body_walk3.rs'
    text = path.read_text().replace('use ilang_macro::{body_engine3, body_walk, call, call3};',
        'use ilang_macro::{body_engine3, call3};\nuse migration_syntax::{macro_body_walk as body_walk, macro_call as call};')
    text = text.replace('ilang_syntax::ast::', 'migration_syntax::ast::')
    text = text.replace('ilang_syntax::syn3::lower::', 'migration_syntax::syn3::lower::')
    begin = text.index('impl body_walk::Hook for Old')
    end = helper.item_end(text, begin)
    old = text[begin:end].replace('&[Local]', '&[body_walk::Local]').replace('visible(', 'visible_old(')
    text = text[:begin] + old + text[end:]
    text += '''
// The private oracle owns its walker locals. Compare the same source facts.
fn visible_old(locals: &[body_walk::Local], depth: u32) -> Vec<(String, (u32, u32))> {
    let locals: Vec<_> = locals.iter().map(|l| Local {
        name: l.name.clone(), span: l.span, fn_only: l.fn_only,
    }).collect();
    visible(&locals, depth)
}
'''
    path.write_text(text)
    for name in ['registry3.rs', 'definition_world3.rs']:
        path = base / 'src' / name
        text = path.read_text().replace('ilang_syntax::ast', 'migration_syntax::ast')
        text = text.replace('ilang_syntax::syn3::lower::', 'migration_syntax::syn3::lower::')
        text = text.replace('crate::body_walk::for_each_binder', 'migration_syntax::macro_body_walk::for_each_binder')
        path.write_text(text)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    if parser.parse_args().apply:
        apply()
    else:
        print('Preparation only; requires the exact green step-three proof before --apply.')
