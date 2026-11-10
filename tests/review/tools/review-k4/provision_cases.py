from pathlib import Path
r=Path('/tmp/review-k4/cases')
def put(name,src):
 p=r/name;(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "app"\n');(p/'src/main.i').write_text(src)
base='''S(A) { S(value: A) }
read = (s ! S(String)) { s.value <> "!" }
read_int = (s ! S(Int)) { s.value + 1 }
run = (f) { f() }
'''
for tag,body in {
 'direct':'with(A, R, N) = (x: A, f: () -> R ! N) { ! S(A) = S(x) f() }',
 'helper':'with(A, R, N) = (x: A, f: () -> R ! N) { ! S(A) = S(x) run(f) }',
 'closure':'with(A, R, N) = (x: A, f: () -> R ! N) { () { ! S(A) = S(x) f() } }',
 'inferred':'with(A) = (x: A, f) { ! S(A) = S(x) f() }',
}.items():
 for valid in [False,True]:
  call='with("ok", read)' if valid else 'with(1, read)'
  if tag=='closure':call='f = '+call+' f()'
  put(f'provision_{tag}_{valid}',base+body+'\n@main\nmain = () { '+call+' Nil }\n')
# Selective unions and helper generalization hidden conflicts
for inner in ['f() g()', 'run(f) run(g)', 'run(() { f() g() })']:
 for body in ['join(read, read_int)', 'h = () { join(read, read_int) } h()']:
  tag=str(abs(hash((inner,body))))
  put('conflict_'+tag,base+'join(N, M) = (f: () -> String ! N, g: () -> Int ! M) { '+inner+' }\nbad = () { '+body+' }\n')
# One generalized local is independently instantiated twice.
put('local_instances', '''S(A, N) { S(() -> A ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(f, _) = s f() }
@main
main = () {
 run = (body) { body() }
 ! S(Int, Never) = S(() { 7 }, (_: Never) { Nil })
 x = run(() { use() })
 ! S(String, Never) = S(() { "yes" }, (_: Never) { Nil })
 y = run(() { use() })
 assert(x == 7)
 assert(y == "yes")
}
''')
