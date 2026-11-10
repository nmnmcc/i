from pathlib import Path
import re,json,subprocess,sys
from measure import instance
w=Path(__file__).resolve().parent
level=sys.argv[1]; src=w/('o'+level+'.wat')
lines=src.read_text().splitlines(); metrics={}; out=[]; active=False; pending=None
functions={}; current=[]; number=None
def tick(key):
 n=metrics.setdefault(key,len(metrics))
 return [f'    global.get $prof_{n}', '    i64.const 1','    i64.add',f'    global.set $prof_{n}']
for line in lines:
 found=re.match(r'  \(func (?:\$[^ ]+ )?\(;(\d+);\)',line)
 if found:
  active=True; number=found[1]; pending='function:'+number; current=[line];out.append(line);continue
 if active:
  current.append(line)
  if pending and not line.lstrip().startswith('(local '):
   out+=tick(pending);pending=None
  op=line.strip().split(' ')[0]
  if op.startswith(('struct.new','array.new')): out+=tick('allocation:'+line.strip())
  elif op in ['f64.add','f64.sub','f64.mul','f64.div','f64.neg','call_ref','return_call_ref','ref.i31','i31.get_s','ref.test']:
   out+=tick('operation:'+op)
  elif op in ['array.get','struct.get','ref.cast','ref.as_non_null']:out+=tick('access:'+op)
 out.append(line)
 if active and line=='  )':
  active=False;functions[number]='\n'.join(current)
pos=next(i for i,l in enumerate(out) if l.startswith('  (export '))
out[pos:pos]=[f'  (global $prof_{n} (mut i64) (i64.const 0))' for n in metrics.values()]
extra=[f'  (func (export "counter_{n}") (result i64) global.get $prof_{n})' for n in metrics.values()]
extra+=['  (func (export "reset")']
for n in metrics.values(): extra+=['    i64.const 0',f'    global.set $prof_{n}']
extra+=['  )'];out[-1:-1]=extra
counted=w/('counted'+level+'.wat');counted.write_text('\n'.join(out)+'\n')
binary=w/('counted'+level+'.wasm')
subprocess.run(['wasm-tools','parse',str(counted),'-o',str(binary)],check=True)
s,e=instance(binary); os,oe=instance(w/('o'+level+'.wasm'))
result={}
for name in json.loads((w/'exports.json').read_text()):
 e['reset'](s);v=e['num/kernels.'+name](s,.5)
 assert v==oe['num/kernels.'+name](os,.5)
 counts={k:e['counter_'+str(n)](s) for k,n in metrics.items()};result[name]={'value':v,'counts':counts}
 hot=sorted([(v,k.split(':')[1]) for k,v in counts.items() if k.startswith('function:') and v],reverse=True)
 (w/(name+'-o'+level+'-hot.wat')).write_text('\n\n'.join(';; executed '+str(v)+' times\n'+functions[n] for v,n in hot[:12]))
 print(name, v, {prefix:sum(v for k,v in counts.items() if k.startswith(prefix+':')) for prefix in ['allocation','function','access']},flush=True)
(w/('counts'+level+'.json')).write_text(json.dumps(result,indent=2))
