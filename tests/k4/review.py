#!/usr/bin/env python3
"""Run the named review programs against the review base, merged base, and lane."""
import argparse
import json
from pathlib import Path
import subprocess

FIXTURES = Path(__file__).resolve().parents[2] / 'crates/ilang-cli/tests/k4'
NEGATIVE = ['row_alias', 'x1', 'x2', 'x3', 'x4', 'x6', 'x7', 'x8', 'z1', 'z4', 'g_ann_SDep', 'g_inf_SDep', 'm3', 'm4', 'r6', 'b4', 'b7', 'r2', 'r3']


def cases():
    for name in NEGATIVE:
        yield name, (FIXTURES / (name + '.i')).read_text(), True, ''
    for n in range(21, 41):
        source = (FIXTURES / f'q{n}.i').read_text()
        yield f'q{n}', source + '\n@main\nmain = () { ! S(Never) = S(() { 1 }) assert(fw() == 1) }\n', False, ''
    yield 'r4', (FIXTURES / 'r4.i').read_text(), False, 'four\nthree\n'
    for n in range(1, 17):
        for row in ['Never', 'Dep']:
            source = (FIXTURES / f'w{n}.i').read_text()
            marker = f', (_: {row}) {{ Nil }}' if n >= 9 else ''
            service = f'S(() {{ {"1" if row == "Never" else "get()"} }}{marker})'
            if n in [1, 2, 9, 10]:
                call = 'wrap(() { use() })'
            elif n in [3, 4, 5, 11, 12, 13]:
                call = f'wrap(() {{ use() }}, {service})'
            else:
                call = f'wrap({service})'
            expected = '1' if row == 'Never' or n in [1, 2, 9, 10] else '7'
            supplied = ''
            if row == 'Dep':
                source = 'Dep { Dep(value: Int) }\nget = (d ! Dep) { d.value }\n' + source
                supplied = '! Dep = Dep(7)'
            yield f'w{n}-{row}', source + f'\n@main\nmain = () {{ {supplied} assert({call} == {expected}) }}\n', False, ''


def run(binary, project, args):
    try:
        p = subprocess.run([str(binary), *args, '--project', str(project)], capture_output=True, text=True, timeout=60)
        return {'status': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr}
    except subprocess.TimeoutExpired:
        return {'status': 'timeout', 'stdout': '', 'stderr': ''}


def main():
    p = argparse.ArgumentParser()
    for version in ['before', 'merged', 'after']:
        p.add_argument('--' + version, required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    results = []
    failures = []
    for name, source, reject, stdout in cases():
        project = a.out / name
        (project / 'src').mkdir(parents=True, exist_ok=True)
        (project / 'i.toml').write_text('name = "app"\n')
        (project / 'src/main.i').write_text(source)
        record = {'name': name, 'reject': reject}
        for version in ['before', 'merged', 'after']:
            binary = getattr(a, version)
            checked = run(binary, project, ['check', '--types'])
            runs = [run(binary, project, ['run', '--opt', opt]) for opt in ['0', '2']]
            record[version] = {'check': checked, 'runs': runs}
        after = record['after']
        if reject:
            valid = all(r['status'] == 1 and ('E0515' in r['stderr'] or 'E0522' in r['stderr']) and 'trap' not in r['stderr'] for r in [after['check'], *after['runs']])
        else:
            valid = after['check']['status'] == 0 and all(r['status'] == 0 and r['stdout'] == stdout for r in after['runs'])
        if not valid:
            failures.append(name)
        results.append(record)
        (a.out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    changed = {version: [r['name'] for r in results if r[version]['check']['status'] == 0 and r['after']['check']['status'] != 0] for version in ['before', 'merged']}
    print(json.dumps({'cases': len(results), 'failures': failures, 'accepted_then_rejected': changed}, indent=2))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
