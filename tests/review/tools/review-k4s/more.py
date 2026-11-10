from probe import check
prefix='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A, N) = (body: () -> A ! N) { body() }
D { D }
'''
for name,body in [
 ('outer_closure_ann','test(N) = (s ! S(N)) { ! D = D run(() -> Nil ! N | S(N) { use() }) }'),
 ('outer_body_ann','test(N) = (s ! S(N)) { ! D = D run(() { use() }) }'),
 ('outer_inner_local_ann','test(N) = (s ! S(N)) { ! D = D local: () -> Nil ! N | S(N) = () { use() } run(local) }'),
 ('outer_no_helper','test(N) = (s ! S(N)) { ! D = D use() }'),
 ('outer_closure_var','test(N) = (s ! S(N)) { ! D = D local = () { use() } local() }'),
 ('outer_closure_direct','test(N) = (s ! S(N)) { ! D = D (() { use() })() }'),
 ('outer_no_sparam','test = () { ! D = D run(() { use() }) }'),
 ('inner_body_ann','test(N) = (s ! S(N)) { run(() -> Nil ! N | S(N) { ! D = D use() }) }'),
]: check(name,prefix+body)
# no marker; covariant N can infer additional bounds
for marker in ['',', (N) -> Nil']:
 for ann in ['', '-> Nil ! N | S(N)']:
  check('co_'+str(len(marker))+'_'+str(len(ann)),f'''S(N) {{ S(() -> Nil ! N{marker}) }}
use(N) = (s ! S(N)) {ann} {{ S(f{', _' if marker else ''}) = s f() }}
run(N) = (body: () -> Nil ! N) {{ body() }}
f(N) = (s ! S(N)) {{ run(() {{ use() }}) }}''')
