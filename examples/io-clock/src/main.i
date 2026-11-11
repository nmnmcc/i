(
	i/clock
	i/stdio
	i/task
)

main = () {
	start = clock.monotonic_ns()
	clock.sleep_ns(10000000)
	end = clock.monotonic_ns()
	assert(end - start >= 10000000)
	task.run(() {
		task.scope((scope) {
			task.go(scope, () { clock.sleep_ns(10000000) })
			task.go(scope, () { clock.sleep_ns(10000000) })
			Nil
		})
	})
	stdio.print("clock ok")
}
