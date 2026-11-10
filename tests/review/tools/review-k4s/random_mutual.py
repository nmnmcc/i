from probe import write,CLI
import subprocess,itertools,json,random
random.seed(817)
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
T(N) { T(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
tuse(N) = (s ! T(N)) { T(f, _) = s f() }
run(A,N) = (body: () -> A ! N) { body() }
D { D }
'''
def wrap(x,ops):
 for i,o in enumerate(ops):
  if o=='r':x=f'run(() {{ {x} }})'
  elif o=='d':x=f'! D = D {x}'
  elif o=='l':x=f'h{i} = () {{ {x} }} h{i}()'
 return x
rows=[]
for case in range(200):
 decls=[]
 for i in range(3):
  param=random.choice(['s ! S(N)','t ! T(N)','s ! S(N), t ! T(N)',''])
  ann=random.choice(['','-> Nil ! N | S(N) | T(N)'])
  expr=f'f{(i+1)%3}(False)'
  expr=wrap(expr,random.choices('rdl',k=random.randint(0,3)))
  use=random.choice(['use()','tuse()','use() tuse()'])
  decls.append(f'f{i}(N) = (b: Bool'+(', '+param if param else '')+f') {ann} {{ b |> {{ True -> '+expr+f' False -> {{ {use} }} }} }}')
 vals=[]
 for order in [decls,list(reversed(decls))]:
  src=prefix+'\n'.join(order);p=write('rand_mut_tmp',src)
  try:r=subprocess.run([CLI,'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=5);vals.append((r.returncode,r.stdout+r.stderr,src))
  except subprocess.TimeoutExpired:vals.append((99,'TIMEOUT',src))
 if vals[0][0]!=vals[1][0]:
  name=f'rand_mut_{case}';row=dict(name=name,outs=[v[1] for v in vals]);rows.append(row)
  for i,v in enumerate(vals):write(name+'_'+str(i),v[2])
  print(row,flush=True)
open('/tmp/review-k4s/rand-mut-summary.json','w').write(json.dumps(rows,indent=2))
