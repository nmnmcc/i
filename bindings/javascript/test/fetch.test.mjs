import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { instantiate, GuestError } from '../ilang.mjs';
const root = resolve(import.meta.dirname, '../../..');
const cli = resolve(root, 'target/debug/i');

// Compile once, then exercise ordinary http functions through their JS provider plan.
test('fetch provider: GET POST headers status timeout cancel refused streamed body', { timeout: 120000 }, async t => {
  let aborted, bodyAborted, streamResponse;
  const wasAborted = new Promise(resolve => { aborted = resolve; });
  const bodyWasAborted = new Promise(resolve => { bodyAborted = resolve; });
  const server = createServer(async (req, res) => {
    if (req.url === '/slow' || req.url === '/cancel') {
      res.on('close', () => { if (req.url === '/cancel') aborted(); });
      return;
    }
    if (req.url === '/body-cancel') { res.on('close', bodyAborted); res.write('partial'); return; }
    if (req.url === '/stream') {
      streamResponse = res;
      res.writeHead(200, { 'x-provider': 'fetch' }); res.write('first'); return;
    }
    if (req.url === '/stream/advance') { streamResponse.end('second'); res.end('advanced'); return; }
    const chunks = []; for await (const chunk of req) chunks.push(chunk);
    res.writeHead(req.url === '/status' ? 418 : 200, { 'x-provider': 'fetch' });
    res.end(req.method === 'POST' ? Buffer.concat(chunks) : req.headers['x-request'] ?? 'hello');
  });
  server.listen(0, '127.0.0.1'); await once(server, 'listening');
  const base = `http://127.0.0.1:${server.address().port}`;
  const dir = await mkdtemp(join(tmpdir(), 'i-fetch-'));
  try {
    await mkdir(join(dir, 'src'));
    await writeFile(join(dir, 'i.toml'), `name = "app"\n[dependencies]\nstd = {path = ${JSON.stringify(join(root, 'std'))}}\n[wasm]\nexports = ["app/main.get", "app/main.post", "app/main.headers", "app/main.status", "app/main.timeout", "app/main.cancel", "app/main.stream"]\n`);
    await writeFile(join(dir, 'src/main.i'), `(
	i/fail
	i/core
	i/clock
	i/task
	std/http
	std/bit_array
	(Some, None) = std/option
)

(
	get
	post
	headers
	status
	timeout
	cancel
	stream
)
checked = <A, N>(body: () <- N -> A) -> A {
	fail.catch(body) |>
		(Ok(value)) { value }
		| (Error(e)) { fail.raise(core.inspect(e)) }
}
text = (res: http.Response) {
	http.body_text(res) |> (Ok(s)) { s } | (Error(_)) { panic("bad UTF-8") }
}
get = (url: String) { checked(() { text(http.get(url)) }) }
post = (url: String) { checked(() { text(http.post(url, <<"posted":utf8>>)) }) }
headers = (url: String) {
	checked(() {
		initial = http.request(http.Method("GET"), url)
		req = http.Request(
			method = initial.method,
			target = initial.target,
			headers = http.Headers([("X-Request", "sent")]),
			body = initial.body,
			trailers = initial.trailers,
		)
		res = http.send(req)
		assert(text(res) == "sent")
		http.header(res.headers, "X-Provider") |>
			(Some(s)) { s }
			| (None) { panic("missing response header") }
	})
}
status = (url: String) {
	checked(() {
		http.Status(n) = http.get(url).status
		n
	})
}
timeout = (url: String) {
	checked(() {
		text(
			http.send_timeout(
				http.client(),
				http.request(http.Method("GET"), url),
				20000000,
			),
		)
	})
}
cancel = (url: String) {
	checked(() {
		task.race(
			() { text(http.get(url)) },
			() {
				clock.sleep_ns(100000000)
				"cancelled"
			},
		)
	})
}
stream = (url: String) {
	checked(() {
		reader = http.send_stream(http.request(http.Method("GET"), url))
		first = reader.read(5)
		assert(first == <<"first":utf8>>)
		assert(
			text(http.get(url <> "/advance")) == "advanced",
		)
		second = reader.read(6)
		assert(second == <<"second":utf8>>)
		assert(reader.read(1) == <<>>)
		reader.close()
		"stream ok"
	})
}
`);
    execFileSync(cli, ['build', '--project', dir, '--target', 'js', '--opt', '0'], { stdio: 'pipe' });
    const wasm = await readFile(join(dir, 'build/app.wasm'));
    const plan = execFileSync(cli, ['deps', '--project', dir, '--target', 'js'], { encoding: 'utf8' });
    assert.match(plan, /fetch/); assert.doesNotMatch(plan, /i:(net|tls)\//);
    const guest = await instantiate(wasm, { standardServices: true });
    await t.test('fetch GET', async () => assert.equal(await guest.callAsync('app/main.get', [base]), 'hello'));
    await t.test('fetch POST body', async () => assert.equal(await guest.callAsync('app/main.post', [base]), 'posted'));
    await t.test('fetch headers', async () => assert.equal(await guest.callAsync('app/main.headers', [base]), 'fetch'));
    await t.test('non-2xx response is a response', async () => assert.equal(await guest.callAsync('app/main.status', [base + '/status']), 418n));
    await t.test('timeout aborts fetch', async () => assert.rejects(guest.callAsync('app/main.timeout', [base + '/slow']), e => e instanceof GuestError && e.value === 'Timeout'));
    await t.test('task cancel aborts fetch', async () => { assert.equal(await guest.callAsync('app/main.cancel', [base + '/cancel']), 'cancelled'); await wasAborted; });
    await t.test('task cancel aborts a streamed response body', async () => { assert.equal(await guest.callAsync('app/main.cancel', [base + '/body-cancel']), 'cancelled'); await bodyWasAborted; });
    await t.test('connection refused', async () => {
      const closed = createServer(); closed.listen(0, '127.0.0.1'); await once(closed, 'listening');
      const port = closed.address().port; await new Promise(resolve => closed.close(resolve));
      await assert.rejects(guest.callAsync('app/main.get', [`http://127.0.0.1:${port}`]), e => e instanceof GuestError && e.value.startsWith('Connect('));
    });
    await t.test('DNS failure', async () => assert.rejects(guest.callAsync('app/main.get', ['http://i-fetch-no-such-host.invalid']), e => e instanceof GuestError && e.value.startsWith('Dns(')));
    await t.test('stream reads arrive before response end', async () => assert.equal(await guest.callAsync('app/main.stream', [base + '/stream']), 'stream ok'));
  } finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); await rm(dir, { recursive: true, force: true }); }
});
