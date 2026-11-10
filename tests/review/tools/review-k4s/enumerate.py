from probe import write,CLI
from itertools import product
import subprocess,json
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
'''
def wrap(op,x,k):
 if op=='r': return f'run(() {{ {x} }})'
 if op=='a': return f'run(() -> Nil ! N | S(N) {{ {x} }})'
 if op=='d': return f'! D = D\n {x}'
 if op=='l': return f'h{k} = () {{ {x} }}\n h{k}()'
 if op=='t': return f'h{k}: () -> Nil ! N | S(N) = () {{ {x} }}\n h{k}()'
 if op=='u': return 'use()\n'+x
rows=[]
for depth in range(1,4):
 for ops in product('radltu',repeat=depth):
  vals=[]
  for ann in ['', '-> Nil ! N | S(N)']:
   x='use()'
   for k,op in enumerate(ops):x=wrap(op,x,k)
   src=prefix+f'test(N) = (s ! S(N)) {ann} {{ {x} }}'
   p=write('enum_case',src)
   r=subprocess.run([CLI,'check','--project',str(p),'--offline','--types'],capture_output=True,text=True,timeout=5)
   vals.append((r.returncode,r.stdout+r.stderr,src))
  if vals[0][0]!=vals[1][0]:
   row=dict(ops=''.join(ops),codes=[v[0] for v in vals],outs=[v[1] for v in vals]);rows.append(row)
   for i,v in enumerate(vals): write('enum_'+''.join(ops)+'_'+str(i),v[2])
   print(row,flush=True)
open('/tmp/review-k4s/enum-summary.json','w').write(json.dumps(rows,indent=2))
