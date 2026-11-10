(
	old
	Old(..)
	fresh
)

@deprecated("use fresh")
old = 1

@deprecated("use New")
Old {
	OldC
}

fresh = old
