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
Missing = (value = Int)
needs = () <- (m : Missing) { m.value }

main = () <- (io : Io) {
	task.scope((s) {
		task.go(s, () { needs() })
		Nil
	})
}
