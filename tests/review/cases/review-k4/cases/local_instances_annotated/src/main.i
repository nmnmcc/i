S(A, N) { S(() -> A ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(f, _) = s f() }

main = () {
 run = (body) { body() }
 ! S(Int, Never) = S(() { 7 }, (_: Never) { Nil })
 x: Int = run(() { use() })
 ! S(String, Never) = S(() { "yes" }, (_: Never) { Nil })
 y: String = run(() { use() })
 assert(x == 7)
 assert(y == "yes")
}
