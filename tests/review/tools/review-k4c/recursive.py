from harness import *
for variant,decl,value in [('co','S(N) { S(() -> Nil ! N) }','S(() { use() })'),('inv','S(N) { S(() -> Nil ! N, (N) -> Nil) }','S(() { use() }, (_: N) { Nil })')]:
 use='use(N) = (s ! S(N)) { '+('S(f) = s' if variant=='co' else 'S(f, _) = s')+' f() }'
 for i,body in enumerate([
  'bad(N) = (s ! S(N)) -> Nil ! N { '+('S(f) = s' if variant=='co' else 'S(f, _) = s')+' f() }',
  f'bad(N) = () {{ ! S(N) = {value} use() }}',
  f'bad(N) = () {{ local = () {{ ! S(N) = {value} use() }} local() }}',
  f'bad(N) = () {{ ! S(N) = {value} local = () {{ use() }} local() }}',
  f'bad(N) = () {{ ! S(N) = {value} run(() {{ use() }}) }}',
  'bad(N) = (s ! S(N)) -> Nil ! N { local = () { use() } local() }',
 ]):
  src=decl+'\n'+use+'\nrun(N) = (f: () -> Nil ! N) { f() }\n'+body
  for b in [False,True]:
   r=check(f'recursive_{variant}_{i}',src,b)
   print(variant,i,'base' if b else 'lane',r,flush=True)
