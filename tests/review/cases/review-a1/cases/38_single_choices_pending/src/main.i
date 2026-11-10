f = (r: Result(Int, Int)) {
	r |> {
		Ok(v) | Error(v) -> v
	}
}
