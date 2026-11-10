from harness import *
headers={
 'pattern':'S(N) { S(() -> Nil ! N) }\nuse(N) = (s ! S(N)) { S(f) = s f() }',
 'field':'S(N) { S(f: () -> Nil ! N) }\nuse(N) = (s ! S(N)) { s.f() }',
 'field_annot':'S(N) { S(f: () -> Nil ! N) }\nuse(N) = (s ! S(N)) -> Nil ! N | S(N) { s.f() }',
}
for h,header in headers.items():
 for wi,body in enumerate([
  'helper(N) = () { run(() { use() }) }',
  'helper(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }',
  'helper(N) = () { local: () -> Nil ! N | S(N) = () { run(() { use() }) } run(local) }',
  'helper(N) = () { run(() -> Nil ! N | S(N) { use() }) }',
  'helper(N) = () -> Nil ! N | S(N) { run(() { use() }) }',
 ]):
  for b in [False,True]:
   r=check(f'finite_{h}_{wi}',header+'\nrun(N) = (body: () -> Nil ! N) { body() }\n'+body,b)
   print(h,wi,'base' if b else 'lane',r,flush=True)
