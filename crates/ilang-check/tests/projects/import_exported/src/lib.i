(
	public
	Opaque
	Hidden
	Open(..)
)

public = 1

secret = 2

Hidden = (Int)
Opaque = Hidden

Shown = (Int)
Open = Shown

Private = ()

// Syntax three §2.8: Hidden is a labelled type; Opaque is its transparent alias.
// §5.2.1 requires the type used by an exported alias to be exported too, so
// Hidden is exported above without (..). Its constructor remains private.
