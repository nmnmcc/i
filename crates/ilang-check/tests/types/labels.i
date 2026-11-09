// §4.6 and §6.6: labels put arguments in order; they have no effect on
// types.
User { User(name: String, age: Int) }
make = (name: String, age: Int) { User(name, age) }
by_label = make(age: 3, name: "bob")
mixed = make("bob", age: 3)
ctor = User(age: 3, name: "bob")
annotated: (x: Int) -> Int = (y) { y }
call_annotated = annotated(x: 1)
// Labels have no effect on subtyping.
relabel: (z: Int) -> Int = annotated
local = () {
  f = (a: Int, b: String) { b }
  f(b: "x", a: 1)
}
// A pattern takes labelled fields by label.
age = (u) {
  User(age, ..) = u
  age
}
names = (u) {
  User(n = name, age) = u
  (n, name, age)
}
// Problems.
unknown = make(height: 1, name: "x")
twice = make("a", name: "b")
missing = make(name: "b")
param = (f) { f(x: 1) }
bad_pattern = (u) {
  User(n, a) = u
  n
}
