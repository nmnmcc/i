(api = lib/api)
f = (x: api.Modes) { api.identity(x) }
@main
main = () { assert(f(api.last) == "v19") Nil }
