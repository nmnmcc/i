(api = lib/api)
f = (x: api.Modes) { api.identity(x) }

main = () { assert(f(api.last) == "v19") Nil }
