import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { runTests } from '../ilang.mjs';

const root = resolve(import.meta.dirname, '../../..');
if (process.env.I_UDP_TEST_WASM) {
  // The CLI integration test builds the same i test collection at O0 and O2.
  test('UDP i test collection on Node', { timeout: 120000 }, async () => {
    const results = await runTests(await readFile(process.env.I_UDP_TEST_WASM), { standardServices: true });
    assert.equal(results.length, 5);
    for (const result of results) assert.equal(result.passed, true, `${result.name}: ${result.error?.stack}`);
  });
} else {
  test('UDP IPv4/IPv6, boundaries, errors, close and scope cancellation on Node', { timeout: 120000 }, () => {
    const project = resolve(root, 'examples/io-udp');
    for (const opt of ['0', '2']) {
      const wasm = resolve(project, `build/udp-checks-o${opt}.wasm`);
      execFileSync(resolve(root, 'target/debug/i'), ['build', 'checks', '--target', 'nodejs', '--opt', opt, '--project', project, '--out', wasm], { cwd: root, stdio: 'pipe' });
      execFileSync(process.execPath, [resolve(root, 'bindings/javascript/run.mjs'), wasm], { cwd: root, stdio: 'pipe', timeout: 60000 });
    }
  });
}
