// spec: 9.9
// expect: pass
// runtime.md R2/R4: go does not yield; a failure before the first child turn
// drops that child without entering its body or installing its final.
(
	i/task
	i/resource
	i/fail
)


main = () {
	result = fail.catch(() {
		task.scope((s) {
			task.go(s, () {
				resource.ensure(() { panic("unstarted child ran") }, () {
					panic("unstarted final ran")
				})
			})
			fail.raise("body")
		})
	})
	assert(result == Error("body"))
}
