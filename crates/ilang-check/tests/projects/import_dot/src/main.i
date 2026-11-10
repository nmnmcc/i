(
	app/lib
)

// `lib` is a value here, so `.x` is a field (§2.3).
f = (lib) { lib.x }

g = lib.x

h = lib

k = (v) { v.C }

m = x.C

n: g.T = 1
