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
			ready: task.Channel(Nil) = task.channel()
			resource.ensure(
				() {
					task.go(s, () {
						task.receive(ready)
						fail.raise("first")
					})
					// §9.9 permits the sibling to start or be dropped unstarted.
					unstarted = task.go(s, () { task.sync(task.never()) })
					// Install the body final and create its join target before failure.
					task.send(ready, Nil)
					task.join(unstarted)
				},
				() {
					task.yield()
					out.print("body-final")
				},
			)
		})
	})
	assert(result == Error("first"))
}
