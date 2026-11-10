S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
first(N) = (b: Bool, s ! S(N)) {
 b |> { True -> second(False) False -> use() }
}
second(N) = (b: Bool, s ! S(N)) {
 b |> { True -> first(False) False -> use() }
}
