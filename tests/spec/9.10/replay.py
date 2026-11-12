#!/usr/bin/env python3
"""Run the §9.10 assertions on Rust, Node's SDK, and both stock WASI hosts.

Run through direnv after building target/release/i. Uses only ignored target/
artifacts. The spec runner separately checks the same cases at O0 and O2 in
Rust; this script checks their identical assertions in other targets/engines.
"""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
WORK = ROOT / 'target/r1/replay'


def run(*args):
    subprocess.run([str(a) for a in args], cwd=ROOT, check=True)


def main():
    cli = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/release/i'
    WORK.mkdir(parents=True, exist_ok=True)
    sdk = (ROOT / 'bindings/javascript/ilang.mjs').as_uri()
    node = WORK / 'node.mjs'
    node.write_text(f'''import {{ readFile }} from 'node:fs/promises';
import {{ WASI }} from 'node:wasi';
import {{ instantiate }} from {sdk!r};
const bytes = await readFile(process.argv[3]);
if (process.argv[2] === 'ilang') {{
  for (let i = 0; i < 2; ++i) {{
    const guest = await instantiate(bytes);
    await guest.run();
  }}
}} else {{
  const wasi = new WASI({{ version: 'preview1', args: [], returnOnExit: true }});
  const module = await WebAssembly.compile(bytes);
  const guest = await WebAssembly.instantiate(module, wasi.getImportObject());
  process.exitCode = wasi.start(guest);
}}
''')
    combinations = 0
    for name in ['laws', 'vectors', 'interface']:
        source = Path(__file__).with_name(name + '.i').read_text()
        # Preserve the literal macro bodies as ordinary named functions for
        # command targets, which execute one main rather than the test runner.
        tests = re.findall(r'test\("([^"\n]+)",\s*\(\)\s*\{', source)
        assert tests
        project = WORK / name
        (project / 'src').mkdir(parents=True, exist_ok=True)
        (project / 'i.toml').write_text('name = "replay"\n')
        source = re.sub(r'\s*testlib\s*=\s*\(test\)\s*=\s*std/test\s*,?', '', source)
        source = re.sub(r'test\("([^"\n]+)",\s*(\(\)\s*\{)', r'\1 = \2', source)
        source = re.sub(r'(?m)^\}\)\s*$', '}', source)
        source += '\nmain = () {\n' + ''.join('\t' + t + '()\n' for t in tests) + '\tNil\n}\n'
        (project / 'src/main.i').write_text(source)
        for level in [0, 2]:
            run(cli, 'run', '--project', project, '--opt', level)
            combinations += 1
            for target in ['ilang', 'wasi']:
                wasm = project / f'{target}-{level}.wasm'
                run(cli, 'build', '--project', project, '--opt', level,
                    '--target', target, '--out', wasm)
                run('node', '--no-warnings', node, target, wasm)
                combinations += 1
                if target == 'wasi':
                    run('wasmtime', 'run', wasm)
                    combinations += 1
            print(f'{name} O{level}: Rust host, Node SDK, Node WASI, Wasmtime WASI pass', flush=True)
    print(f'Cross-target replay: {combinations} engine/case/optimization combinations passed')


if __name__ == '__main__':
    main()
