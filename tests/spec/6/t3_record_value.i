// spec: 4.8, 6.7
// expect: pass
(main)

name_of = (r: (age: Int, name: String)) { r.name }


main = () {
  ada = (age: 36, name: "Ada")
  updated = (..ada, age: 37)
  assert(name_of(ada) == "Ada")
  assert(updated.name == "Ada")
  assert(updated.age == 37)
  assert(updated == (name: "Ada", age: 37))
  assert((name: "Ada", age: 37) == updated)
  Nil
}
