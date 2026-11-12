(
	(Io) = i/io
)
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
wrap = (body, svc: Svc(String)) {
  ! Svc(String) = svc
  body()
}
tell = (x: Int) { x |> {
  2 -> "two"
  _ -> "other"
} }

main = (io ! Io) {
  ! Svc(Int) = Svc(f: () { 1 }, m: (_: Int) { Nil })
  io.print(tell(wrap(() { ask() + 1 }, Svc(f: () { "s" }, m: (_: String) { Nil }))))
}
