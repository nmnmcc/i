#!/usr/bin/env node
import { WASI } from 'node:wasi';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

/** Runs an i preview-1 command with explicit args, environment and preopens. */
export async function runWasi(file, { args = [], env = {}, preopens = {}, stdout = 1, stderr = 2, stdin = 0 } = {}) {
  const wasi = new WASI({ version: 'preview1', args: [file, ...args], env, preopens, stdout, stderr, stdin, returnOnExit: true });
  const module = await WebAssembly.compile(await readFile(file));
  const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
  return wasi.start(instance);
}
if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [file, options = '{}'] = process.argv.slice(2);
  process.exitCode = await runWasi(file, JSON.parse(options));
}
