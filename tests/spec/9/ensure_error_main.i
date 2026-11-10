// spec: 9.3, 9.5, 11.7
// expect: stop "error: 1"
// at: spec/ensure_error_main:12:1
(
	i/fail
	i/resource
)

// The final function runs, then the error comes to the program entry.

@main
main = () { resource.ensure(() { fail.raise(1) }, () { Nil }) }
