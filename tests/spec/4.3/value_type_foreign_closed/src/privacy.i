(
	Opaque
	Alias(..)
	make
)

Opaque {
	Hidden(Int)
}

value: Opaque = Hidden(1)

Alias = value

make = () { value }
