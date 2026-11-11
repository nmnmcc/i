(
	app/facade
)

read = (v: facade.Alias) { v.number }


main = () { assert(read(facade.value) == 42) }
