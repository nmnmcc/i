// spec: 4.9
// expect: pass
(i/record)
(main)
f(K) = (r: (K: Int), key: K) -> (K: Int) {
  h = () { r }
  h()
}
@main
main = () {
  assert(record.get(f((name: 9), "name"), "name") == 9)
  Nil
}
