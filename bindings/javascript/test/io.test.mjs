import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { createConnection } from 'node:net';
import { createInterface } from 'node:readline';
import { once } from 'node:events';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '../../..');
const cli = resolve(root, 'target/debug/i');
const runner = resolve(root, 'bindings/javascript/run.mjs');
function build(name) {
  const project = resolve(root, 'examples', name);
  execFileSync(cli, ['build', '--target', 'host', '--project', project], { cwd: root, stdio: 'pipe' });
  return resolve(project, 'build', name.replaceAll('-', '_') + '.wasm');
}
for (const name of ['io-clock', 'io-fs', 'io-net', 'io-process', 'io-tls']) {
  test(name, { timeout: 120000 }, () => {
    const wasm = build(name);
    const output = execFileSync(process.execPath, [runner, wasm, ...(name === 'io-process' ? ['--', 'alpha', 'beta'] : [])], { cwd: root, encoding: 'utf8', timeout: 100000 });
    assert.equal(output.trimEnd().split('\n').at(-1), name.slice(3) + ' ok');
  });
}
test('echo handles two simultaneous clients', { timeout: 120000 }, async () => {
  const wasm = build('echo');
  const server = spawn(process.execPath, [runner, wasm], { cwd: root, stdio: ['ignore', 'pipe', 'inherit'] });
  try {
    const lines = createInterface({ input: server.stdout });
    const [line] = await once(lines, 'line');
    const match = /^listening (\d+)$/.exec(line);
    assert.ok(match, line);
    const connect = async () => { const socket = createConnection({host:'127.0.0.1',port:Number(match[1])}); await once(socket,'connect'); return socket; };
    const [a,b] = await Promise.all([connect(),connect()]);
    const collect = socket => new Promise((resolve,reject) => { const chunks=[];socket.on('data',chunk=>chunks.push(chunk));socket.on('end',()=>resolve(Buffer.concat(chunks).toString()));socket.on('error',reject); });
    const replies = Promise.all([collect(a),collect(b)]);
    a.write('first '); b.write('second '); a.end('client'); b.end('connection');
    assert.deepEqual(await replies,['first client','second connection']);
  } finally { server.kill(); }
});
