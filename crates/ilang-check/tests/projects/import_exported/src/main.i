(
	lib = (public, secret, nothing, Opaque, Open, Shown, Hidden, Private, Missing) = app/lib
)

a = lib.secret

b = lib.Hidden(1)

c = lib.nothing

d : lib.Private = lib.public

e : lib.Missing = public

f = Shown(1)

g : Opaque = Hidden
// Importing Hidden above succeeds as a type (§5.2.1); it does not import its
// private constructor. Both lib.Hidden(1) and the bare Hidden still fail.
// Private retains the negative check for importing an unexported type.
