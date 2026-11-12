import { GuestError, requestSignal } from './ilang.mjs';
import { ResourceError } from './resources.mjs';

const fail = value => { throw new GuestError(value); };
const bits = bytes => ({ bytes: new Uint8Array(bytes), bitLength: bytes.length * 8 });
function failure(error) {
  if (error instanceof GuestError || (error instanceof ResourceError && error.code === "memory exhausted")) throw error;
  const code = error.cause?.code ?? error.code;
  if (code === 'ENOTFOUND' || code === 'EAI_AGAIN') fail('dns:' + code);
  if (error instanceof ResourceError) fail('connect:' + error.code);
  fail('connect:' + (code ?? error.name));
}
const checked = fn => async (...args) => { try { return await fn(...args); } catch (error) { failure(error); } };

/** WHATWG fetch transport. Its response and reader are run-owned resources. */
export function fetchServices() {
  const use = async (ctx, id, lane, action) => {
    const lease = ctx.acquire(id, 'fetch-response', lane);
    const signal = requestSignal();
    const abort = () => { lease.release(); ctx.close(id, 'fetch-response'); };
    signal.addEventListener('abort', abort, { once: true });
    try { return lease.commit(await action(lease.value)); }
    finally { signal.removeEventListener('abort', abort); lease.release(); }
  };
  return {
    start: checked(async (ctx, method, url, encoded, body) => {
      const signal = requestSignal(), controller = new AbortController(), job = ctx.cleanupJob();
      const abort = () => controller.abort();
      signal.addEventListener('abort', abort, { once: true });
      try {
        if (body.bitLength % 8) fail('protocol:invalid body');
        const headers = new Headers(), pairs = encoded.split('\0');
        for (let i = 0; i + 1 < pairs.length; i += 2) headers.append(pairs[i], pairs[i + 1]);
        const response = await fetch(url, { method, headers, body: body.bytes.length ? body.bytes : undefined, signal: controller.signal });
        const value = { response, controller, reader: response.body?.getReader(), pending: new Uint8Array(), done: !response.body };
        const id = ctx.insert('fetch-response', value, async value => { try { await value.reader?.cancel(); } finally { value.controller.abort(); } });
        if (signal.aborted) { ctx.close(id, 'fetch-response'); throw new ResourceError('closed'); }
        return id;
      } catch (error) { controller.abort(); throw error; }
      finally { signal.removeEventListener('abort', abort); job.complete(); }
    }),
    status: checked((ctx, id) => use(ctx, id, 'metadata', value => BigInt(value.response.status))),
    headers: checked((ctx, id) => use(ctx, id, 'metadata', value => [...value.response.headers].map(([k, v]) => k + '\0' + v + '\0').join(''))),
    read: checked((ctx, id, max) => use(ctx, id, 'read', async value => {
      if (max <= 0n) fail('protocol:invalid read size');
      const n = Number(max > 65536n ? 65536n : max);
      while (!value.pending.length && !value.done) {
        const next = await value.reader.read();
        value.done = next.done;
        value.pending = next.value ?? new Uint8Array();
      }
      const chunk = value.pending.slice(0, n);
      value.pending = value.pending.subarray(chunk.length);
      return bits(chunk);
    })),
    close: checked((ctx, id) => ctx.close(id, 'fetch-response')),
  };
}
