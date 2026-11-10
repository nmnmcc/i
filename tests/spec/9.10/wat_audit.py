#!/usr/bin/env python3
"""Classify R1 WAT differences without changing the acceptance comparator.

The wat-diff comparison is run unchanged on every baseline export. A second,
explicitly diagnostic comparison identifies the six new constructor entries,
their data strings, and the compiler's nominal ID changes. Removing verified
additions in that diagnostic does not make the original comparison pass.
Artifacts are written only under target/r1/wat-audit.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import re
import runpy
import subprocess

ROOT = Path(__file__).resolve().parents[3]
WAT = runpy.run_path(str(ROOT / 'scripts/wat-diff'))
EXTRA = ['Split', 'Path', 'State', 'Generator', 'InvalidBound', 'InvalidPath']
FIRST_NEW_CTOR = 17  # Io is 16; number_types then inserts the six R1 constructors.
FIRST_NEW_TYPE = 22  # Four nominal types precede the first former program type.
DATA = re.compile(r'(?m)^\(data (\$s\d+) (' + WAT['STRING'] + r')\)\n')


def strings(text):
    return {m[1]: WAT['wat_bytes'](m[2]) for m in DATA.finditer(text)}


def install(text):
    marker = '(call $constructors_install '
    start = text.index(marker)
    end = WAT['end_form'](text, start)
    form = text[start:end]
    head = re.match(r'\(call \$constructors_install \(array.new_fixed \$values (\d+) ', form)
    assert head, form
    items = []
    pos = head.end()
    while pos < len(form) and form[pos].isspace():
        pos += 1
    while form.startswith('(', pos):
        stop = WAT['end_form'](form, pos)
        items.append(form[pos:stop])
        pos = stop
        while pos < len(form) and form[pos].isspace():
            pos += 1
    assert form[pos:] == '))'
    assert len(items) == int(head[1])
    names = strings(text)
    labels = []
    for item in items:
        match = re.fullmatch(r'\(call \$string_from_bytes \(array.new_data \$bytes (\$s\d+) \(i32.const 0\) \(i32.const (\d+)\)\)\)', item)
        assert match, item
        value = names[match[1]]
        assert len(value) == int(match[2])
        labels.append(value.decode())
    return start, end, items, labels


def symbols(text):
    """Rename data references by content; retain every byte of string data."""
    names = strings(text)
    use = re.compile(r'(\((?:data|array.new_data \$bytes) )(\$s\d+)(?=[\s)])')
    text = use.sub(lambda m: m[1] + '$string:' + names[m[2]].hex(), text)
    # wat-diff.renumber also sorts symbolic data definitions by content.
    lines = text.splitlines(keepends=True)
    data = sorted(line for line in lines if line.startswith('(data $string:'))
    return ''.join(line for line in lines if not line.startswith('(data $string:')) + ''.join(data)


def nominal_ids(text):
    counts = Counter()
    def ctor(n):
        n = int(n)
        if n >= FIRST_NEW_CTOR + len(EXTRA):
            counts['constructor_operands'] += 1
            return str(n - len(EXTRA))
        assert n < FIRST_NEW_CTOR, f'unexpected use of new constructor {n}'
        return str(n)
    def ty(n):
        n = int(n)
        if n >= FIRST_NEW_TYPE + 4:
            counts['type_operands'] += 1
            return str(n - 4)
        assert n < FIRST_NEW_TYPE, f'unexpected use of new type {n}'
        return str(n)
    # These are exactly the emitted record and nullary-global representations.
    text = re.sub(r'(\(call \$record \(i32.const )(\d+)(\) \(i64.const )(\d+)(\))',
                  lambda m: m[1] + ctor(m[2]) + m[3] + ty(m[4]) + m[5], text)
    text = re.sub(r'(\(struct.new \$obj \(i32.const 5\) \(i32.const )(\d+)(\) \(i64.const )(\d+)(\))',
                  lambda m: m[1] + ctor(m[2]) + m[3] + ty(m[4]) + m[5], text)
    def global_name(m):
        counts['nullary_global_references'] += 1
        return '$k' + ctor(m[1]) + '_' + ty(m[2])
    text = re.sub(r'\$k(\d+)_(\d+)(?=[\s)])', global_name, text)
    # A type/constructor test has an atom expression then a constant ID.
    for name, integer, change in [('is_ctor', 'i32', ctor), ('is_type', 'i64', ty)]:
        marker = '(call $' + name + ' '
        pos = 0
        pieces = []
        while (start := text.find(marker, pos)) >= 0:
            end = WAT['end_form'](text, start)
            form = text[start:end]
            match = re.search(r'(\(' + integer + r'\.const )(\d+)(\)\))$', form)
            assert match, form
            pieces.extend([text[pos:start], form[:match.start(2)], change(match[2]), form[match.end(2):]])
            pos = end
        pieces.append(text[pos:])
        text = ''.join(pieces)
    return text, counts


def measure(base, after, output, name):
    results = {'export': name}
    for label, directory in [('before', base), ('after', after)]:
        source = directory / name
        binary = output / (label + '-' + name + '.wasm')
        stripped = output / (label + '-' + name + '.stripped.wasm')
        subprocess.run(['wasm-tools', 'parse', str(source), '-o', str(binary)], check=True)
        subprocess.run(['wasm-tools', 'strip', str(binary), '-o', str(stripped)], check=True)
        subprocess.run(['wasm-tools', 'validate', str(stripped)], check=True)
        results[label] = {'wat_bytes': source.stat().st_size,
                          'wasm_bytes': binary.stat().st_size,
                          'stripped_wasm_bytes': stripped.stat().st_size,
                          'constructors_at_start': len(install(source.read_text())[2])}
    return results


def validate_exports(base, after, output):
    paths = [path for path in sorted(base.glob('*.wat'))
             if not path.read_text().startswith('ERROR')]
    def validate(path):
        current = after / path.name
        assert not current.read_text().startswith('ERROR'), path.name
        binary = output / (path.name + '.validated.wasm')
        subprocess.run(['wasm-tools', 'parse', str(current), '-o', str(binary)], check=True)
        subprocess.run(['wasm-tools', 'validate', str(binary)], check=True)
        return path.name
    with ThreadPoolExecutor(max_workers=4) as pool:
        names = list(pool.map(validate, paths))
    (output / 'validated-exports.json').write_text(json.dumps(names, indent=2) + '\n')
    return len(names)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('base', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    output = ROOT / 'target/r1/wat-audit'
    output.mkdir(parents=True, exist_ok=True)
    if args.validate_only:
        print(f'Validated {validate_exports(args.base, args.after, output)} existing exports as Wasm')
        return
    summary = Counter()
    files = []
    for path in sorted(args.base.glob('*.wat')):
        a = path.read_text()
        b = (args.after / path.name).read_text()
        row = {'export': path.name, 'baseline_error': a.startswith('ERROR')}
        summary['all_exports'] += 1
        try:
            # Existing comparator's normalization, no allowed owners/patterns.
            WAT['compare'](a, b, [], True)
            row['wat_diff'] = 'equal'
            summary['wat_diff_equal'] += 1
        except (WAT['Failure'], ValueError) as error:
            row['wat_diff'] = str(error)
            summary['wat_diff_failed'] += 1
        if a.startswith('ERROR'):
            assert a == b, 'baseline error changed: ' + path.name
            summary['unchanged_baseline_errors'] += 1
            files.append(row)
            continue
        assert not b.startswith('ERROR'), 'existing WAT no longer compiles: ' + path.name
        summary['existing_wat_compiles'] += 1
        if a == b:
            summary['raw_equal_wat'] += 1
        else:
            summary['raw_changed_wat'] += 1
        old_start, old_end, old_items, old_names = install(a)
        start, end, items, names = install(b)
        # This insertion follows number_types source order, not a loose mask.
        assert old_names[0] == names[0] == 'Io'
        assert names == old_names[:1] + EXTRA + old_names[1:]
        assert len(names) - len(old_names) == 6
        old_data, new_data = strings(a), strings(b)
        additions = set(new_data.values()) - set(old_data.values())
        assert additions <= {s.encode() for s in EXTRA}
        assert not (set(old_data.values()) - set(new_data.values()))
        old_order = list(old_data.values())
        new_order = [value for value in new_data.values() if value in set(old_order)]
        row['existing_data_definition_order_changed'] = old_order != new_order
        if row['existing_data_definition_order_changed']:
            summary['existing_data_definition_reordering'] += 1
        row['added_constructor_names'] = EXTRA
        row['added_string_definitions'] = sorted(s.decode() for s in additions)
        row['added_constructor_payload_bytes'] = sum(len(s) for s in additions)
        inverse, counts = nominal_ids(b)
        row['renumbering'] = dict(counts)
        row['only_renumbering'] = symbols(a) == symbols(inverse)
        summary['renumbering_only' if row['only_renumbering'] else 'has_nonrenumbering_additions'] += 1
        # Diagnostic only: remove the individually verified additions to ask
        # whether any OTHER program operation changed. Not an acceptance pass.
        start, end, items, _ = install(inverse)
        kept = items[:1] + items[7:]
        replacement = '(call $constructors_install (array.new_fixed $values ' + str(len(kept)) + ' ' + ' '.join(kept) + '))'
        diagnostic = inverse[:start] + replacement + inverse[end:]
        diagnostic = DATA.sub(lambda m: '' if WAT['wat_bytes'](m[2]) in additions else m[0], diagnostic)
        row['diagnostic_no_other_change'] = symbols(a) == symbols(diagnostic)
        summary['diagnostic_no_other_change' if row['diagnostic_no_other_change'] else 'unclassified_difference'] += 1
        if not row['diagnostic_no_other_change']:
            (output / (path.name + '.before')).write_text(symbols(a))
            (output / (path.name + '.diagnostic')).write_text(symbols(diagnostic))
        files.append(row)
        if len(files) % 50 == 0:
            print(f'Audited {len(files)} exports', flush=True)
    summary['assembled_and_validated_wasm'] = validate_exports(args.base, args.after, output)
    measurements = [measure(args.base, args.after, output, name) for name in
                    ['examples_hello.ilang.false.wat', '11_float_arithmetic.i.ilang.false.wat']]
    result = {'summary': dict(summary), 'measurements': measurements, 'files': files}
    (output / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'summary': summary, 'measurements': measurements}, indent=2))
    # Honest acceptance outcome: extra constructor installation is a failure.
    if summary['wat_diff_failed'] or summary['has_nonrenumbering_additions'] or summary['unclassified_difference']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
