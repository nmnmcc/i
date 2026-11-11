// spec: 4.9
// expect: compile-error E0541
(main)
f(K) = (r: (K: Int), key: K) -> Int { 1 }
g = (key: String) { f((name: 9), key) }

main = () { Nil }
