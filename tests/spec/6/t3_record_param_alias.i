// spec: 4.9
// expect: pass
(i/record)
(main)
f(K) = (r: (K: Int), other = key: K) -> Int {
  record.get(r, key) + record.get(r, other)
}

main = () { assert(f((a: 3), "a") == 6)  Nil }
