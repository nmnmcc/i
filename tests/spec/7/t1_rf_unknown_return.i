// spec: 6.12, 7.3
// expect: stop "panic: unknown return caught"
apply = (g) { g() |> ("a" | "c") { 1 } }

main = () {
	apply(() { "b" }) <- (Assert(fail = (_) { panic("unknown return caught") }))
}
