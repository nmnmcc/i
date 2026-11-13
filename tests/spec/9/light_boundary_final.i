// spec: 9.3, 9.5
// expect: stop "panic: final"
(
	i/fail
	i/resource
)

// An error that leaves a library `catch` (a light boundary) runs the final
// functions on its way, before the catch takes it.
main = () {
	r = fail.catch(() {
		resource.ensure(() { fail.raise("e") }, () { panic("final") })
	})
	panic("not reached")
}
