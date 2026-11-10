from pathlib import Path
root=Path('/tmp/review-std2')
cases={
'minimal': '''S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
example = () { ! S(Never) = S(() { Nil }) use() }
''',
'annotated': '''S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f) = s f() }
''',
'closed': '''S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
closed: () -> Nil ! S(Never) = use
@main
example = () { ! S(Never) = S(() { Nil }) closed() }
''',
'explicit_arg': '''S(N) { S(() -> Nil ! N) }
use(N) = (s: S(N)) { S(f) = s f() }
@main
example = () { use(S(() { Nil })) }
''',
'replacement': '''(i/fail)
S(E) { S(() -> Nil ? E) }
use(E) = (s ! S(E)) { S(f) = s f() }
first: () -> Nil ? String ! S(String) = use
second: () -> Nil ? Int ! S(Int) = use
@main
example = () {
 ! S(String) = S(() { fail.raise("s") })
 Error("s") = fail.catch(first)
 ! S(Int) = S(() { fail.raise(2) })
 Error(2) = fail.catch(second)
 Nil
}
''',
'full': '''(i/fail)
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }
make(E, N) = (send: (Entry) -> Nil ? E ! N) -> Sink(E, N) { Sink(send, (_: N) { Nil }) }
write(E, N) = (entry: Entry, sink ! Sink(E, N)) { Sink(send, _) = sink send(entry) }
example = () {
 ! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
 fail.catch(() { write(Entry("seen")) })
}
''',
'unsafe': '''Consumer(A) { Consumer((A) -> Nil) }
Sink(E,N) { Sink(() -> Nil ? E ! N, (N) -> Nil) }
bad = (s: Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }
'''
}
for name,src in cases.items():
 p=root/name
 (p/'src').mkdir(parents=True,exist_ok=True)
 (p/'i.toml').write_text('name = "probe"\n')
 (p/'src/main.i').write_text(src)
