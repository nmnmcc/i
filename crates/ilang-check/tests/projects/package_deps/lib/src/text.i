(
	textlib/inner
)

(
	shout
)

shout = (s: String) { inner.bang(s) }

// `@main` in other packages is not used (§10.2).

main = () { Nil }
