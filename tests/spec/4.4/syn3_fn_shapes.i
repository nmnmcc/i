// spec: 4.4, 4.6
// expect: pass
// Syntax three stage 7: a function type has one data parameter. Functions
// of different shapes put together in one union have the parameter `Never`
// (shown `{ Never } -> A`); this is not a problem until such a function is
// called. The shown type of `xs` is `List({ Never } -> Int)`
// (crates/ilang-check/tests/types/syn3_fn_shapes.out).
(main)

xs = [(a) { a }, (a, b) { a }]

main = () {
  assert(list_length(xs) == 2)
  Nil
}

list_length(A) = (l: List(A)) -> Int { l |> { [] -> 0  [_, ..rest] -> 1 + list_length(rest) } }
