// spec: 9.9
// expect: pass
// runtime.md R5: failure before a successful commit follows ordinary scope
// failure, while a successful commit suppresses later unobserved errors.
(
	i/task
	i/fail
	i/random
)

main = () {
	// R5 checks a particular order. task.test's exhausted script uses FIFO
	// (spec §9.9); a yield alone cannot impose that order across workers.
	task.test(random.from_seed(7), [], () {
		assert(
			fail.catch(() {
				task.race(
					() { fail.raise("first") },
					() {
						task.yield()
						2
					},
				)
			}) ==
				Error("first"),
		)
		assert(
			fail.catch(() {
				task.race(
					() { 1 },
					() {
						task.yield()
						fail.raise("late")
					},
				)
			}) ==
				Ok(1),
		)
	})
	// Without a testing policy either completion may be observed first.
	first = fail.catch(() {
		task.race(
			() { fail.raise("first") },
			() {
				task.yield()
				2
			},
		)
	})
	assert(first == Error("first") || first == Ok(2))
	late = fail.catch(() {
		task.race(
			() { 1 },
			() {
				task.yield()
				fail.raise("late")
			},
		)
	})
	assert(late == Ok(1) || late == Error("late"))
	assert(
		task.both(
			() {
				task.yield()
				1
			},
			() { 2 },
		) ==
			(1, 2),
	)
	assert(
		task.map_concurrently([1, 2, 3], (n) {
			task.yield()
			n + 1
		}) ==
			[2, 3, 4],
	)
}
