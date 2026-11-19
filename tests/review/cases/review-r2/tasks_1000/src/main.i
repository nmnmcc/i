(
	i/task
	i/resource
	i/fail
	i/request
	i/control
	i/core
	rng = i/random
	(Io) = i/io
)
many = (s, n : Int) {
	{ n == 0 } |>
		(True) { () }
		& (False) {
			task.go(s, () { () })
			many(s, n - 1)
		}
}

main = () <- (io : Io) {
	task.scope((s) { many(s, 1000) })
	io.print("tasks-ok")
}
