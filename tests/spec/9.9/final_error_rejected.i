// spec: 9.5
// expect: pass
// Owner decision 9 permits typed final errors; preserve the same final error.
(
	i/resource
	i/fail
)

main = () {
	assert(
		fail.catch(() {
			resource.ensure(() { Nil }, () { fail.raise("final") })
		}) ==
			Error("final"),
	)
}
