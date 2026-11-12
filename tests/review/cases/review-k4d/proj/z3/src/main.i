(
	(Io) = i/io
)
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
wrap(N) = (k: (() -> Int ! N) -> Int ! N, svc: Svc(String)) -> Int ! N {
  ! Svc(String) = svc
  k(() { ask() + 1 })
}
tell = (x: Int) { x |> {
  2 -> "two"
  _ -> "other"
} }

main = (io ! Io) {
  ! Svc(Int) = Svc(f: () { 1 }, m: (_: Int) { Nil })
  io.print(tell(wrap((b) { b() }, Svc(f: () { "s" }, m: (_: String) { Nil }))))
}
