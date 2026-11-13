#!/usr/bin/env python3
"""Apply step 4 on a lane/*-delete branch after the green resolver gate.

The migration tools retain a standalone private library under examples/legacy.
No build or test is run here: run every command through the heavy wrapper.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path.cwd().resolve()  # Run this staged preparation only from the lane root.
SYNTAX = ROOT / 'crates/ilang-syntax'
# Last shared-value move, before the old consumers change. The migration tools
# need this old implementation even when steps 2c/2d already removed its files.
LEGACY_REF = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
proof_path = ROOT / 'target/fa-night/step3-green.json'
ORACLE_REF = json.loads(proof_path.read_text()).get('oracle_ref', LEGACY_REF) if proof_path.exists() else LEGACY_REF


def old(path):
    return subprocess.check_output(['git', 'show', f'{ORACLE_REF}:crates/ilang-syntax/{path}'], cwd=ROOT, text=True)


def put(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def edit(path, before, after):
    text = path.read_text()
    if before not in text:
        raise RuntimeError(f'{path.relative_to(ROOT)}: expected integration boundary not found: {before!r}')
    path.write_text(text.replace(before, after, 1))


def private_migrator(move_examples=True):
    dest = SYNTAX / 'examples/legacy'
    for name in ['ast.rs', 'ast_eq.rs', 'ast_clone.rs', 'walk.rs', 'print.rs', 'shape.rs',
                 'legacy/parser.rs',
                 'syn3/convert.rs', 'syn3/lower.rs', 'lisp/encode.rs', 'lisp/encode_charge.rs',
                 'lisp/source_syntax.rs', 'lisp/quote_lower.rs', 'lisp/to_ast.rs',
                 'lisp/from_ast.rs', 'lisp/comments.rs', 'lisp/trampoline.rs']:
        put(dest / 'src' / name, old('src/' + name).replace('#[cfg(feature = "legacy")]\n', ''))
    # The green step-3 lowerer shares native metadata. Its private oracle
    # must own that container, so its inherent old-tree recording impl stays local.
    lower_path = dest / 'src/syn3/lower.rs'
    lower = lower_path.read_text()
    if 'pub use super::facts::Transition;' in lower:
        facts = old('src/syn3/facts.rs')
        start = facts.index('#[derive(Clone, Debug, Default)]')
        end = facts.index('impl Transition {', start)
        declaration = facts[start:end].replace('pub struct Transition {',
            'pub struct Transition {\n    native: Option<syntax3::syn3::facts::Transition>,')
        methods = []
        for name in ['extend', 'expression', 'ty', 'pattern', 'is_expression',
                     'is_type', 'is_pattern', 'is_complete', 'formals', 'sum_members']:
            begin = facts.index('    pub fn ' + name + '(')
            brace = facts.index('{', begin)
            count = 1
            finish = brace + 1
            while count:
                count += (facts[finish] == '{') - (facts[finish] == '}')
                finish += 1
            method = facts[begin:finish]
            if name == 'extend':
                method = method.replace('        self.complete |= other.complete;', '''        if let Some(native) = &other.native {
            self.native.get_or_insert_with(Default::default).extend(native);
        }
        self.complete |= other.complete;''')
            elif name in ['is_expression', 'is_type', 'is_pattern']:
                field = {'is_expression':'expressions', 'is_type':'types', 'is_pattern':'patterns'}[name]
                method = method.replace('self.' + field + '.contains(&span)',
                    'self.' + field + '.contains(&span) || self.native.as_ref().is_some_and(|n| n.' + name + '(span))')
            elif name == 'is_complete':
                method = method.replace('self.complete',
                    'self.complete || self.native.as_ref().is_some_and(|n| n.is_complete())')
            elif name == 'formals':
                method = method.replace('self.parameters.get(&(binding, pattern)).map(Vec::as_slice)',
                    'self.parameters.get(&(binding, pattern)).map(Vec::as_slice).or_else(|| self.native.as_ref().and_then(|n| n.formals(binding, pattern)))')
            elif name == 'sum_members':
                method = method.replace('self.sums.get(&parent).map(Vec::as_slice)',
                    'self.sums.get(&parent).map(Vec::as_slice).or_else(|| self.native.as_ref().and_then(|n| n.sum_members(parent)))')
            methods.append(method)
        methods.append('''    /// Read-only native facts remain shared; old recording deltas stay private.
    pub fn from_native(native: &syntax3::syn3::facts::Transition) -> Self {
        Self { native: Some(native.clone()), ..Self::default() }
    }''')
        replacement = declaration + 'impl Transition {\n' + '\n'.join(methods) + '\n}\n'
        lower = lower.replace('pub use super::facts::Transition;', replacement)
        for field, accessor in [('expressions','is_expression'), ('types','is_type'), ('patterns','is_pattern')]:
            lower = lower.replace('self.transition.' + field + '.contains(&span)',
                'self.transition.' + accessor + '(span)')
        lower = lower.replace('self.transition.complete', 'self.transition.is_complete()')
        lower = lower.replace('self.transition.sums.get(&item.span.0)', 'self.transition.sum_members(item.span.0)')
        lower = lower.replace('''let original = self
            .transition
            .parameters
            .get(&(b.span.0, b.pattern.span.0))
            .cloned();''', '''let original = self.transition.formals(b.span.0, b.pattern.span.0).map(|p| p.to_vec());''')
        lower_path.write_text(lower)
    for source, name in [('body_walk.rs', 'macro_body_walk.rs'), ('call.rs', 'macro_call.rs')]:
        code = subprocess.check_output(['git', 'show', f'{LEGACY_REF}:crates/ilang-macro/src/{source}'], cwd=ROOT, text=True)
        put(dest / 'src' / name, code.replace('ilang_syntax::', 'crate::'))
    lexer = old('src/lexer.rs')
    put(dest / 'src/lexer.rs', """//! Private legacy lexer; production token and comment types stay shared.
