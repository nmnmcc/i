use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
S(A,N) { S((A) -> A ! N) }
Dep { Dep }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
run(A,N) = (body: () -> A ! N) { body() }
