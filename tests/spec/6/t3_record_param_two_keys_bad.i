// spec: 4.9
// expect: compile-error E0541
(i/record)
(main)
pick(K) = (a: K, b: K) -> (K: Int) {
  r: (K: Int) = record.put((), a, 1)
  record.put(r, b, 2)
}

main = () { pick("a", "b") }
