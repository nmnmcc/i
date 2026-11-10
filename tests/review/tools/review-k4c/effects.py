from harness import *
heads={
 'co':'S(A,E,N) { S((A) -> A ? E ! N) }',
 'inv':'S(A,E,N) { S((A) -> A ? E ! N, (N) -> Nil) }',
 'two':'S(A,E,N,M) { S((A) -> A ? E ! N, () -> Nil ! M) }',
}
for h,header in heads.items():
 params='A,E,N,M' if h=='two' else 'A,E,N'
 svc=f'S({params})'
 eff=f'A ? E ! N | M | {svc}' if h=='two' else f'A ? E ! N | {svc}'
 use=f'use({params}) = (x: A, s ! {svc}) {{ '+('S(f, g) = s g() f(x)' if h=='two' else ('S(f, _) = s f(x)' if h=='inv' else 'S(f) = s f(x)'))+' }'
 common=header+'\n'+use+'\nrun(A,E,N) = (body: () -> A ? E ! N) { body() }\n'
 for i,body in enumerate([
  'use(x)',
  'run(() { use(x) })',
  'local = () { use(x) } local()',
  f'local: () -> {eff} = () {{ use(x) }} local()',
  'local = () { run(() { use(x) }) } local()',
  f'local: () -> {eff} = () {{ run(() {{ use(x) }}) }} local()',
  'local = () { inner = () { use(x) } inner() } run(local)',
  f'run(() -> {eff} {{ use(x) }})',
 ]):
  for a,ann in enumerate(['',f'-> {eff}']):
   source=common+f'wrap({params}) = (x: A) {ann} {{ {body} }}'
   name=f'effects_{h}_{i}_{a}'
   r=check(name,source)
   b=check(name,source,True)
   if r['code']!=0 or (b['code']==0 and r['out']!=b['out']):print(name,'LANE',r,'BASE',b,flush=True)
