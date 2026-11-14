S(A) = (value = A)
one = () <- (s : S(Int)) { s.value + 1 }
two = () <- (s : S(String)) { s.value <> "!" }
join = <N, M>(f : () <- N -> Int, g : () <- M -> String) { (f(), g()) }
bad = () { join(one, two) }
