from probe import write,CLI
import subprocess,itertools,json
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
D { D }
'''
helpers=[
'run(A,N) = (body: () -> A ! N) { body() }',
'run(A,N) = (body: () -> A ! N) -> A ! N { body() }',
'run(A,N): (body: () -> A ! N) -> A ! N = (body) { body() }',
'run(A,N): (body: () -> A ! N) -> A ! N = (body: () -> A ! N) -> A ! N { body() }',
'run = (body) { body() }',
'run(N) = (body: () -> Nil ! N) { body() }',
'run(N) = (body: () -> Nil ! N) -> Nil ! N { body() }',
'run(N): (body: () -> Nil ! N) -> Nil ! N = (body) { body() }',
]
bodies=[
'run(() { use() })',
'run(() -> Nil ! N | S(N) { use() })',
'! D = D run(() { use() })',
'run(() { ! D = D use() })',
'local = () { use() } run(local)',
'local: () -> Nil ! N | S(N) = () { use() } run(local)',
'local = run local(() { use() })',
]
rows=[]
for k,b in enumerate(bodies):
 for ret in ['', '-> Nil ! N | S(N)']:
  vals=[]
  for i,h in enumerate(helpers):
   src=prefix+h+'\n'+f'forward(N) = (s ! S(N)) {ret} {{ {b} }}'
   p=write('helper_ann_tmp',src)
   r=subprocess.run([CLI,'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=5)
   vals.append((r.returncode,r.stdout+r.stderr,src))
  if len(set(v[0] for v in vals[:4]))>1 or len(set(v[0] for v in vals[5:]))>1:
   row=dict(body=k,ret=ret,outs=[v[1] for v in vals]);rows.append(row)
   for i,v in enumerate(vals):write('helper_ann_'+str(k)+'_'+str(len(ret))+'_'+str(i),v[2])
   print(row,flush=True)
open('/tmp/review-k4s/helper-ann-summary.json','w').write(json.dumps(rows,indent=2))
