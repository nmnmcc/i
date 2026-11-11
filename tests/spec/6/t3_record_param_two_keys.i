// spec: 4.9
// expect: pass
(i/record)
(main)
pick(K) = (a: K, b: K) -> (K: Int) {
  r: (K: Int) = record.put((), a, 1)
  record.put(r, b, 2)
}
@main
main = () {
  assert(record.get(pick("a", "a"), "a") == 2)
  Nil
}
