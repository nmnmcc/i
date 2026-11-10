S(A, N) { S(() -> A ! N, (N) -> Nil) }
use(A, N) = (s ! S(A, N)) { S(f, _) = s f() }
run = (body) { body() }
@main
main = () {

 ! S(Int, Never) = S(() { 7 }, (_: Never) { Nil })
 x = run(() { use() })
 ! S(String, Never) = S(() { "yes" }, (_: Never) { Nil })
 y = run(() { use() })
 assert(x == 7)
 assert(y == "yes")
}
