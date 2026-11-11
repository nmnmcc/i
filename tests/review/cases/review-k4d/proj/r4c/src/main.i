(
	(Io) = i/io
)
Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) { s.f() }
run(A, N) = (body: () -> A ! N) { body() }
fw = () { use() }
fr = () { run(() { use() }) }
tell = (x: Int) { x |> {
  0 -> "zero"
  1 -> "one"
  2 -> "two"
  3 -> "three"
  4 -> "four"
  _ -> "many"
} }


main = (io ! Io) {
  ! S(Dep) = S(() { get() }, (_: Dep) { Nil })
  ! Dep = Dep(2)
  h = () { run(fr) }
  k = () { run(h) + fw() }
  io.print(tell(k()))
  ! S(Never) = S(() { 1 }, (_: Never) { Nil })
  io.print(tell(k() + fr()))
}
