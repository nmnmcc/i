(i/fail)
S(E) { S(() -> Nil ? E) }
use(E) = (s ! S(E)) { S(f) = s f() }
first: () -> Nil ? String ! S(String) = use
second: () -> Nil ? Int ! S(Int) = use
@main
example = () {
 { ! S(String) = S(() { fail.raise("s") }) Error("s") = fail.catch(first) Nil }
 { ! S(Int) = S(() { fail.raise(2) }) Error(2) = fail.catch(second) Nil }
}
