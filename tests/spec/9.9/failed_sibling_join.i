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
Out {
	Out(print: (text: String) -> Nil)
}

@main
main = (out ! Out) {
	result = fail.catch(() {
		task.scope((s) {
			task.go(s, () { fail.raise("first") })
			unstarted = task.go(s, () { panic("dropped sibling ran") })
			resource.ensure(() { task.join(unstarted) }, () {
				task.yield()
				out.print("body-final")
			})
		})
	})
	assert(result == Error("first"))
}
