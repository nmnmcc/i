S(A) { S(A) }
T(N) { T(f: () -> Nil ! N) }
merge(N, M) = (a: T(N), b: T(M)) { T(() { a.f() b.f() }) }
one = (s ! S(Int)) { Nil }
two = (s ! S(String)) { Nil }
@main
main = () { unused = merge(T(one), T(two)) Nil }
