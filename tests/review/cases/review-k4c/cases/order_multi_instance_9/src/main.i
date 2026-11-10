use(A,N) = (x: A, s ! S(A,N)) { S(f) = s f(x) }
run(A,N) = (body: () -> A ! N) { body() }
texts = () { ! S(String,Never) = S((x: String) { x <> "!" }) wrap("a") }
wrap(A,N) = (x: A) { local = () { use(x) } run(local) }
Dep { Dep }
S(A,N) { S((A) -> A ! N) }
ints = () { ! S(Int,Never) = S((x: Int) { x + 1 }) wrap(1) }
