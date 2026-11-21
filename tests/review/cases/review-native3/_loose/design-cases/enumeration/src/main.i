// Design counterexample; not runnable until R2 exists.
// Host gate remains outstanding until the review fixture releases it.
(
	i/task
	i/request
	i/hostcall
)
@host("review:gate/v1", "request")
Gate = (wait = () -> Request(()))

main = () <- (gate : Gate) {
	root = hostcall.root()
	pending = gate.wait()
	task.scope((s) {
		child = task.go(s, () {
			request.outstanding_root(root) |> ([]) { 0 } & ([_, ..]) { 1 }
		})
		count = task.join(child)
		request.cancel(request.ticket(pending))
		assert(count == 1)
	})
}
