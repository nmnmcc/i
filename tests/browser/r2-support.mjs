// The same completion-gated requirements run in Node and all browser engines.
// No timer determines when a producer settles. The guest's release operation
// selects the answer; id 9 is ready at the first host opportunity.
export async function checkR2(sdk, bytes, plan, repetitions = 200) {
  const require = (condition, message) => { if (!condition) throw new Error(message); };
  const bindings = await sdk.authorizeBindings(bytes, plan, () => true);
  async function guest() {
    const gates = [];
    const value = await sdk.instantiate(bindings.module, { bindings, contextImports: {
      "r2:gates/v1": { wait(context, id) {
        require(context.live, "start belongs to a live invocation");
        const gate = { id, context, signal: sdk.requestSignal(), completed: false };
        gates.push(gate);
        return new Promise((resolve) => {
          gate.release = () => { gate.completed = true; resolve(id); };
          if (id === 9n) gate.release();
        });
      } },
      "r2:control/v1": {
        release(context, id) {
          require(context.live, "release context live");
          require(gates.length === 2 && gates[0].id === 1n && gates[1].id === 2n, "both starts precede release");
          for (const gate of gates) if (id === 0n || gate.id === id) gate.release();
        },
        check(context, phase) {
          require(context.live, "check context live");
          const first = gates[0];
          if (phase === 1n) require(first.signal.aborted, "owned cancellation is immediate");
          else if (phase === 2n || phase === 3n) require(!first.signal.aborted, "borrowed producer survives waiter cancellation");
          else require(phase === 4n, "known check phase");
        },
      },
    } });
    return { value, gates };
  }
  const results = [];
  for (let repetition = 0; repetition < repetitions; ++repetition) {
  for (const [name, expected] of [["overlap", 3n], ["unlisted_answer", 3n], ["cancel_owned", undefined], ["cancel_borrowed", undefined], ["race_read", 2n], ["nested_interest", 9n], ["raw_only", 7n]]) {
    const { value, gates } = await guest();
    require(await value.callAsync(`app/r2.${name}`) === expected, `${name} value`);
    require(gates.length > 0 && gates.every((g) => !g.context.live), `${name} closes invocation`);
    require(gates.filter((g) => !g.completed).every((g) => g.signal.aborted), `${name} cancels leftovers`);
    if (repetition === 0) results.push(name);
  }
  for (const name of ["owned_idle", "unrelated_idle"]) {
    const { value, gates } = await guest();
    const pending = value.callAsync(`app/r2.${name}`);
    // An unrelated producer cannot cause even the initial host handoff. Stop
    // tears down its context before the first Promise continuation is queued.
    require(gates.length === 1, "one raw operation started");
    require(gates[0].signal.aborted === (name === "unrelated_idle"), `${name} initial Root decision`);
    let stopped = false;
    try { await pending; } catch (error) {
      stopped = error instanceof sdk.Stop && error.stopMessage.split("\n")[0] === "panic: task: deadlock";
    }
    require(stopped && !gates[0].context.live, `${name} quiescent deadlock`);
    if (repetition === 0) results.push(name);
  }
  }
  const { value } = await guest();
  const meta = value.abi.exports;
  for (const name of ["cmp", "cmp2", "expected_trace", "fake", "sync_poll"]) require(!meta[`app/r2.${name}`].async, `${name} stays synchronous`);
  for (const name of ["pure_tasks", "trace", "overlap"]) require(meta[`app/r2.${name}`].async, `${name} task/request entry is async`);
  require(value.call("app/r2.cmp2", [3, 2]) === 1, "pure callback through wrapper");
  require(value.call("app/r2.fake") === 7n, "settled fake needs no host entry");
  const expected = value.call("app/r2.expected_trace", [-7n]);
  for (let iteration = 0; iteration < repetitions; ++iteration) {
    require(await value.callAsync("app/r2.trace", [-7n]) === expected, `replay ${iteration}`);
  }
  const immediate = globalThis.setImmediate;
  let macrotasks = 0;
  globalThis.setImmediate = (resolve) => { ++macrotasks; queueMicrotask(resolve); };
  try { require(await value.callAsync("app/r2.pure_tasks") === 7n, "pure tasks finish"); }
  finally {
    if (immediate === undefined) delete globalThis.setImmediate;
    else globalThis.setImmediate = immediate;
  }
  require(macrotasks === 0, "pure task switches make no macrotask request");
  let inactive = false;
  try { value.call("app/r2.sync_poll"); } catch (error) {
    inactive = error instanceof sdk.Stop && error.stopMessage.startsWith("panic: request: no active entry\n");
  }
  require(inactive, "stale async root does not authorize synchronous poll");
  return { results, expected, repetitions };
}
