// spec: 7.1, 4.8
// expect: compile-error E0501
// Retains the migration probe's nested-pattern and missing-field intent.
P = (x = Int, y = Int)
k = (P(x = (a, b))) { _ = b
	a }
main = () { Nil }
