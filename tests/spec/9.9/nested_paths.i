// spec: 9.9
// expect: pass
// runtime.md D10/6.5: nested runs and go share the same counter; nested paths
// retain their absolute prefix and root seed derivation, including task.top.
(
	i/task
	i/random
	i/fail
)

@main
main = () {
	! task.Runner = task.top(random.from_seed(7))
	assert(task.run(() {
		assert(task.path() == [1])
		task.scope((s) {
			a = task.go(s, () { task.path() })
			assert(task.join(a) == [1, 1])
			inner = task.run(() { (task.path(), random.word(task.random())) })
			expected = random.word(fail.from_result(fail.catch(() {
				random.at_path(random.from_seed(7), [1, 2])
			})))
			assert(inner == ([1, 2], expected))
			task.join(task.go(s, () { task.path() }))
		})
	}) == [1, 3])
}
