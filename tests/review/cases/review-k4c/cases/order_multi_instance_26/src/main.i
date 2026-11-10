Dep { Dep }
run(A,N) = (body: () -> A ! N) { body() }
use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
S(A,N) { S((A) -> A ! N) }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
