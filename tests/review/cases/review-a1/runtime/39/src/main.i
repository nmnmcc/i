f = (r: Result(Int, Int)) { r |> (Ok(v)) { v } | (Error(v)) { v } }

main = () {
	assert(f(Ok(42)) == 42 && f(Error(7)) == 7)
	Nil
}
