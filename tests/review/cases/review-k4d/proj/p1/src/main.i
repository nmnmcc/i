(
	(Io) = i/io
)
Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) { s.f() }
run(A, N) = (body: () -> A ! N) { body() }
tell = (x: Int) { x |> {
  0 -> "zero"
  1 -> "one"
  2 -> "two"
  3 -> "three"
  4 -> "four"
  _ -> "many"
} }

fw1(N) = (s ! S(N)) -> Int ! N | S(N) { run(() { use() }) }
fw1i = () { run(() { use() }) }
fw2(N) = () -> Int ! N | S(N) { fw1() }
fw2i = () { fw1i() }
outer(N) = () -> Int ! N | S(N) {
  local: () -> Int ! N | S(N) = () { use() }
  run(local)
}
outeri = () {
  local = () { use() }
  run(local)
}
dep3(N) = (s ! S(N), d ! Dep) -> Int ! N | S(N) | Dep { use() + d.v }
dep3i = (d ! Dep) { use() + d.v }
spec = () -> Int ! Dep | S(Dep) { use() }
half(N) = () -> Int ! N | S(Dep) { use() }
wrap(N) = (body: () -> Int ! N | S(N)) -> Int ! N {
  ! S(N) = S(() { 1 })
  body()
}
wrapi = (body) {
  ! S(Never) = S(() { 1 })
  body()
}
wrapj = (body) {
  ! S(_) = S(() { 1 })
  body()
}

main = (io ! Io) {
  a = wrap(() { use() })
  ! Dep = Dep(3)
  b = wrap(() { use() + get() })
  c = wrapi(() { use() + get() })
  ! S(Dep) = S(() { get() - 1 })
  d = fw2() + fw2i() - outer() - outeri() + dep3() - dep3i() + spec() - half()
  io.print(tell(a))
  io.print(tell(b))
  io.print(tell(c))
  io.print(tell(d))
}
