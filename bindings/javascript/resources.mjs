/** Run-owned resources, independent of Wasm, request ids and language finals. */
export class ResourceError extends Error {
  constructor(code) { super(code); this.name = "ResourceError"; this.code = code; }
}

const contextKey = Symbol("ResourceContext constructor");

/** One live host identity domain; reuse across instances and sequential tests. */
export class ResourceHost {
  #next = 1n;
  #limit;
  #cleanupErrors = [];
  constructor({ limit } = {}) {
    if (limit !== undefined && typeof limit !== "bigint") throw new TypeError("Resource limit must be bigint");
    this.#limit = limit;
  }
  /** Drain host cleanup diagnostics. Close/teardown themselves remain error-free. */
  takeCleanupErrors() { return this.#cleanupErrors.splice(0); }
  context() { return new ResourceContext(contextKey, () => {
    if (this.#limit !== undefined && this.#next > this.#limit) throw new ResourceError("memory exhausted");
    return this.#next++;
  }, (phase, error) => this.#cleanupErrors.push(Object.freeze({ phase, error }))); }
}

export class ResourceContext {
  #allocate;
  #report;
  #entries = new Map();
  #closing = new Map();
  #live = true;
  #jobs = new Set();
  #requests = new Set();
  #teardown;
  constructor(key, allocate, report) {
    if (key !== contextKey) throw new TypeError("Create contexts through ResourceHost.context()");
    this.#allocate = allocate;
    this.#report = report;
  }
  get live() { return this.#live; }
  #attempt(phase, action) {
    try {
      // Cancellation/wake callbacks have a void result, not a language value.
      // Observe accidental async failures without delaying synchronous revoke.
      Promise.resolve(action()).catch((error) => { this.#report(phase, error); });
    } catch (error) { this.#report(phase, error); }
  }
  /** A genuine AbortSignal whose user listeners cannot interrupt host cleanup. */
  abortController(phase = "cancel") {
    const controller = new AbortController();
    const { signal } = controller;
    const add = signal.addEventListener.bind(signal);
    const remove = signal.removeEventListener.bind(signal);
    const listeners = new WeakMap();
    const wrap = (listener) => {
      if (!listener || (typeof listener !== "function" && typeof listener !== "object")) return listener;
      if (!listeners.has(listener)) {
        const report = this.#report;
        listeners.set(listener, function (event) {
          try {
            if (typeof listener === "function") return listener.call(this, event);
            return listener.handleEvent(event);
          } catch (error) { report(phase, error); }
        });
      }
      return listeners.get(listener);
    };
    const onabort = Object.getOwnPropertyDescriptor(AbortSignal.prototype, "onabort");
    let handler = null;
    Object.defineProperties(signal, {
      addEventListener: { value: (type, listener, options) => add(type, wrap(listener), options) },
      removeEventListener: { value: (type, listener, options) => remove(type, listeners.get(listener) ?? listener, options) },
      onabort: { get: () => handler, set: (value) => {
        handler = typeof value === "function" ? value : null;
        onabort.set.call(signal, wrap(handler));
      } },
    });
    return controller;
  }
  /** Context cancellation authority independent of a Wasm instance. */
  ownRequest(cancel) {
    if (!this.#live) { this.#attempt("cancel", cancel); throw new ResourceError("closed"); }
    const request = { cancel };
    this.#requests.add(request);
    return () => this.#requests.delete(request);
  }
  /** Track independent creation/supervision before starting OS work. */
  cleanupJob() {
    if (!this.#live) throw new ResourceError("closed");
    let finish;
    const receipt = new Promise((resolve) => { finish = resolve; });
    this.#jobs.add(receipt);
    return Object.freeze({ receipt, complete: () => { this.#jobs.delete(receipt); finish(); } });
  }
  /** Cleanup owns host data only and returns a Promise if cleanup is deferred. */
  insert(kind, value, cleanup) {
    const dispose = () => {
      let result;
      try { result = cleanup(value); } catch (error) { this.#report("cleanup", error); return Promise.resolve(); }
      return Promise.resolve(result).catch((error) => { this.#report("cleanup", error); });
    };
    let id;
    try { id = this.#allocate(); } catch (error) { dispose(); throw error; }
    if (!this.#live) { dispose(); throw new ResourceError("closed"); }
    this.#entries.set(id, { kind, value, dispose, leases: new Map() });
    return id;
  }
  acquire(id, kind, lane) {
    const entry = this.#entries.get(id);
    if (!this.#live || !entry || entry.kind !== kind) throw new ResourceError("closed");
    if (entry.leases.has(lane)) throw new ResourceError("busy");
    const abort = this.abortController("wake");
    entry.leases.set(lane, abort);
    const release = () => { if (entry.leases.get(lane) === abort) entry.leases.delete(lane); };
    return Object.freeze({
      value: entry.value,
      signal: abort.signal,
      release,
      commit: (answer) => {
        if (!this.#live || this.#entries.get(id) !== entry || entry.leases.get(lane) !== abort) throw new ResourceError("closed");
        release();
        return answer;
      },
    });
  }
  /** Transfer an idle resource to another owner without running cleanup. */
  takeIdle(id, kind) {
    const entry = this.#entries.get(id);
    if (!this.#live || !entry || entry.kind !== kind) throw new ResourceError("closed");
    if (entry.leases.size) throw new ResourceError("busy");
    this.#entries.delete(id);
    return entry.value;
  }
  // Publish removal and the receipt without calling any embedding code.
  #remove(id, entry) {
    this.#entries.delete(id);
    let complete;
    const receipt = new Promise((resolve) => { complete = resolve; });
    this.#closing.set(id, { kind: entry.kind, receipt });
    return { id, entry, receipt, complete };
  }
  #finishClose({ id, entry, complete }) {
    for (const abort of entry.leases.values()) this.#attempt("wake", () => abort.abort());
    entry.leases.clear();
    entry.dispose().then(() => {
      this.#closing.delete(id);
      complete();
    });
  }
  /** Remove first, then revoke/wake and run cleanup. Reentrant closes join it. */
  close(id, kind) {
    const pending = this.#closing.get(id);
    if (pending) return pending.kind === kind ? pending.receipt : Promise.resolve();
    const entry = this.#entries.get(id);
    if (!entry || entry.kind !== kind) return Promise.resolve();
    const work = this.#remove(id, entry);
    this.#finishClose(work);
    return work.receipt;
  }
  /** Make every entry inaccessible before any cancellation or wake callback. */
  teardown() {
    if (this.#teardown) return this.#teardown;
    this.#live = false;
    const requests = [...this.#requests];
    this.#requests.clear();
    const work = [...this.#entries].map(([id, entry]) => this.#remove(id, entry));
    const receipts = [...this.#closing.values()].map(({ receipt }) => receipt).concat([...this.#jobs]);
    // Publish the join before callbacks, including reentrant teardown calls.
    this.#teardown = Promise.all(receipts).then(() => {});
    for (const { cancel } of requests) this.#attempt("cancel", cancel);
    for (const entry of work) this.#finishClose(entry);
    return this.#teardown;
  }
  /** An explicit drop guard for a not-yet-published resource reply (N9.4). */
  provisional(id, kind) {
    let delivered = false;
    return Object.freeze({
      id,
      deliver: () => {
        if (!this.#live || this.#entries.get(id)?.kind !== kind) throw new ResourceError("closed");
        delivered = true;
        return id;
      },
      discard: () => delivered ? Promise.resolve() : this.close(id, kind),
    });
  }
}
