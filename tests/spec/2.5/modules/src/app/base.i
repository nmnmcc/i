(
	start
	Color(..)
	describe
)

Color {
	Red
	Green
	Blue(level: Int)
}

start = 40

describe = (c: Color) {
	c |> {
		Red         -> "red"
		Green       -> "green"
		Blue(level = level) -> "blue " <> core_inspect(level)
	}
}

core_inspect = (n: Int) {
	n |> {
		0 -> "0"
		_ -> "many"
	}
}
