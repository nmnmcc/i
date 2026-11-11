// spec: 4.9
// expect: compile-error E0543
(main)
f(K) = (r: (K: Int), key: K) {
  x: key = key
  x
}
@main
main = () { Nil }
