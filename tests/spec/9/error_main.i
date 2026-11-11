// spec: 9.3, 10.2, 11.7
// expect: stop "error: HttpError(500, 2)"
// at: spec/main:20:1
(
	i/fail
)

HttpError {
	HttpError(Int, Int)
}

get = (n) {
	n |> {
		0 -> fail.raise(HttpError(500, 2))
		_ -> get(n - 1) + 1
	}
}


main = () { get(10) }
