#!/usr/bin/env python3
"""Run the 331 known answers on native, JavaScript and WASI at O0/O2.

direnv exec /home/agent/Programming/i python3 std/test/float_hosts.py

Creates only build artifacts below target/std3-hosts. The copied math module
adds one private-path classification export for the gate; production APIs are
unchanged. Classification runs the real fast_sincos certificate. Hypot uses its
exact-square path and is counted separately from interval refinement.
"""
import json
from pathlib import Path
import re
import subprocess
from float_gate import cases

ROOT = Path(__file__).resolve().parents[2]


def command(args):
    print(' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=ROOT, check=True)


def run():
    work = ROOT / 'target/std3-hosts'
    src = work / 'src'
    src.mkdir(parents=True, exist_ok=True)
    (work / 'i.toml').write_text('name = "probe"\n[dependencies]\nstd = { path = "../../std" }\n'
                               '[wasm]\nexports = ["probe/math.slow_probe"]\n')
    fixture = (ROOT / 'std/test/float_mpfr_test.i').read_text()
    functions = re.findall(r'^(mpfr_\w+) = ', fixture, re.M)
    assert len(functions) == 331
    (src / 'cases.i').write_text(fixture.replace('@test\n', '') + '\n@main\nrun_all = () {\n' +
                                '\n'.join(' ' + fn + '()' for fn in functions) + '\n}\n')
    math = (ROOT / 'std/src/float.i').read_text()
    math = math.replace('\tmin_positive\n', '\tmin_positive\n\tslow_probe\n', 1)
    math += '''
slow_probe = (code: Int, x: Float, y: Float) -> Bool {
 tiny = 7.450580596923828e-9
 code |> {
  0 -> { abs(x) >. tiny } && { fast_sincos(x, False) |> { None -> True Some(_) -> False } }
  1 -> { abs(x) >. tiny } && { fast_sincos(x, True) |> { None -> True Some(_) -> False } }
  2 -> abs(x) >. tiny
  3 -> abs(x) >. tiny && abs(x) <. 1.0
  4 -> abs(x) <. 1.0 && x != 0.0
  5 -> abs(x) >. tiny
  6 -> x != 0.0 && y != 0.0
  7 -> False
  _ -> x != 0.0
 }
}
'''
    (src / 'math.i').write_text(math)
    inputs = cases()
    rows = [dict(group=r['group'], code=['sin', 'cos', 'tan', 'asin', 'acos', 'atan',
                                      'atan2', 'hypot', 'radians', 'degrees'].index(r['fn']),
                 x=r['x'], y=r['y'], fn=r['fn']) for r in inputs]
    (work / 'inputs.json').write_text(json.dumps(rows))
    sdk = (ROOT / 'bindings/javascript/ilang.mjs').as_uri()
    js = f'''import {{ readFileSync, writeFileSync }} from 'node:fs';
import {{ instantiate }} from {json.dumps(sdk)};
const app = await instantiate(new Uint8Array(readFileSync(process.argv[2])));
app.run();
const rows = JSON.parse(readFileSync(new URL('./inputs.json', import.meta.url)));
for (const r of rows) r.slow = app.call('probe/math.slow_probe', [BigInt(r.code), r.x, r.y]);
writeFileSync(new URL('./slow-share.json', import.meta.url), JSON.stringify(rows, null, 2));
for (const group of ['ordinary', 'huge', 'subnormal', 'hard']) {{
 const selected = rows.filter(r => r.group === group);
 console.log(group, 'interval path', selected.filter(r => r.slow).length, '/', selected.length,
             'exact norm', selected.filter(r => r.fn === 'hypot').length);
}}
console.log('331 JavaScript known answers passed');
'''
    (work / 'run.mjs').write_text(js)
    compiler = ROOT / 'target/release/i'
    for opt in [0, 2]:
        command([compiler, 'run', '--project', work, '--opt', opt])
        wasm = work / f'host-o{opt}.wasm'
        command([compiler, 'build', '--project', work, '--opt', opt, '--out', wasm])
        command(['node', work / 'run.mjs', wasm])
        wasi = work / f'wasi-o{opt}.wasm'
        command([compiler, 'build', '--project', work, '--opt', opt, '--target', 'wasi', '--out', wasi])
        command(['wasmtime', 'run', '-W', 'exceptions=y', wasi])
        print('331 native, JavaScript and WASI known answers passed at O' + str(opt), flush=True)


if __name__ == '__main__':
    run()
