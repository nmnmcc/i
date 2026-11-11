// spec: 3, 6.1, 6.4
// expect: pass
(main)
read = (r: (name: String)) { r.name }
main = () {
  old = { name: "Ada" }
  new = (name: "Ada")
  assert(read(old) == read(new))
  assert((1, 2).0 == 1)
  Nil
}
