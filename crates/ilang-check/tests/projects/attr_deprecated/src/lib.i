(
	old
	Old(..)
	fresh
)

@deprecated("use fresh")
old = 1

// A type and its constructor have one name (stage 3, syntax-3.md §2.8).
@deprecated("use New")
Old = ()

fresh = old
