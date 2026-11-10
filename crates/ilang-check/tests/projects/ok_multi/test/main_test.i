(
	app/main
	i/assertion
)

@test
pair_works = () { assert(main.Pair(1, 2) == main.Pair(1, 2)) }
