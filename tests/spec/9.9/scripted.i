// spec: 9.9
// expect: pass
// runtime.md 6.5: scripts use absolute semantic paths; exhaustion falls back
// to FIFO, bad paths raise NotRunnable only after structured cleanup.
(
	i/task
	i/random
	i/fail
	i/resource
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

main = () {
	assert(
		task.test(random.from_seed(7), [[1]], () { task.path() }) == [1],
	)
	assert(
		fail.catch(() { task.test(random.from_seed(7), [[999]], () { 1 }) }) ==
			Error(task.NotRunnable(path = [999])),
	)
}
