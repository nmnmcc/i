(
	start
	Color
	Red(..)
	Green(..)
	Blue(..)
	describe
)

Color = Red | Green | Blue

Red {
	Red
}

Green {
	Green
}

Blue {
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
