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


main = () {
	assert(f() == "2")
	Nil
}
