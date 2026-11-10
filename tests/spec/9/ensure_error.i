// spec: 9.3, 9.5
// expect: stop "panic: final"
(
	i/fail
	i/resource
)

// `with` runs when an error goes through, before the error is taken.
@main
main = () {
	r = fail.catch(() {
		resource.ensure(() { fail.raise("e") }, () { panic("final") })
	})
	panic("not reached")
}
