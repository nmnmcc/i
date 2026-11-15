// spec: 4.8, 6.12
// expect: pass
(
	main
)
Ok(A) = (A)
Error(E) = (E)
Result(A, E) = Ok(A) | Error(E)
unwrap = (r : Result(Int, Int)) { r |> (Ok(v) | Error(v)) { v } }

main = () {
	assert(unwrap(Ok(7)) == 7)
	assert(unwrap(Error(9)) == 9)
	Nil
}
