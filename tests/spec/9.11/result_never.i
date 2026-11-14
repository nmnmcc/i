// spec: 9.7, 9.11
// expect: pass
// runtime.md 5.3: Request(Result(A,Never)) still decodes Ok(A).
(
	i/request
)
@host("test:async/v1", "request")
Later = (double = (Int) -> Request(Result(Int, Never)))

main = () <- (later : Later) {
	assert(
		request.await_owned(later.double(21)) == Ok(42),
	)
}
