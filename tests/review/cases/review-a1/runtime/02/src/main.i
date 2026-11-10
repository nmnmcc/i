(
	i/core
)

(
	f
)

f = () { core.inspect(42) }

@main
main = () {
	assert(f() == "42")
	Nil
}
