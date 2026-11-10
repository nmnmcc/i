from harness import *
headers=[
 'S(N) { S(f: () -> Nil ! N) }\nuse(N) = (s ! S(N)) { s.f() }',
 'S(N) { S(f: () -> Nil ! N, mark: (N) -> Nil) }\nuse(N) = (s ! S(N)) { s.f() }',
 'S(E,N) { S(f: () -> Nil ? E ! N) }\nuse(E,N) = (s ! S(E,N)) { s.f() }',
 'S(A,B) { S(f: () -> Nil ! A, g: () -> Nil ! B) }\nuse(A,B) = (s ! S(A,B)) { s.f() s.g() }',
]
for hi,header in enumerate(headers):
 declarations='Dep { Dep }\nget = (d ! Dep) { Nil }\nrun(E,N) = (body: () -> Nil ? E ! N) { body() }\n'+header+'\n'
 value=['S(() { get() })','S(() { get() }, (_: Dep) { Nil })','S(() { get() })','S(() { get() }, () { get() })'][hi]
 supplied=['S(Dep)','S(Dep)','S(Never,Dep)','S(Dep,Dep)'][hi]
 wrap_decl=['N','N','E,N','A,B'][hi]
 effect=['Nil ! N | S(N)','Nil ! N | S(N)','Nil ? E ! N | S(E,N)','Nil ! A | B | S(A,B)'][hi]
 for wi,wrap in enumerate([
  'use()',
  'run(() { use() })',
  'local = () { use() } local()',
  f'local: () -> {effect} = () {{ use() }} local()',
  'local: () -> Nil ! N = () { use() } local()' if hi<3 else 'local: () -> Nil ! A | B = () { use() } local()',
  f'run(() -> {effect} {{ use() }})',
  f'local: () -> {effect} = () {{ run(() {{ use() }}) }} run(local)',
  'local = () { inner = () { use() } inner() } run(local)',
 ]):
  for si,statement in enumerate(['',f'! {supplied} = {value}\n']):
   for pi,position in enumerate(['main','helper','closure']):
    if position=='main':body=f'@main\nmain({wrap_decl}) = () {{ {statement}{wrap} }}'
    elif position=='helper':body=f'helper({wrap_decl}) = () {{ {statement}{wrap} }}\n@main\nmain = () {{ helper() }}'
    else:body=f'@main\nmain({wrap_decl}) = () {{ local2 = () {{ {statement}{wrap} }} run(local2) }}'
    name=f'missing_{hi}_{wi}_{si}_{pi}'
    r=check(name,declarations+body)
    if r['code']==0:
     print('ACCEPTED',name,r,flush=True)
     for opt in [0,2]:
      try:print('RUN',opt,run(name,opt),flush=True)
      except Exception as e:print('RUN ERROR',e,flush=True)
    elif 'E0505' in r.get('err',''):print('RECURSIVE',name,r['err'][:180],flush=True)
