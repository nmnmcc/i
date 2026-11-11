// spec: 4.8, 4.9
// expect: pass
(main)
User { User(name: String) }
Cfg = (cb: (User) -> String, n: Int)
@main
main = () {
  base: Cfg = (cb: (u) { u.name }, n: 0)
  next: Cfg = (..base, cb: (u) { u.name })
  assert(next.cb(User("Ada")) == "Ada")
  Nil
}
