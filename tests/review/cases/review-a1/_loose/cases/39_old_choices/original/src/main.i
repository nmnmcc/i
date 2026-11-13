f = (r: Result(Int, Int)) {
	case
	r
	(Ok(v) | Error(v)) { v }
}
