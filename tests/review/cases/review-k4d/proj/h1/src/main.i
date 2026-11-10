Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(() -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
c1 = () -> Int ! S(Dep) { use() }
c2: () -> Int ! S(Dep) = () { use() }
c3 = () {
  cb: () -> Int ! S(Dep) = () { use() }
  cb
}
c4 = () -> Int ! S(Dep) { get() }
c5 = (s ! S(Dep)) -> Int ! S(Dep) {
  S(f) = s
  f()
}
c6 = (h: () -> Int ! Dep | S(Dep)) -> () -> Int ! S(Dep) { h }
c7 = (h: () -> Int ! Dep | S(Dep), k: (() -> Int ! S(Dep)) -> Int) { k(h) }
