// spec: 9.9
// expect: pass
// runtime.md R5: failure before a successful commit follows ordinary scope
// failure, while a successful commit suppresses later unobserved errors.
// The scripted run fixes which branch gets the first turn in each race.
(
	i/task
	i/fail
	i/random
)


main = () { task.test(random.from_seed(1), [], () {
	assert(fail.catch(() {
		task.race(() { fail.raise("first") }, () {
			task.yield()
			2
		})
	}) == Error("first"))
	assert(fail.catch(() {
		task.race(() { 1 }, () {
			task.yield()
			fail.raise("late")
		})
	}) == Ok(1))
	assert(task.both(() {
		task.yield()
		1
	}, () { 2 }) == (1, 2))
	assert(task.map_concurrently([1, 2, 3], (n) {
		task.yield()
		n + 1
	}) == [2, 3, 4])
	})
}
