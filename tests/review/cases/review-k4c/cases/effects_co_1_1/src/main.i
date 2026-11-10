S(A,E,N) { S((A) -> A ? E ! N) }
use(A,E,N) = (x: A, s ! S(A,E,N)) { S(f) = s f(x) }
run(A,E,N) = (body: () -> A ? E ! N) { body() }
wrap(A,E,N) = (x: A) -> A ? E ! N | S(A,E,N) { run(() { use(x) }) }
