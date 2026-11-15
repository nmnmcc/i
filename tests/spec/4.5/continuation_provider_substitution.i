// spec: 4.5, 9.4
// expect: compile-error E0515

// EF02: the original direct continuation substitution probe.
(
	i/control
)

Read(A) = (read = () -> A)
read_int = () <- (r : Read(Int)) { r.read() }
use = (k : Continuation(Nil, Int, Read(Int | String))) {
	control.resume(k, Nil) <- (Read(read = () { "oops" }))
}

main = () {
	control.boundary((b) {
		control.suspend(b, (k) { use(k) })
		read_int() + 1
	}) <- (Read(read = () { 1 }))
	Nil
}