pub use syntax3::lexer::{Comment, DocComment, Lexed};
use crate::{Code, Diagnostic};
use crate::span::Span;
use crate::token::{StringPart, Token, TokenKind as T};
use num_bigint::BigUint;
""" + lexer[lexer.index('pub fn lex('):])
    # Stage 12 already moved the historical lexer/parser behind migration
    # tools. Keep that boundary; never resurrect its deleted formatter.
    put(dest / 'src/legacy/mod.rs', """pub mod parser;
pub mod lexer { pub use crate::lexer::*; }
pub mod lisp;
""")
    support = SYNTAX / 'examples/support/legacy'
    if (support / 'lisp.rs').exists():
        put(dest / 'src/legacy/lisp.rs', (support / 'lisp.rs').read_text().replace('ilang_syntax::', 'crate::'))
    else:
        put(dest / 'src/legacy/lisp.rs', "pub use crate::lisp::{parse_source_legacy as parse_source};\n")
    put(dest / 'src/lisp/render.rs', old('src/lisp/render.rs').replace('pub(super)', 'pub'))
    # Both representations use the production Datum and span/token types. Only
    # the old tree and readers are private, so conversion returns the real new AST.
    datum_reader = old('src/lisp/mod.rs')
    reader = datum_reader[datum_reader.index('fn parse_lexed('):datum_reader.index('/// Parse a Lisp-surface source')]
    reader += datum_reader[datum_reader.index('struct Parser'):datum_reader.index('/// Print a datum')]
    helpers = old('src/lisp/to_source.rs')
    ends = helpers[helpers.index('pub(super) fn ends_in_type_test'):helpers.index('pub(super) fn is_match')]
    put(dest / 'src/lisp/to_source.rs', '''use super::{Datum, DatumKind as K};
fn round(d: &Datum) -> Option<&[Datum]> { match &d.kind { K::Round(xs) => Some(xs), _ => None } }
fn symbol(d: &Datum, s: &str) -> bool { matches!(&d.kind, K::Symbol(x) if x == s) }
''' + ends)
    put(dest / 'src/lisp/mod.rs', '''use crate::{Code, Diagnostic};
use crate::token::{Token, TokenKind as T};
pub use syntax3::lisp::*;
pub mod encode;
pub mod encode_charge;
pub mod source_syntax;
pub mod quote_lower;
pub mod comments;
pub use encode::{SourceEncoder, expression_syntax, item_syntax, pattern_syntax, type_syntax};
pub use encode_charge::SyntaxCharge;
mod to_ast;
mod from_ast;
mod to_source;
pub(crate) mod trampoline;
pub use to_ast::{parse_expression, parse_item, parse_items, parse_module, parse_pattern, parse_type};
pub use from_ast::{print_module, print_with_comments};
pub use to_source::{normal_source, normal_with_comments};
pub fn parse_legacy(source: &str) -> Result<Document, Vec<Diagnostic>> { parse_lexed(crate::lexer::lex_lisp(source)) }
pub fn parse_source_legacy(source: &str) -> (crate::ast::Module, Vec<Diagnostic>) {
    match parse_legacy(source).and_then(|d| parse_module(&d).map_err(|e| vec![e])) {
        Ok(m) => (m, Vec::new()), Err(e) => (Default::default(), e)
    }
}
pub fn parse_source(source: &str) -> (crate::ast::Module, Vec<Diagnostic>) {
    let (tree, mut errors) = crate::syn3::lisp::from_lisp(source);
    let (tree, more) = crate::syn3::lower::lower_lisp(&tree);
    errors.extend(more); errors.sort_by_key(|d| (d.span.start, d.span.end)); (tree, errors)
}
''' + reader + datum_reader[datum_reader.index('fn float(n:'):])
    rows = old('src/syn3/lisp.rs')
    quote_helpers = rows[rows.index("fn round<'a>"):rows.index('fn lower(d:')]
    quote_helpers += rows[rows.index('fn sym(s:'):]
    put(dest / 'src/syn3/lisp.rs', '''pub use syntax3::syn3::lisp::*;
use crate::lisp::{Datum, DatumKind as K};
use crate::span::Span;
''' + quote_helpers)
    put(dest / 'src/lisp/to_source.rs', old('src/lisp/to_source.rs').replace('pub(super)', 'pub'))
    private_mod = dest / 'src/lisp/mod.rs'
    private_mod.write_text(private_mod.read_text().replace('mod to_source;', 'pub mod to_source;\npub mod render;'))
    old_front = old('src/syn3/mod.rs')
    private_front = old_front[old_front.index('/// Reads a normal-surface file'):old_front.index('/// Temporary integration switch')]
    put(dest / 'src/syn3/mod.rs', '''pub use syntax3::syn3::*;
pub mod lisp;
pub mod convert;
pub mod lower;
''' + private_front)
    put(dest / 'src/lib.rs', '''//! Private old syntax for migration tools; never linked into the compiler.
pub use syntax3::{Code, Diagnostic, span, token, format, old_syntax_hint, lex};
pub mod macro_body_walk;
pub mod macro_call;
pub mod lexer;
pub mod ast;
mod ast_clone;
mod ast_eq;
pub mod walk;
pub mod print;
pub mod shape;
pub mod legacy;
pub mod lisp;
pub mod syn3;
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (tree, mut errors) = syn3::parse_module(source);
    let (tree, more) = syn3::lower::lower(&tree);
    errors.extend(more); errors.sort_by_key(|d| (d.span.start, d.span.end)); (tree, errors)
}
''')
    if (SYNTAX / 'examples/support').exists():
        shutil.copytree(SYNTAX / 'examples/support', dest / 'support', dirs_exist_ok=True)
    for name in ['migrate-syntax3', 'migrate-syntax3-pipe', 'migrate-tests']:
        src = SYNTAX / 'examples' / (name + '.rs')
        code = src.read_text()
        if name == 'migrate-syntax3-pipe':
            code = code.replace('cargo run -p ilang-syntax --example migrate-syntax3-pipe',
                'cargo run --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --bin migrate-syntax3-pipe')
            code = code.replace('path == "crates/ilang-syntax/examples/migrate-syntax3-pipe.rs"',
                'path.starts_with("crates/ilang-syntax/examples/legacy/")')
        put(dest / (name + '.rs'), code)
        if move_examples:
            src.unlink()
    put(dest / 'Cargo.toml', '''[package]
name = "ilang-syntax-migration"
version = "0.1.0"
edition = "2024"
publish = false

[workspace]
resolver = "3"

[lib]
name = "ilang_syntax"

[dependencies]
syntax3 = { package = "ilang-syntax", path = "../.." }
num-bigint = "0.5.1"
serde_json = "1"

[[bin]]
name = "migrate-syntax3"
path = "migrate-syntax3.rs"
[[bin]]
name = "migrate-syntax3-pipe"
path = "migrate-syntax3-pipe.rs"
[[bin]]
name = "migrate-tests"
path = "migrate-tests.rs"
''')
    lock = ROOT / 'scripts/syn3-migration.Cargo.lock'
    if lock.is_file():
        put(dest / 'Cargo.lock', lock.read_text())
    put(dest / 'README.md', '''Private old tree, readers and converter for migration tools. This is a
standalone Cargo package, outside the compiler workspace. Build with the shared
heavy wrapper and `cargo check --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --all-targets`.
The compiler has no production dependency on this package; its query index test uses it as an oracle. The migration implementation is
preserved from the green step-three oracle recorded by the deletion driver. Production Datum, span and token types remain shared.
''')


def update_migration_commands():
    path = ROOT / 'scripts/syn3-convert-all'
    text = path.read_text().replace(
        'cargo build --release --locked -p ilang-syntax --features legacy --example migrate-syntax3',
        'cargo build --release --locked --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --bin migrate-syntax3 --target-dir "$repo/target"')
    text = text.replace(
        'cargo build --release --locked -p ilang-syntax --example migrate-syntax3',
        'cargo build --release --locked --manifest-path crates/ilang-syntax/examples/legacy/Cargo.toml --bin migrate-syntax3 --target-dir "$repo/target"')
    text = text.replace('target/release/examples/migrate-syntax3', 'target/release/migrate-syntax3')
    path.write_text(text)
    path = ROOT / 'scripts/test-macro-migrate'
    text = path.read_text().replace('debug/examples/migrate-tests', 'debug/migrate-tests')
    text = text.replace('crates/ilang-syntax/examples/migrate-tests.rs', 'crates/ilang-syntax/examples/legacy/migrate-tests.rs')
    text = text.replace("'cargo', 'build', '-p', 'ilang-syntax', '--example', 'migrate-tests', '--locked'",
        "'cargo', 'build', '--manifest-path', str(ROOT / 'crates/ilang-syntax/examples/legacy/Cargo.toml'), '--bin', 'migrate-tests', '--target-dir', str(BINARY.parent.parent), '--locked'")
    path.write_text(text)
    checker = 'check-snippets' if (ROOT / 'scripts/check-snippets').exists() else 'old-syntax-check'
    for name in [checker, 'syn3-convert-embedded']:
        path = ROOT / 'scripts' / name
        text = path.read_text()
        if name == checker:
            text = text.replace('crates/ilang-syntax/examples/migrate-syntax3', 'crates/ilang-syntax/examples/legacy/')
        else:
            text = text.replace('crates/ilang-syntax/examples/migrate-syntax3.rs', 'crates/ilang-syntax/examples/legacy/migrate-syntax3.rs')
        if name == checker:
            old_prefix = '    "crates/ilang-syntax/src/syn3/",'
            new_paths = ['ast.rs', 'parse.rs', 'print.rs', 'print/', 'walk.rs', 'rows.rs', 'rows/', 'facts.rs', 'facts/', 'group.rs', 'surface.rs', 'deep.rs', 'syn3.rs']
            text = text.replace(old_prefix, '\n'.join('    "crates/ilang-syntax/src/' + p + '",' for p in new_paths))
        path.write_text(text)


def finish_tool_readers():
    check = ROOT / 'crates/ilang-check'
    path = check / 'src/query.rs'
    text = path.read_text().replace('mod tree_legacy;', '#[cfg(test)]\nmod tree_legacy;')
    start = text.index('    fn parse_ast(')
    end = text.index('    // ---- type_at ----', start)
    text = text[:start] + """    fn parse_ast(&self, m: ModuleId) -> tree::Module {
        ilang_syntax::syn3::deep::deep(|| tree::from_new(&self.syntax(m), self.is_lisp(m)))
    }

