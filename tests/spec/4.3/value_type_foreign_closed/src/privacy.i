(
	Opaque
	Hidden
	Alias(..)
	make
)

Hidden = (Int)

Opaque = Hidden
value : Opaque = Hidden(1)

Alias = value

make = () { value }
