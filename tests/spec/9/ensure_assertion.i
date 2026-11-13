// spec: 9.5, 9.6
// expect: stop "panic: final"
(
	i/assertion
	i/resource
)

// An `Assert` failure taken by `assertion.catch` unwinds, so final
// functions run.
main = () {
	assertion.catch(() {
		resource.ensure(() { assert(False) }, () { panic("final") })
	})
}
