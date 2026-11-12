// Export artifacts with NUMERIC_WASM_DUMP when running the Rust numeric corpus.
// node tests/perf/numeric/compare-js.mjs [artifact-directory]
import {readFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
import {instantiate,Stop} from '../../../bindings/javascript/ilang.mjs';
const base = pathToFileURL(resolve(process.argv[2] ?? 'target/numeric-step2/js-corpus') + '/');
const {cases,observations} = JSON.parse(await readFile(new URL('cases.json',base),'utf8'));
for (const filename of ['o0-Reference.wasm','o2-Visible.wasm','o2-Scalars.wasm','o3-Scalars.wasm']) {
 const module = await WebAssembly.compile(await readFile(new URL(filename,base)));
 for (let i=0;i<cases.length;i++) {
  const c=cases[i], events=[];
  const guest = await instantiate(module,{imports:{'numeric:events':{record:(value)=>{events.push(value);return null;}}}});
  let input;
  if(c.input.kind==='int') input=BigInt(c.input.value);
  else { const d=new DataView(new ArrayBuffer(8));d.setBigUint64(0,BigInt(c.input.bits));input=d.getFloat64(0); }
  let value;
  try { value=guest.call(c.export,[input]);assert.equal(c.stop,false,`${filename} case ${i} did not stop`); }
  catch(error) { assert.equal(error instanceof Stop,true,`${filename} case ${i}: ${error.stack}`);assert.equal(c.stop,true);value=error.stopMessage; }
  finally { guest.dispose(); }
  assert.deepEqual([value,events],observations[i],`${filename} case ${i}`);
 }
 console.log(`${filename}: ${cases.length} values, effects and complete stop traces agree with native O0`);
}