""" + text[end:]
    path.write_text(text)
    # Preserve the differential index test with the old implementation kept
    # exclusively in the migration package, never in the production syntax crate.
    path = check / 'src/query/tree_legacy.rs'
    path.write_text(path.read_text().replace('ilang_syntax::', 'migration_syntax::'))
    manifest = check / 'Cargo.toml'
    manifest.write_text(manifest.read_text() + '\nmigration-syntax = { package = "ilang-syntax-migration", path = "../ilang-syntax/examples/legacy" }\n')
    # This is a compatibility oracle, not a production tree reader. Keep
    # its old needs-row assertions against the private migration lowerer.
    path = check / 'src/types/typed_macro.rs'
    text = path.read_text()
    marker = '    fn typed_macro_function_type_rows_are_read_back_by_the_lowering()'
    if marker in text:
        start = text.index(marker)
        end = text.index('    #[test]', start)
        oracle = text[start:end]
        if 'use ilang_syntax::ast::{TypeBody' in oracle:
            oracle = oracle.replace('ilang_syntax::ast::', 'migration_syntax::ast::')
            oracle = oracle.replace('ilang_syntax::syn3::parse_lisp_lowered', 'migration_syntax::lisp::parse_source')
            text = text[:start] + oracle + text[end:]
    path.write_text(text)
    path = ROOT / 'crates/ilang-cli/src/commands/expand.rs'
    text = path.read_text()
    marker = '    if ilang_syntax::syn3::FINAL_AST_READERS {'
    if marker in text:
        start = text.index(marker)
        end = text.index('    if trace {', start)
        block = text[start:end]
        old_visitor = '    } else {\n        use ilang_syntax::walk'
        if old_visitor in block:
            split = block.index(old_visitor)
            body = block[block.index('{') + 1:split]
            text = text[:start] + '    {' + body + '    }\n' + text[end:]
    if 'syn3::convert::convert(&module.ast' in text:
        bridge_start = text.index('let (mut tree, _) =')
        bridge_end = text.index(';', bridge_start) + 1
        if 'syn3::convert::convert' not in text[bridge_start:bridge_end]:
            raise RuntimeError('expanded-tree conversion boundary changed')
        text = text[:bridge_start] + 'let mut tree = module.ast.clone();' + text[bridge_end:]
    marker = '        let text = if ilang_syntax::syn3::FINAL_AST_READERS {'
    if marker in text:
        start = text.index(marker)
        end = text.index('        output.push_str', start)
        block = text[start:end]
        split = block.index('        } else {')
        body = block[block.index('{') + 1:split]
        text = text[:start] + '        let text = {' + body + '        };\n' + text[end:]
    path.write_text(text)
    path = ROOT / 'crates/ilang-lsp/src/analysis.rs'
    path.write_text(path.read_text().replace('module.ast = ilang_syntax::syn3::lower::lower(&tree).0;', 'module.ast = tree;'))


def apply():
    if not proof_path.exists() or json.loads(proof_path.read_text())['commit'] != LEGACY_REF:
        raise RuntimeError('record the green step-three proof for this exact integration head first')
    branch = subprocess.check_output(['git', 'branch', '--show-current'], cwd=ROOT, text=True).strip()
    if not (branch.startswith('lane/') and branch.endswith('-delete')):
        raise RuntimeError('apply only on a lane/*-delete side branch, after committing the green preparation')
    dirty = subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True)
    if dirty:
        raise RuntimeError('commit or deliberately finish pending changes before applying deletion')
    captured = ROOT / 'target/fa-r3/engine-boundary-oracle.json'
    if not captured.exists() or json.loads(captured.read_text())['source_hash'] != LEGACY_REF:
        raise RuntimeError('capture the five matching old/native engine answers on this exact head before deletion')
    private_migrator()
    manifest = ROOT / 'Cargo.toml'
    text = manifest.read_text()
    if 'exclude = ' not in text:
        text = text.replace('members = ["crates/*"]', 'members = ["crates/*"]\nexclude = ["crates/ilang-syntax/examples/legacy"]')
    manifest.write_text(text)
    update_migration_commands()
    finish_tool_readers()
    src = SYNTAX / 'src'
    for name in ['ast.rs', 'ast_eq.rs', 'ast_clone.rs', 'walk.rs', 'print.rs', 'shape.rs',
                 'syn3/lower.rs', 'syn3/convert.rs', 'lisp/encode.rs', 'lisp/encode_charge.rs',
                 'lisp/to_ast.rs', 'lisp/from_ast.rs']:
        (src / name).unlink(missing_ok=True)
    shutil.rmtree(src / 'legacy', ignore_errors=True)
    manifest = (SYNTAX / 'Cargo.toml').read_text()
    start = manifest.find('[features]')
    end = manifest.index('[dev-dependencies]')
    # Tests that depend on the legacy reader are retained, not silently skipped.
    # Their remaining old-tree uses are part of the integration audit.
    (SYNTAX / 'Cargo.toml').write_text(manifest[:start] + manifest[end:] if start >= 0 else manifest)
    lib = (src / 'lib.rs').read_text().replace('mod ast_clone;\n', '').replace('mod ast_eq;\n', '')
    lib = lib.replace('#[cfg(feature = "legacy")]\npub mod legacy;\n', '').replace('#[doc(hidden)]\npub mod shape;\n', '')
    lib = lib.replace('    syn3::parse_lowered(source)', """    let (module, mut errors) = syn3::parse_module(source);
    errors.extend(facts::Transition::capture_module(&module, false).1);
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (module, errors)""")
    shape = old('src/shape.rs').replace('crate::', 'migration_syntax::')
    lib += '\n#[cfg(test)]\n#[allow(dead_code)]\nmod legacy_shape {\n' + shape + '\n}\n'
    (src / 'lib.rs').write_text(lib)
    syn = (src / 'syn3/mod.rs').read_text()
    start = syn.find('/// Reads a normal-surface file')
    if start >= 0:
        end = syn.index('/// Temporary integration switch')
        syn = syn[:start] + syn[end:]
    syn = syn.replace('pub mod convert;\n', '').replace('pub mod lower;\n', '').replace('pub(crate) mod print;', 'pub mod print;')
    syn = syn.replace('pub const FINAL_AST_READERS: bool = false;', 'pub const FINAL_AST_READERS: bool = true;')
    (src / 'syn3/mod.rs').write_text(syn)
    # Production Lisp no longer exports old encoders or old datum readers.
    lisp = (src / 'lisp/mod.rs').read_text()
    for line in ['mod encode;\n', 'mod encode_charge;\n', 'mod to_ast;\n', 'mod from_ast;\n',
                 'pub use encode::{SourceEncoder, expression_syntax, item_syntax, pattern_syntax, type_syntax};\n',
                 'pub use encode_charge::SyntaxCharge;\n', 'pub use from_ast::print_module;\n', 'pub use from_ast::{print_module, print_with_comments};\n', 'mod render;\n']:
        lisp = lisp.replace(line, '')
    start = lisp.find('pub use to_ast::{')
    if start >= 0:
        end = lisp.index('};', start) + 3
        lisp = lisp[:start] + lisp[end:]
    start = lisp.find('/// Parses a Lisp-surface file with the rows before syntax three')
    if start >= 0:
        end = lisp.index('pub fn format(', start)
        # Preserve the formatter function; its preceding documentation is optional.
        lisp = lisp[:start] + lisp[end:]
    lisp = lisp.replace('    crate::syn3::parse_lisp_lowered(source)', """    let (module, mut errors) = crate::syn3::lisp::from_lisp(source);
    errors.extend(crate::syn3::facts::Transition::capture_module(&module, true).1);
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (module, errors)""")
    start = lisp.find('/// Read all datums with the rows before syntax three')
    if start >= 0:
        end = lisp.index('/// Read all datums of a syntax-three Lisp file', start)
        lisp = lisp[:start] + lisp[end:]
    (src / 'lisp/mod.rs').write_text(lisp)
    source = (src / 'lisp/to_source.rs').read_text()
    if 'fn checked_expression(' in source:
        first = source.index('/// The depth up to which')
        last = source.index('fn checked_expression(')
        native = source[first:last]
        if 'pub fn normal_source3(' not in native:
            native += """
