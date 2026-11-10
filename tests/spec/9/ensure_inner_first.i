// spec: 9.5
// expect: stop "panic: inner"
(
	i/fail
	i/resource
)

// Final functions run from the innermost to the outermost.
@main
main = () {
	fail.catch(() {
		resource.ensure(
			() { resource.ensure(() { fail.raise(1) }, () { panic("inner") }) }
			() { panic("outer") }
		)
	})
}
