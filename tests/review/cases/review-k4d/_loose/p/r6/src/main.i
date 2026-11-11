(
	(Io) = i/io
)
Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N) }
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

T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
uset(N) = (s ! T(N)) { s.f() }

main = () {
  ! S(Never) = S(() { 1 })
  ! T(S(Never)) = T(() { fw() }, (_: S(Never)) { Nil })
  ! S(Dep) = S(() { get() })
  uset()
  Nil
}
