from harness import *
sources={
 'multi_run':'''S(A,N) { S((A) -> A ! N) }
use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
run(A,N) = (body: () -> A ! N) { body() }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
@main
main = () { assert(ints() == 2) assert(texts() == "a!") Nil }''',
 'two_run':'''D { D(Int) }
read = (d ! D) { D(n) = d n }
S(A,B) { S(() -> Int ! A, () -> Int ! B) }
use(A,B) = (s ! S(A,B)) { S(f,g) = s f() + g() }
run(N) = (f: () -> Int ! N) { f() }
wrap(A,B) = () { local: () -> Int ! A | B | S(A,B) = () { run(() { use() }) } run(local) }
@main
main = () { ! D = D(7) ! S(D,Never) = S(() { read() }, () { 3 }) assert(wrap() == 10) Nil }''',
 'mutual_run':'''S(N) { S(() -> Int ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f,_) = s f() }
a(N) = (x: Int) -> Int ! N | S(N) { x |> { 0 -> use() _ -> b(x-1) } }
b(N) = (x: Int) -> Int ! N | S(N) { x |> { 0 -> use() _ -> a(x-1) } }
@main
main = () { ! S(Never) = S(() { 42 }, (_: Never) { Nil }) assert(a(4) == 42) assert(b(3) == 42) Nil }''',
 'nested_env_function':'''Dep { Dep(Int) }
get = (d ! Dep) { Dep(x) = d x }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
make(N) = (f: () -> Int ! N) { inner = (g) { local = () { g(f) } local() } inner }
run(N) = (f: () -> Int ! N) { f() }
@main
main = () { ! Dep = Dep(11) ! S(Dep) = S(() { get() }) wrapped = make(() { use() }) assert(wrapped(run) == 11) Nil }'''
}
for name,src in sources.items():
 r=check(name,src);print(name,r,flush=True)
 if r['code']==0:
  for opt in [0,2]:print(opt,run(name,opt),flush=True)
