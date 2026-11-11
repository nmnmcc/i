// spec: 4.9
// expect: compile-error E0544
(main)
f(K) = (r: (K: Int), key: K) -> Int { r.name }

main = () { Nil }
