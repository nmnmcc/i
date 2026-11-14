(
	old
	Old(..)
	fresh
)

@deprecated("use fresh")
old = 1

// A type and its same-named constructor share deprecation warnings.
@deprecated("use New")
Old = ()

fresh = old
