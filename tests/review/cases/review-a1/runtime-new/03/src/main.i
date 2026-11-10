(
	i/core
)

(
	f
)

f = () {
	2 |> {
		x -> core.inspect(x)
	}
}

@main
main = () {
	assert(f() == "2")
	Nil
}
