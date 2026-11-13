// spec: 6.7, 11.4, 11.8
// expect: pass
(i/core)
(main)

main = () {
  assert(core.inspect(()) == "()")
  assert(core.inspect((name: "ada", age: 36)) == "(age = 36, name = \"ada\")")
  assert(core.inspect((first: 1, nested: (x: 2))) == "(first = 1, nested = (x = 2))")
  Nil
}