pub fn normal_source3(document: &Document) -> Result<String> {
    crate::syn3::deep::deep(|| {
        let (module, errors) = crate::syn3::lisp::from_document(document);
        if let Some(error) = errors.into_iter().next() { return Err(error); }
        Ok(crate::syn3::print_tree(&module, false))
    })
}
"""
        # Keep old renderer tests executable through the private oracle, with
        # their exact assertions. Production normal printing uses native rows.
        tests = source[source.index('#[cfg(test)]'):]
        tests = tests.replace('    use super::*;',
            '    use super::*;\n    use migration_syntax::lisp::to_source::{show_as, normal_source, normal_with_comments};', 1)
        tests = tests.replace('super::super::render::', 'migration_syntax::lisp::render::')
        put(src / 'lisp/to_source.rs', """//! Position-directed printing of the final syntax-three tree.
use super::{Datum, DatumKind as K, Document};
use crate::span::Span;
use crate::{Code, Diagnostic};
type Result<T> = std::result::Result<T, Diagnostic>;
pub fn normal_source(document: &Document) -> Result<String> { normal_source3(document) }
""" + native + tests)
    render = (src / 'lisp/render.rs').read_text()
    tests = render[render.index('#[cfg(test)]'):]
    tests = tests.replace('mod tests {', 'mod legacy_render_tests {', 1)
    tests = tests.replace('    use super::*;', '    use super::*;\n    use migration_syntax::lisp::render::{P, render};', 1)
    target = src / 'lisp/to_source.rs'
    target.write_text(target.read_text() + '\n' + tests)
    (src / 'lisp/render.rs').unlink(missing_ok=True)
    # Convert/unconvert of old quote rows belongs only to the private migrator.
    rows = (src / 'syn3/lisp.rs').read_text()
    marker = rows.find('fn sym(s:')
    if marker >= 0:
        rows = rows[:marker]
    (src / 'syn3/lisp.rs').write_text(rows)
    # Raise syntax three to the crate root. The compatibility namespace has no
    # old tree; it allows independent consumer imports to settle in later commits.
    for path in list((src / 'syn3').iterdir()):
        if path.name == 'mod.rs':
            continue
        name = 'rows' + path.suffix if path.stem == 'lisp' else path.name
        if path.name == 'lisp':
            name = 'rows'
        shutil.move(str(path), str(src / name))
    helpers = syn[syn.index('/// Temporary integration switch'):]
    (src / 'syn3/mod.rs').unlink()
    (src / 'syn3').rmdir()
    put(src / 'syn3.rs', '''//! Compatibility namespace for the syntax-three tree now at the crate root.
pub use crate::{ast, deep, facts, group, rows as lisp, surface, walk, print};
pub use crate::parse::{parse_module, parse_expr, parse_type, parse_pattern, pipe_target};
pub use crate::print::{MAX_CALL_DEPTH, WIDTH, call_depth, format, print_expr, print_module,
    print_pattern, print_type, print_with_comments, string_literal, record_label};
''' + helpers)
    lib = (src / 'lib.rs').read_text().replace('pub mod syn3;', 'pub mod syn3;\npub mod deep;\npub mod facts;\npub mod group;\npub mod surface;\npub mod rows;\nmod parse;\npub use print::string_literal;')
    (src / 'lib.rs').write_text(lib)
    path = src / 'surface.rs'
    path.write_text(path.read_text().replace('super::lisp::', 'crate::rows::').replace('super::print_module(', 'crate::print::print_module(').replace('super::parse_module(', 'crate::parse::parse_module('))
    # Keep formatter parsing at its existing grammar-only boundary; the
    # crate-root public reader retains the old post-reader validation contract.
    path = src / 'print.rs'
    path.write_text(path.read_text().replace('super::parse_module(', 'crate::parse::parse_module('))
    # The new AST moved up a level; its literal printer still has one definition.
    path = src / 'ast.rs'
    path.write_text(path.read_text().replace('super::string_literal(s)', 'crate::print::string_literal(s)'))
    # Native facts were beside the row reader before flattening.
    path = src / 'facts.rs'
    path.write_text(path.read_text().replace('super::lisp::', 'crate::rows::'))
    # Keep all engine-owned source files unchanged. The check deliberately tells
    # the integrator which remaining old consumers still block step 4.
    # The current green resolver/engine have additional old-reader oracles.
    # Keep every assertion in the standalone migration package or unit adapters.
    import importlib.util
    for name in ['resolver', 'macro', 'source-syntax', 'definitions', 'lisp-compat', 'syntax-tests', 'comment-api', 'cli-oracle']:
        path = ROOT / 'scripts' / ('syn3-delete-' + name + '.py')
        spec = importlib.util.spec_from_file_location('syn3_delete_' + name.replace('-', '_'), path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.apply()
    print('Deletion applied. Run the wrapped workspace/all-targets check and record every error in §6.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare-migrator', action='store_true', help='copy the private migration library without deleting production files')
    parser.add_argument('--apply', action='store_true', help='apply only on a clean lane/*-delete side branch')
    args = parser.parse_args()
    if args.apply:
        apply()
    elif args.prepare_migrator:
        branch = subprocess.check_output(['git', 'branch', '--show-current'], cwd=ROOT, text=True).strip()
        if not (branch.startswith('lane/') and branch.endswith('-delete')):
            raise RuntimeError('prepare the private migration library on a lane/*-delete side branch')
        private_migrator(move_examples=False)
    else:
        print('No files changed. Use --apply on a clean lane/*-delete side branch after steps 2c, 2d and 3. See docs/design/syn3-final-ast.md §6.')
