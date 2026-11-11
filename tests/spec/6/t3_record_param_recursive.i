// spec: 4.9
// expect: compile-error E0543
(main)
f(K) = (r: (K: Int), key: K, n: Int) -> Int {
  n |> { 0 -> 0  _ -> f(r, "other", n - 1) }
}
@main
main = () { Nil }
