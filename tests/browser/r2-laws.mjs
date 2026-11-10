// Execute every runnable R2 spec body. The generator adapts only the entry
// attribute/export wrapper; expected messages and output come from source.
export async function checkR2Laws(sdk, bytes, plan, cases) {
  const bindings = await sdk.authorizeBindings(bytes, plan, () => true);
  let passed = 0;
  for (const item of cases) {
    const printed = [];
    const contextImports = {};
    const implementations = {
      "test:io/v1": { print(context, text) {
        if (!context.live) throw new Error("ended print context");
        printed.push(text);
      } },
      "test:async/v1": {
        double(context, n) {
          if (!context.live) throw new Error("ended double context");
          printed.push(`double ${n}`);
          return n < 0n ? Promise.reject(new sdk.GuestError("negative")) : Promise.resolve(2n * n);
        },
        echo(context, text) {
          if (!context.live) throw new Error("ended echo context");
          printed.push(`echo ${text}`);
          return Promise.resolve(text);
        },
      },
    };
    for (const namespace of new Set(plan.declarations.map((d) => d.namespace))) {
      const suffix = ["test:io/v1", "test:async/v1"].find((name) => namespace.endsWith(name));
      if (!suffix) throw new Error(`unknown conformance service ${namespace}`);
      contextImports[namespace] = implementations[suffix];
    }
    const value = await sdk.instantiate(bindings.module, { bindings, contextImports });
    if (Object.keys(value.abi.exports).length !== cases.length) throw new Error("case/export coverage differs");
    let thrown;
    try {
      if (value.abi.exports[item.entry].async) await value.callAsync(item.entry);
      else value.call(item.entry);
    } catch (error) { thrown = error; }
    if (item.expectation === "pass") {
      if (thrown) throw new Error(`${item.name}: ${thrown.message}`, { cause: thrown });
    } else {
      const expected = JSON.parse(item.expectation.slice("stop ".length));
      if (!(thrown instanceof sdk.Stop) || thrown.stopMessage.split("\n")[0] !== expected) {
        throw new Error(`${item.name}: expected ${expected}; got ${thrown?.message ?? "normal return"}`);
      }
    }
    if (item.output !== null && JSON.stringify(printed) !== JSON.stringify(item.output)) throw new Error(`${item.name}: output ${JSON.stringify(printed)}; expected ${JSON.stringify(item.output)}`);
    ++passed;
  }
  return passed;
}
