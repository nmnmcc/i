// spec: 4.8, 4.9
// expect: pass
(main)
map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
  xs |> { [] -> []  [x, ..rest] -> [f(x), ..map(rest, f)] }
}
@main
main = () {
  assert(map([], (x) { x.name }) == [])
  Nil
}
