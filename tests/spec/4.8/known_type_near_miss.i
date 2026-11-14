// spec: 4.8
// expect: pass
(main)
User = (name = String)
apply = (f: (User) -> String) { f(User(name = "Ada")) }
main = () {
  written = (u: User) { u.name }
  expected: (User) -> String = (u) { u.name }
  assert(written(User(name = "Ada")) == "Ada")
  assert(expected(User(name = "Ada")) == "Ada")
  assert(apply((u) { u.name }) == "Ada")
}
