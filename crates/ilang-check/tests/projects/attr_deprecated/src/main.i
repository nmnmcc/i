(
	lib = (old) = app/lib
)

a = old

b = lib.old

c: lib.Old = lib.Old

d = lib.fresh
