// spec: 9.4, 9.9
// expect: pass
// A run retains the caller's Fail and Sample across three resumptions.
(
	i/task
	i/fail
)
Sample = (value = Int)
read = () <- (sample : Sample) { sample.value }
main = () {
	result = fail.catch(() {
		task.run(() {
			assert(read() == 7)
			task.yield()
			task.yield()
			task.yield()
			assert(read() == 7)
			fail.raise("resumed-run")
		})
	}) <- (Sample(value = 7))
	assert(result == Error("resumed-run"))
}
