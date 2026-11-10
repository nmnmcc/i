from probe import check
from pathlib import Path
import subprocess,json
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
'''
variants=[
('direct','(s ! S(N)) ANNOT { S(f, _) = s f() }'),
('wrap','(s ! S(N)) ANNOT { run(() { use() }) }'),
('arg','(f: () -> Nil ! N, s ! S(N)) ANNOT { run(f) }'),
('local','(s ! S(N)) ANNOT { helper = () { use() } run(helper) }'),
('capture','(f: () -> Nil ! N, s ! S(N)) ANNOT { helper = () { f() use() } run(helper) }'),
('install','(s ! S(N)) ANNOT { ! D = D run(() { use() }) }'),
('poly','(s ! S(N)) ANNOT { a = () { use() } b = () { a() } run(b) }'),
('nested','(s ! S(N)) ANNOT { run(() { run(() { run(() { use() }) }) }) }'),
('local_install','(s ! S(N)) ANNOT { run(() { ! D = D use() }) }'),
]
results=[]
for name,body in variants:
 for i,ann in enumerate(['','-> Nil ! N | S(N)']):
  r=check('ann_'+name+str(i),prefix+'test(N) = '+body.replace('ANNOT',ann));results.append(r)
# Invariant values link E, N parameters across uses
for name,body in [
('twice','test(N) = (s ! S(N)) ANNOT { use() use() }'),
('different','test(N,M) = (f: () -> Nil ! N, g: () -> Nil ! M) ANNOT { f() g() }')]:
 for i,ann in enumerate(['','-> Nil ! N | S(N)' if name=='twice' else '-> Nil ! N | M']):
  check('ann_'+name+str(i),prefix+body.replace('ANNOT',ann))
