from probe import write,CLI
import subprocess,json
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (f: () -> A ! N) { f() }
'''
variants=[
'@main\nmain(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } run(local) }',
'@main\nmain(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } local() }',
'@main\nmain(N) = (s ! S(N)) { local = () -> Nil ! N | S(N) { use() } local() }',
'@main\nmain(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { use() } Nil }',
'@main\nmain(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { Nil } local() }',
'@main\nmain(N) = (s ! S(N)) { local: () -> Nil ! N | S(N) = () { Nil } Nil }',
'@main\nmain(N) = (s ! S(N)) { run(() { use() }) }',
'@main\nmain(N) = () { local: () -> Nil ! N | S(N) = () { use() } run(local) }',
'@main\nmain(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }',
]
rows=[]
for i,v in enumerate(variants):
 for marker in [True,False]:
  name=f'min_main_{i}_{int(marker)}';src=prefix+v
  if not marker:src=src.replace(', (N) -> Nil','').replace('S(f, _)','S(f)')
  if 'run(' not in v:src=src.replace('run(A,N) = (f: () -> A ! N) { f() }\n','')
  p=write(name,src)
  r=subprocess.run([CLI,'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=10)
  row=dict(name=name,code=r.returncode,out=r.stdout+r.stderr);rows.append(row);print(row,flush=True)
open('/tmp/review-k4s/min-main-summary.json','w').write(json.dumps(rows,indent=2))
