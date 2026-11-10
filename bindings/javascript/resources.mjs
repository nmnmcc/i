/** Run-owned resources, independent of Wasm, request ids and language finals. */
export class ResourceError extends Error {
  constructor(code) { super(code); this.name = "ResourceError"; this.code = code; }
}

/** One live host identity domain; reuse across instances and sequential tests. */
export class ResourceHost {
  #next = 1n;
  #limit;
  constructor({ limit } = {}) {
    if (limit !== undefined && typeof limit !== "bigint") throw new TypeError("Resource limit must be bigint");
    this.#limit = limit;
  }
  context() { return new ResourceContext(() => {
    if (this.#limit !== undefined && this.#next > this.#limit) throw new ResourceError("memory exhausted");
    return this.#next++;
  }); }
}

export class ResourceContext {
  #allocate;
  #entries = new Map();
  #closing = new Map();
  #live = true;
  constructor(allocate) { this.#allocate = allocate; }
  get live() { return this.#live; }
  /** Cleanup owns host data only and returns a Promise if cleanup is deferred. */
  insert(kind, value, cleanup) {
    const dispose = () => {
      try { return Promise.resolve(cleanup(value)).catch(() => {}); }
      catch { return Promise.resolve(); }
    };
    let id;
    try { id = this.#allocate(); } catch (error) { dispose(); throw error; }
    if (!this.#live) { dispose(); throw new ResourceError("closed"); }
    this.#entries.set(id, { kind, value, dispose, leases: new Map() });
    return id;
  }
  acquire(id, kind, lane) {
    const entry = this.#entries.get(id);
    if (!entry || entry.kind !== kind) throw new ResourceError("closed");
    if (entry.leases.has(lane)) throw new ResourceError("busy");
    const abort = new AbortController();
    entry.leases.set(lane, abort);
    const release = () => { if (entry.leases.get(lane) === abort) entry.leases.delete(lane); };
    return Object.freeze({
      value: entry.value,
      signal: abort.signal,
      release,
      commit: (answer) => {
        if (this.#entries.get(id) !== entry || entry.leases.get(lane) !== abort) throw new ResourceError("closed");
        release();
        return answer;
      },
    });
  }
  /** Remove first, then revoke/wake and run cleanup. Reentrant closes join it. */
  close(id, kind) {
    const pending = this.#closing.get(id);
    if (pending) return pending.kind === kind ? pending.receipt : Promise.resolve();
    const entry = this.#entries.get(id);
    if (!entry || entry.kind !== kind) return Promise.resolve();
    this.#entries.delete(id);
    let complete;
    const receipt = new Promise((resolve) => { complete = resolve; });
    this.#closing.set(id, { kind, receipt });
    for (const abort of entry.leases.values()) abort.abort();
    entry.leases.clear();
    entry.dispose().then(() => {
      this.#closing.delete(id);
      complete();
    });
    return receipt;
  }
  /** Close admission synchronously, including to creators that finish late. */
  teardown() {
    this.#live = false;
    for (const [id, entry] of this.#entries) this.close(id, entry.kind);
    return Promise.all([...this.#closing.values()].map(({ receipt }) => receipt)).then(() => {});
  }
  /** An explicit drop guard for a not-yet-published resource reply (N9.4). */
  provisional(id, kind) {
    let delivered = false;
    return Object.freeze({
      id,
      deliver: () => {
        if (this.#entries.get(id)?.kind !== kind) throw new ResourceError("closed");
        delivered = true;
        return id;
      },
      discard: () => delivered ? Promise.resolve() : this.close(id, kind),
    });
  }
}
