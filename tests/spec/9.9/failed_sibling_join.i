// spec: 9.9
// expect: pass
// output: body-final
// runtime.md R2/R4a: a failed child cancels the joining body before it observes
// an unstarted sibling's dropped outcome; only the original error returns.
(
	i/task
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

main = () <- (out: Out) {
	result = fail.catch(() {
		task.scope((s) {
			task.go(s, () { fail.raise("first") })
			// §9.9: the sibling may start before the failure; then it waits
			// until the cancellation reaches it, else it is dropped unstarted.
			unstarted = task.go(s, () { task.sync(task.never()) })
			resource.ensure(
				() { task.join(unstarted) },
				() {
					task.yield()
					out.print("body-final")
				},
			)
		})
	})
	assert(result == Error("first"))
}
