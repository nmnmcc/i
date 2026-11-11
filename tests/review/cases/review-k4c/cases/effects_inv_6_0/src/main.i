S(A,E,N) { S((A) -> A ^ E ! N, (N) -> Nil) }
use(A,E,N) = (x: A, s ! S(A,E,N)) { S(f, _) = s f(x) }
run(A,E,N) = (body: () -> A ^ E ! N) { body() }
wrap(A,E,N) = (x: A)  { local = () { inner = () { use(x) } inner() } run(local) }
