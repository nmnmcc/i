from harness import *
source='''S(A) { S(f: (A) -> Int) }
use(A) = (x: A, s ! S(A)) { s.f(x) }
wrap(A) = (x: A) { local = () { use(x) } local() }
'''
for i,extra in enumerate([
 'same(A): (A) -> Int ! S(A) = wrap',
 'diff(A,B): (A) -> Int ! S(B) = wrap',
 'same = (f: (Int) -> Int ! S(Int)) { f }\ntest = same(wrap)',
 'different = (f: (Int) -> Int ! S(String)) { f }\ntest = different(wrap)',
]):
 for b in [False,True]:print(i,b,check(f'annotation_{i}',source+extra,b))
