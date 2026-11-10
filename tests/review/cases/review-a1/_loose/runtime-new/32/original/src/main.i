Token { Stop }
send = (x) { x }
f = () { send(Stop) }

@main
main = () { assert(f() == Stop) Nil }
