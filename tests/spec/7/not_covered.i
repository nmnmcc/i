// spec: 7.4
// expect: compile-error E0516

Color = Red | Green

Red {
	Red
}

Green {
	Green
}

name = (c: Color) {
	c |> {
		Red           -> "red"
		Green ? False -> "green"
	}
}
