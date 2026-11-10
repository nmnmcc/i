from pathlib import Path
r=Path('/tmp/review-k4/cases')
def put(name,src):
 p=r/name;(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "app"\n');(p/'src/main.i').write_text(src)
base='''S(A, N) { S(() -> A ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(f, _) = s f() }
run = (f) { f() }
read = () { run(() { use() }) <> "!" }
'''
for mode in ['direct','closure']:
 body='! S(Int, Never) = S(() { 1 }, (_: Never) { Nil }) f()'
 if mode=='closure':body='() { '+body+' }'
 src=base+'with(R, N) = (f: () -> R ! N) { '+body+' }\n@main\nmain = () { '+('g = with(read) g()' if mode=='closure' else 'with(read)')+' Nil }\n'
 put('new_unsound_'+mode,src)
# Correctly typed dependency of the inner closure still applied to wrong service.
put('provision_marker', '''S(A) { S(value: A) }
G(N) { G(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! G(N)) { G(f, _) = s f() }
run = (f) { f() }
with(A, R, N) = (x: A, f: () -> R ! N) { ! S(A) = S(x) f() }
read = (s ! S(String)) { s.value <> "!" }
@main
main = () {
 ! G(Never) = G(() { Nil }, (_: Never) { Nil })
 run(() { use() })
 with(1, read)
 Nil
}
''')
