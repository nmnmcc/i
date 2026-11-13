// spec: 9.5, 11.7
// expect: stop "panic: body"
(
	i/resource
)

// When the program stops, final functions are not run.
main = () { resource.ensure(() { panic("body") }, () { panic("final") }) }
