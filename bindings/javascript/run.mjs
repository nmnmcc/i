#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { instantiate } from './ilang.mjs';
const [file, ...rest] = process.argv.slice(2);
if (!file) throw new Error('usage: run.mjs <module.wasm> [-- args]');
const args = rest[0] === '--' ? rest.slice(1) : rest;
const program = await instantiate(await readFile(file), { standardServices: { args } });
await program.run();
