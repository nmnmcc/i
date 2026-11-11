// spec: 6.7, 11.4, 11.8
// expect: pass
(i/core)
(main)
main = () {
  assert(core.inspect(()) == "()")
  assert(core.inspect((name: "ada", age: 36)) == "(age: 36, name: \"ada\")")
  assert(core.inspect(("Type": 1, "first-name": (x: 2))) == "(\"Type\": 1, \"first-name\": (x: 2))")
  assert((a: 1) != (a: 1, b: 2))
  assert((a: 1) != (b: 1))
  Nil
}
