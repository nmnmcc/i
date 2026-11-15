outer = <N>(xs : List(Int), f : (Int) <- N -> Int) -> List(Int) {
	helper = (rest, acc) {
		rest |>
			([]) { list.reverse(acc) }
			| ([x, ..more]) { helper(more, [f(x), ..acc]) }
	}
	helper(xs, [])
}

curried = (a) { (b) { (c) { a + b + c } } }

long_signature = (
	first_parameter : Int,
	second_parameter : String,
	third : Bool
) -> Result(Int, String) { Ok(first_parameter) }

with_env = (x) <- (log : Logger) -> Int { x }

calls = () {
	each(items, (item) { print(item) })
	fold(items, 0, (acc, item) {
		next = acc + item
		next * 2
	})
}
