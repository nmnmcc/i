(
	(Io) = i/io
)
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
tell = (x: Int) { x |> {
  2 -> "two"
  _ -> "other"
} }

main = (io ! Io) {
  ! Svc(Int) = Svc(f: () { 1 }, m: (_: Int) { Nil })
  body = () { ask() + 1 }
  inner(N) = (b: () -> Int ! N) -> Int ! N {
    ! Svc(String) = Svc(f: () { "s" }, m: (_: String) { Nil })
    b()
  }
  io.print(tell(inner(body)))
}
