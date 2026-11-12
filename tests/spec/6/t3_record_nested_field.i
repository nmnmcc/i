// spec: 4.8, 4.9
// expect: pass
(main)
User { User(name: String) }

main = () {
  a = (user: User(name: "Ada"))
  b = (..a, age: 36)
  assert(b.user.name == "Ada")
  Nil
}
