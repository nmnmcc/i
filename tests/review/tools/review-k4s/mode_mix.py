from probe import check
p='''S(A) { S(value: A) }
one = (s ! S(Int)) { s.value + 1 }
two = (s ! S(String)) { s.value <> "!" }
'''
for order in [0,1]:
 check('mixed_order_'+str(order),p+('''run(N) = (x: N, f: () -> Int ! N) { f() }
bad = () { run(S("hi"), () { one() }) }
''' if order==0 else '''run(N) = (f: () -> Int ! N, x: N) { f() }
bad = () { run(() { one() }, S("hi")) }
'''))
for order in [0,1]:
 for ann in ['', '-> Int ! N | S(N)']:
  check('mixed_recursive_'+str(order)+'_'+str(len(ann)),'''S(N) { S(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
D { D }
run(N) = (x: N, f: () -> Int ! N) { f() }
bad(N) = (x: N, s ! S(N)) ANNOT { run(x, () { use() }) }
'''.replace('ANNOT',ann))
