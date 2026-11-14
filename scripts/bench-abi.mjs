#!/usr/bin/env node
// Run through cache/heavy after abi_performance has emitted v4/v5 O2 artifacts.
import { readFile } from 'node:fs/promises';
import { performance } from 'node:perf_hooks';
import { authorizeBindings, instantiate } from '../bindings/javascript/ilang.mjs';

const files = process.argv.slice(2);
if (!files.length) throw new Error('usage: scripts/bench-abi.mjs module.wasm [module.wasm ...]');
for (const file of files) {
  // These modules and their plans were produced by the local benchmark build.
  const plan = JSON.parse(await readFile(file.replace(/\.wasm$/, '.bindings.json'), 'utf8'));
  const bindings = await authorizeBindings(await readFile(file), plan, () => true);
  const guest = await instantiate(bindings.module, {
    bindings,
    standardServices: true,
    imports: { 'abi:bench': {
      tick: n => n + 1n,
      read: n => {
        if (n !== 8n) throw new Error('read length');
        return { bytes: new Uint8Array(8), bitLength: 64 };
      },
    } },
  });
  for (const [name, args, expected] of [
    ['counter', [1000000n, 0n], 1000000n],
    ['read_lengths', [1000000n, 8n], 8n],
  ]) {
    const samples = [];
    for (let sample = 0; sample < 6; sample++) {
      const start = performance.now();
      const result = guest.call(`abiperf/main.${name}`, args);
      const elapsed = (performance.now() - start) / 1000;
      if (result !== expected) throw new Error(`${name}: ${result}`);
      if (sample) samples.push(elapsed);
    }
    samples.sort((a, b) => a - b);
    console.log(`ABI_PERF_JS version=${guest.abi.version} ${name}: median ${samples[2].toFixed(6)} s / 1e6 calls; samples ${JSON.stringify(samples)}`);
  }
}
