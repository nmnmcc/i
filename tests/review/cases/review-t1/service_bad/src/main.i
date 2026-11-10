State(A) { State(get: () -> A) }
f = (s ! State(1)) { s.get() }
g = () { ! State(Int) = State(() { 2 }) f() }
