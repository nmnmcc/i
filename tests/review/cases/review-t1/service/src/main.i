State(A) { State(get: () -> A) }
cur(A) = (seed: A, st ! State(A)) { st.get() }

main = () { ! State(Int) = State(() { 5 }) assert(cur(0)==5) Nil }
