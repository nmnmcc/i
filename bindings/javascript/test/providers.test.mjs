import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { instantiate, authorizeBindings } from '../ilang.mjs';
const cli = resolve(import.meta.dirname, '../../../target/debug/i');

async function build(source, plan, use) {
  const dir = await mkdtemp(join(tmpdir(), 'i-providers-'));
  try {
    await mkdir(join(dir, 'src'));
    await writeFile(join(dir, 'src/main.i'), source);
    await writeFile(join(dir, 'i.toml'), 'name = "app"\n' + plan);
    execFileSync(cli, ['build', '--target', 'nodejs', '--opt', '0', '--project', dir], { stdio: 'pipe' });
    const bytes = await readFile(join(dir, 'build/app.wasm'));
    const declarations = JSON.parse(await readFile(join(dir, 'build/app.bindings.json'), 'utf8'));
    const bindings = await authorizeBindings(bytes, declarations, () => true);
    await use(bindings);
  } finally { await rm(dir, { recursive: true, force: true }); }
}

test('user providers and their host leaves run in plan order on Node', async () => {
  await build('(S, probe)\n@host("custom")\nHost { Host(read: () -> Int) }\nS { S(value: Int) }\nmake = (host ! Host) { S(host.read() + 2) }\nprobe = (s ! S) { s.value }', '[wasm]\nexports = ["app/main.probe"]\n[providers."app/main.S"]\nnodejs = "app/main.make"\n', async (bindings) => {
    await assert.rejects(instantiate(bindings.module, { bindings }), /Missing host function custom read for service app\/main.Host/);
    let calls = 0;
    const guest = await instantiate(bindings.module, { bindings, contextImports: { custom: { read: () => { calls++; return 40n; } } } });
    assert.equal(calls, 0);
    assert.equal(guest.call('app/main.probe'), 42n);
    assert.equal(guest.call('app/main.probe'), 42n);
    assert.equal(calls, 2);
  });
});

test('a fake standard clock uses no host registration and retains request mode', async () => {
  await build('(i/clock, i/request, i/fail)\n(probe)\nfake = () { clock.Clock(() { request.ready(Ok(42)) }, () { request.ready(Ok(42)) }, (_duration: Int) { request.ready(Ok(Nil)) }) }\nprobe = () { fail.catch(() { clock.monotonic_ns() }) |> { Ok(value) -> value Error(_) -> panic("fake clock failed") } }', '[wasm]\nexports = ["app/main.probe"]\n[providers."i/clock.Clock"]\nnodejs = "app/main.fake"\n', async (bindings) => {
    const guest = await instantiate(bindings.module, { bindings });
    assert.equal(guest.abi.exports['app/main.probe'].async, true);
    assert.deepEqual(Object.keys(guest.abi.services), []);
    assert.equal(await guest.callAsync('app/main.probe'), 42n);
  });
});
