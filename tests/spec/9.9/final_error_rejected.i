// spec: 9.5
// expect: compile-error E0501
// A final function may not give typed errors.
(
	i/resource
	i/fail
)

main = () { resource.ensure(() { Nil }, () { fail.raise("final") }) }
