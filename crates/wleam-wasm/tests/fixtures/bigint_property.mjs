import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]));
const e=instance.exports;
const mem=new DataView(e.memory.buffer);
function write(n,ptr){ const sign=n<0n?-1:n>0n?1:0; if(n<0n)n=-n;let len=0;while(n){mem.setUint32(ptr+len*4,Number(n&0xffffffffn),true);n>>=32n;len++;}return [len,sign]; }
function read(sign){let n=0n; const len=mem.getUint32(12288,true); for(let i=len-1;i>=0;i--)n=(n<<32n)|BigInt(mem.getUint32(8192+i*4,true));assert.equal(sign===0,n===0n);if(sign===0)assert.equal(len,0);return sign<0?-n:n;}
function calc(op,a,b){ const aa=write(a,0);const bb=write(b,4096);return read(e.compute(op,...aa,...bb)); }
let state=0x7c19ac49; function rnd(){state^=state<<13;state^=state>>>17;state^=state<<5;return state>>>0;} function randBig(){let n=0n;const len=rnd()%17;for(let i=0;i<len;i++)n=(n<<32n)|BigInt(rnd());return rnd()&1?-n:n;}
const ops=[(a,b)=>a+b,(a,b)=>a-b,(a,b)=>a*b,(a,b)=>b===0n?0n:a/b,(a,b)=>b===0n?0n:a%b,(a,b)=>a&b,(a,b)=>a|b,(a,b)=>a^b,(a)=>~a,(a,b)=>a<<b,(a,b)=>a>>b];
const boundaries=[0n,1n,-1n,10n,-10n,31n,-31n,32n,-32n,33n,-33n,36n,-36n,4294967295n,-4294967295n,(1n<<30n)-1n,1n<<30n,-(1n<<30n),-(1n<<30n)-1n,(1n<<31n)-1n,1n<<31n,(1n<<32n)-1n,1n<<32n,(1n<<63n)-1n,1n<<63n,-(1n<<63n),1n<<64n,(1n<<128n)-1n,1n<<128n,-(1n<<128n)];
let cases=0;
function check(a,b){for(let op=0;op<ops.length;op++){let bb=op>=9?BigInt(Number(b%300n)):b;const expected=ops[op](a,bb);const actual=calc(op,a,bb);assert.equal(actual,expected,`op ${op}: ${a} and ${bb}`);cases++;}}
for(const a of boundaries)for(const b of boundaries)check(a,b);
for(let i=0;i<2000;i++)check(randBig(),randBig());
for(let i=0;i<300;i++){const a=randBig();const args=write(a,0);assert.equal(e.to_float(...args),Number(a));for(const base of [2,8,10,16,36]){const n=e.to_string(...args,base);const text=Buffer.from(e.memory.buffer,8192,n).toString('utf8');assert.equal(text,a.toString(base).toUpperCase());cases++;}}
for(const x of [0,-0,1,-1,0.1,-0.1,1.999,-1.999,2**30,-(2**30),2**53+2,2**63,-(2**63),1e100,-1e100,Number.MIN_VALUE,Number.MAX_VALUE,-Number.MAX_VALUE]){assert.equal(read(e.from_float(x)),BigInt(Math.trunc(x)),`fromfloat ${x}`);cases++;}
console.log(`Passed ${cases} bigint arithmetic/bitwise/conversion comparisons`);
