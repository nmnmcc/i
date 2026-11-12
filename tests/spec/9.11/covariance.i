// spec: 4.5, 9.11
// expect: pass
// runtime.md D2: Request(A) is covariant; reading a widened handle keeps the union.
(
	i/request
)
widen = (r: Request(Int | String)) { request.take(r) }

main = () {
	assert(
		widen(request.ready(37)) == 37,
	)
}
