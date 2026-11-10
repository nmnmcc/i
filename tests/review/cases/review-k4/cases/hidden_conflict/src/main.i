S(A) { S(value: A) }
one = (s ! S(Int)) { s.value + 1 }
two = (s ! S(String)) { s.value <> "!" }
join(N, M) = (f: () -> Int ! N, g: () -> String ! M) { (f(), g()) }
bad = () { join(one, two) }
