// spec: 9.9
// expect: pass
// output: script-final
// runtime.md 6.5: NotRunnable is reported only after structured cleanup.
(
	i/task
	i/random
	i/fail
	i/resource
)

@host("test:io/v1")
Out {
	Out(print: (text: String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }


main = () {
	result = fail.catch(() {
		task.test(random.from_seed(7), [[1], [1], [999]], () {
			resource.ensure(() {
				task.yield()
				task.yield()
				1
			}, () {
				task.yield()
				print("script-final")
			})
		})
	})
	assert(result == Error(task.NotRunnable([999])))
}
