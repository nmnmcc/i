from harness import *
import random
random.seed(412)
fixtures={
 'mutual':[
 'S(N) { S(() -> Nil ! N) }',
 'use(N) = (s ! S(N)) { S(f) = s f() }',
 'a = (x: Bool) { x |> { True -> use() False -> b(True) } }',
 'b = (x: Bool) { x |> { True -> use() False -> a(True) } }',
 'run(N) = (body: () -> Nil ! N) { body() }',
 'wrap = () { run(() { a(True) }) }',
 ],
 'mutual_annot':[
 'S(N) { S(() -> Nil ! N, (N) -> Nil) }',
 'use(N) = (s ! S(N)) { S(f, _) = s f() }',
 'a(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> b(True) } }',
 'b(N) = (x: Bool) -> Nil ! N | S(N) { x |> { True -> use() False -> a(True) } }',
 'run(N) = (body: () -> Nil ! N) { body() }',
 'wrap = () { run(() { a(True) }) }',
 ],
 'multi_instance':[
 'Dep { Dep }',
 'S(A,N) { S((A) -> A ! N) }',
 'use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }',
 'run(A,N) = (body: () -> A ! N) { body() }',
 'wrap(A,N) = (x: A) { local = () { use(x) } run(local) }',
 'ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }',
 'texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }',
 ],
 'hidden_conflict':[
 'S(A) { S(A) }',
 'one = (s ! S(Int)) { Nil }',
 'two = (s ! S(String)) { Nil }',
 'run(N) = (body: () -> Nil ! N) { body() }',
 'bad = () { run(() { one() }) run(() { two() }) }',
 ],
 'recursive':[
 'S(N) { S(() -> Nil ! N, (N) -> Nil) }',
 'use(N) = (s ! S(N)) { S(f, _) = s f() }',
 'a(N) = () { ! S(N) = S(() { b() }, (_: N) { Nil }) use() }',
 'b = () { a() }',
 ],
}
for name,defs in fixtures.items():
 results={}
 for i in range(32):
  order=list(defs)
  if i:random.shuffle(order)
  r=check(f'order_{name}_{i}','\n'.join(order))
  signature=(r['code'],tuple(sorted(r.get('out','').splitlines())),tuple(sorted(__import__('re').findall(r'error\[(.*?)\]',r.get('err','')))))
  results.setdefault(str(signature),[]).append(i)
 print(name,len(results),results,flush=True)
