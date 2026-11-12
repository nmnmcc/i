// spec: 9.9, 9.10
// expect: pass
// runtime.md 6.1/6.5: top preserves the full drawn/split generator state;
// task.random is a data read, and nested run/go paths use the combined counter.
(
	i/task
	i/random
	i/fail
)

at = (seed: random.Generator, path: List(Int)) {
	fail.catch(() { random.at_path(seed, path) }) |>
		(Ok(generator)) { generator }
		| (Error(_)) { panic("invalid expected path") }
}

main = () {
	(_, advanced) = random.word(random.from_seed(-7))
	(left, _) = random.split(advanced)
	task.run(() {
		assert(task.path() == [1])
		assert(task.random() == at(left, [1]))
		assert(
			random.word(task.random()) == random.word(task.random()),
		)
		task.scope((s) {
			child = task.go(s, () { (task.path(), task.random()) })
			assert(task.join(child) == ([1, 1], at(left, [1, 1])))
		})
	}) <- (task.top(left))
}
