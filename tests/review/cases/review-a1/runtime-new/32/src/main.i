Stop = ()
Token = Stop

send = (x) { x }

f = () { send(Stop) }

main = () {
	assert(f() == Stop)
	Nil
}
