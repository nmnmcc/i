import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
const root = resolve(import.meta.dirname, '../../..'), cli = join(root, 'target/debug/i');
const runner = join(root, 'bindings/javascript/wasi-run.mjs');
function build(project, name, opt) {
  const output = join(project, 'build', name + '.wasi-o' + opt + '.wasm');
  execFileSync(cli, ['build', '--project', project, '--target', 'wasi', '--opt', String(opt), '--out', output], { stdio: 'pipe' });
  return output;
}
function run(wasm, options) {
  return execFileSync(process.execPath, [runner, wasm, JSON.stringify(options)], { encoding: 'utf8', timeout: 60000 });
}
for (const opt of [0, 2]) {
  test(`WASI hello, clock and preopened Fs O${opt}`, { timeout: 120000 }, async () => {
    const dir = await mkdtemp(join(tmpdir(), 'i-wasi-fs-'));
    try {
      for (const [example, expected] of [['hello', 'Hello from i!'], ['io-clock', 'clock ok'], ['io-fs', 'fs ok']]) {
        const project = join(root, 'examples', example);
        const wasm = build(project, example.replaceAll('-', '_'), opt);
        assert.equal(run(wasm, { preopens: { '/tmp': dir } }).trim(), expected);
      }
    } finally { await rm(dir, { recursive: true, force: true }); }
  });
  test(`WASI args env entropy and boundary errors O${opt}`, { timeout: 120000 }, async () => {
    const dir = await mkdtemp(join(tmpdir(), 'i-wasi-args-'));
    try {
      await mkdir(join(dir, 'src'));
      await writeFile(join(dir, 'i.toml'), 'name = "app"\n');
      await writeFile(join(dir, 'src/main.i'), `(
	i/process
	i/stdio
	i/core
	i/fail
	i/fs
	i/clock
)
main = () {
	clock.sleep_ns(0)
	assert(
		fail.catch(() { clock.sleep_ns(-1) }) ==
			Error(clock.ClockError(operation = "sleep_ns", code = "invalid")),
	)
	assert(
		fail.catch(() { clock.sleep_ns(core.shift_left(1, 100)) }) ==
			Error(clock.ClockError(operation = "sleep_ns", code = "range")),
	)
	assert(process.args() == ["alpha", "beta"])
	assert(process.env("I_WASI_TEST") == "value")
	assert(
		core.bit_size(process.entropy(32)) == 256,
	)
	assert(
		fail.catch(() { process.env("MISSING") }) ==
			Error(process.ProcessError(operation = "env", code = "not_found")),
	)
	assert(
		fail.catch(() { process.entropy(-1) }) ==
			Error(
				process.ProcessError(operation = "entropy", code = "invalid"),
			),
	)
	assert(
		fail.catch(() { fs.read_file("/outside/no-preopen") }) ==
			Error(
				fs.FsError(
					operation = "open",
					path = "/outside/no-preopen",
					code = "denied",
				),
			),
	)
	stdio.print("args env entropy ok")
}
`);
      const wasm = build(dir, 'app', opt);
      assert.equal(run(wasm, { args: ['alpha', 'beta'], env: { I_WASI_TEST: 'value' } }).trim(), 'args env entropy ok');
    } finally { await rm(dir, { recursive: true, force: true }); }
  });
}
test('WASI rejects unavailable Net, Tls and Process at compile time', { timeout: 120000 }, async () => {
  const net = spawnSync(cli, ['build', '--project', join(root, 'examples/io-net'), '--target', 'wasi'], { encoding: 'utf8' });
  assert.notEqual(net.status, 0); assert.match(net.stderr, /Net/); assert.match(net.stderr, /wasi/i);
  const dir = await mkdtemp(join(tmpdir(), 'i-wasi-unavailable-'));
  try {
    await mkdir(join(dir, 'src')); await writeFile(join(dir, 'i.toml'), 'name = "app"\n');
    for (const [source, service] of [['(\n\ti/tls\n)\nmain = () <- (t: tls.Tls) { Nil }', 'Tls'], ['(\n\ti/process\n)\nmain = () <- (p: process.Process) { Nil }', 'Process']]) {
      await writeFile(join(dir, 'src/main.i'), source);
      const out = spawnSync(cli, ['build', '--project', dir, '--target', 'wasi'], { encoding: 'utf8' });
      assert.notEqual(out.status, 0); assert.match(out.stderr, new RegExp(service)); assert.match(out.stderr, /wasi/i);
    }
  } finally { await rm(dir, { recursive: true, force: true }); }
});
