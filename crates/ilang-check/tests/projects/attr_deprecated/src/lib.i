(
	old
	Old(..)
	fresh
)

@deprecated("use fresh")
old = 1

OldC = ()
@deprecated("use New")
Old = OldC

fresh = old
