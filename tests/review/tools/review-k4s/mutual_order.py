from probe import check,write,CLI
from itertools import permutations,product
import subprocess,json
s='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
D { D }
'''
results=[]
for param in ['', 's ! S(N)']:
 for ann in ['', '-> Nil ! N | S(N)']:
  for ops in product('rdl',repeat=2):
   def wrap(x):
    for i,o in enumerate(ops):
     x= {'r':lambda:f'run(() {{ {x} }})','d':lambda:f'! D = D {x}','l':lambda:f'h{i} = () {{ {x} }} h{i}()'}[o]()
    return x
   ps=('b: Bool'+(', '+param if param else ''))
   bodies=[f'first(N) = ({ps}) {ann} {{ b |> {{ True -> '+wrap('second(False)')+' False -> use() } }',f'second(N) = ({ps}) {ann} {{ b |> {{ True -> '+wrap('first(False)')+' False -> use() } }']
   vals=[]
   for i,decls in enumerate(permutations(bodies)):
    src=s+'\n'.join(decls);p=write('mut_ord_tmp',src)
    r=subprocess.run([CLI,'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=10)
    vals.append((r.returncode,r.stdout+r.stderr,src))
   if vals[0][0]!=vals[1][0]:
    name='mut_ord_'+str(len(results));row=dict(name=name,param=param,ann=ann,ops=ops,outs=[v[1] for v in vals]);results.append(row)
    for i,v in enumerate(vals):write(name+'_'+str(i),v[2])
    print(row,flush=True)
open('/tmp/review-k4s/mut-order-summary.json','w').write(json.dumps(results,indent=2))
