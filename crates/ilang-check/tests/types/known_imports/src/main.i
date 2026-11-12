// §4.8: an imported name, and a member `m.x` of an imported module, have
// their type scheme in its simpler form (§4.7), also when they are not
// generalized.
(
	lib = (x) = app/lib
)

member = lib.x.name

imported = x.age

generalized = lib.y.name

call = lib.mk(lib.y).name

list = lib.xs |> ([u, ..]) { u.name } | ([]) { "" }
