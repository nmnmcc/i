texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
S(A,N) { S((A) -> A ! N) }
use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
run(A,N) = (body: () -> A ! N) { body() }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
Dep { Dep }
