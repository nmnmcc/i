// spec: 9.5
// expect: stop "panic: outer"
(
	i/fail
	i/resource
)

// After the inner final function, the error goes on to the outer one.
main = () {
	fail.catch(() {
		resource.ensure(
			() { resource.ensure(() { fail.raise(1) }, () { Nil }) },
			() { panic("outer") },
		)
	})
}
