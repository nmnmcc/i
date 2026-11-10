S(A,E,N,M) { S((A) -> A ? E ! N, () -> Nil ! M) }
use(A,E,N,M) = (x: A, s ! S(A,E,N,M)) { S(f, g) = s g() f(x) }
run(A,E,N) = (body: () -> A ? E ! N) { body() }
wrap(A,E,N,M) = (x: A) -> A ? E ! N | M | S(A,E,N,M) { use(x) }
