use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
Dep { Dep }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
S(A,N) { S((A) -> A ! N) }
run(A,N) = (body: () -> A ! N) { body() }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
