S(A) { S(value: A) }
read = (s ! S(String)) { s.value <> "!" }
read_int = (s ! S(Int)) { s.value + 1 }
run = (f) { f() }
with(A, R, N) = (x: A, f: () -> R ! N) { ! S(A) = S(value: x) run(f) }

main = () { with("ok", read) Nil }
