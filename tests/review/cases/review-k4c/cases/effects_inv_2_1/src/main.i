S(A,E,N) { S((A) -> A ^ E ! N, (N) -> Nil) }
use(A,E,N) = (x: A, s ! S(A,E,N)) { S(f, _) = s f(x) }
run(A,E,N) = (body: () -> A ^ E ! N) { body() }
wrap(A,E,N) = (x: A) -> A ^ E ! N | S(A,E,N) { local = () { use(x) } local() }
