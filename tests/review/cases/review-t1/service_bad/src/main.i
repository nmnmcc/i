State(A) = (get = () -> A)
f = () <- (s: State(1)) { s.get() }
g = () { f() <- (State(get = () { 2 })) }
