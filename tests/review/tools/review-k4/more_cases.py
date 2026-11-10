from pathlib import Path
r=Path('/tmp/review-k4/cases')
def put(name,src):
 p=r/name;(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "app"\n');(p/'src/main.i').write_text(src)
s=(r/'closure/src/main.i').read_text()
put('closure_inner_annotation',s.replace(' { () { ! S(N)', ' { () -> Nil ! N { ! S(N)'))
put('closure_noreturn',s.replace('make(N) = (f: () -> Nil ! N) { () { ! S(N) = S(f, (_: N) { Nil }) use() } }','make(N) = (f: () -> Nil ! N) { ! S(N) = S(f, (_: N) { Nil }) use() }').replace('f = make(() { Nil }) f()', 'make(() { Nil })'))
put('closure_unused',s[:s.index('@main')])
put('closure_covariant',s.replace(', (N) -> Nil','').replace('S(f, _)','S(f)').replace('S(f, (_: N) { Nil })','S(f)'))
put('closure_explicit_use',s.replace('use() } }', 'useN: () -> Nil ! N | S(N) = use useN() } }'))
put('closure_captured', '''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
make(N) = (s: S(N)) { () { ! S(N) = s use() } }
@main
main = () { f = make(S(() { Nil }, (_: Never) { Nil })) f() }
''')
for marker in ['', ', (N) -> Nil']:
 for helper in ['run(N) = (body: () -> Nil ! N) { body() }','run = (body) { body() }']:
  for anno in ['', '-> Nil ! N | S(N)']:
   ident=f'matrix_{bool(marker)}_{"(N)" in helper}_{bool(anno)}'
   src=f'''S(N) {{ S(() -> Nil ! N{marker}) }}
use(N) = (s ! S(N)) {anno} {{ S(f{', _' if marker else ''}) = s f() }}
{helper}
f(N) = (s ! S(N)) {anno} {{ run(() {{ use() }}) }}
@main
main = () {{ ! S(Never) = S(() {{ Nil }}{', (_: Never) { Nil }' if marker else ''}) f() }}
'''
   put(ident,src)
   lines=src.splitlines();put(ident+'_reverse','\n'.join([lines[0],lines[3],lines[2],lines[1],*lines[4:]])+'\n')
# recursive lower bound in ordinary types, with tautological union
for name,src in {
 'recursive_value':'Box(A) { Box(A) }\nbad = () { Box(bad()) }',
 'recursive_mutual':'Box(A) { Box(A) }\nf = () { Box(g()) }\ng = () { f() }',
 'recursive_inv':'Cell(A) { Cell(A, (A) -> Nil) }\nbad = (x) { y: Cell(_) = x Cell(y, (_) { Nil }) |> bad }',
 'recursive_env_cov':'S(N) { S(() -> Nil ! N) }\nbad(N) = (s ! S(N)) -> Nil ! N { S(f) = s f() }',
 'recursive_env_mutual':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nf(N) = (s ! S(N)) -> Nil ! N { g() }\ng = () { f() }',
 'two_rows':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nT(N) { T(() -> Nil ! N, (N) -> Nil) }\nuse(N, M) = (s ! S(N), t ! T(M)) { S(f, _) = s T(g, _) = t f() g() }',
 'same_row':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nT(N) { T(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N), t ! T(N)) { S(f, _) = s T(g, _) = t f() g() }',
 'invariant_args':'S(A, N) { S(A, (A) -> Nil, () -> Nil ! N, (N) -> Nil) }\nuse(A, N) = (s ! S(A, N)) { S(x, f, g, _) = s f(x) g() x }\nrun = (f) { f() }\nf(A, N) = (s ! S(A, N)) { run(() { use() }) }',
}.items():put(name,src+'\n')
