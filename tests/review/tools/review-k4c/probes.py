from harness import *
cases={}
for shape,decl,use in [
 ('co','S(A) { S(value: A) }','use(A) = (s ! S(A)) { s.value }'),
 ('contra','S(A) { S(f: (A) -> Nil) }','use(A) = (x: A, s ! S(A)) { s.f(x) }'),
 ('inv','S(A) { S(f: (A) -> A) }','use(A) = (x: A, s ! S(A)) { s.f(x) }'),
 ('bi','S(A) { S }','use(A) = (s ! S(A)) { Nil }'),
 ('row','S(A) { S(f: () -> Nil ! A) }','use(A) = (s ! S(A)) { s.f() }'),
]:
 call='use(x)' if shape in ['contra','inv'] else 'use()'
 for wrap,body in {
  'direct':f'wrap(A) = (x: A) {{ {call} }}',
  'local':f'wrap(A) = (x: A) {{ local = () {{ {call} }} local() }}',
  'deep':f'wrap(A) = (x: A) {{ local = () {{ inner = () {{ {call} }} inner() }} local() }}',
  'escape':f'wrap(A) = (x: A) {{ local = () {{ {call} }} local }}',
  'callback':f'wrap(A) = (x: A, f) {{ f(() {{ {call} }}) }}',
  'higher':f'wrap(A) = (x: A, f) {{ local = () {{ f(() {{ {call} }}) }} local() }}',
  'nested':f'wrap(A) = (x: A, f) {{ local = () {{ f() }} {call} local() }}',
 }.items():cases[f'{shape}_{wrap}']=decl+'\n'+use+'\n'+body
for n,body in enumerate([
 'wrap = (f) { g = () { f() } g }',
 'wrap = (f) { g = () { f() } (g, f) }',
 'wrap = (f) { g = (x) { f(x) } g }',
 'wrap = (f) { g = (x) { f(x) } (g, f) }',
 'wrap = (f) { g = () { h = () { f() } h } g }',
 'wrap(A,N) = (f: () -> A ! N) { g = () { f() } g }',
 'wrap(A,N) = (f: () -> A ! N) { g: () -> A ! N = () { f() } g }',
]): cases[f'plain_{n}']=body
for name,src in cases.items():
 a=check(name,src)
 if BASE.exists():
  b=check(name,src,True)
  if (a['code'],a['out'])!=(b['code'],b['out']):print(name,'LANE',a,'BASE',b)
 else:print(name,a)
