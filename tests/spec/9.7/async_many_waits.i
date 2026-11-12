// spec: 9.7, 10.2, B.4, B.5
// expect: pass

// Each call waits for the host before the next request starts.
@host("test:async/v1", "async")
Later = (double = (Int) <- (Fail(String)) -> Int)

sum = (n: Int, total: Int) <- (later: Later) {
	n |> (0) { total } | (_) { sum(n - 1, total + later.double(n)) }
}

main = () {
	// The host doubles each of 1 through 1000: 2 * 1000 * 1001 / 2.
	assert(sum(1000, 0) == 1001000)
}
