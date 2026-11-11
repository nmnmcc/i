// spec: 9.9
// expect: pass
// runtime.md R1/R2/6.3: a retained outer callback uses its outer scope chain.
// Its own scope failure returns its error; it does not cancel the caller's run.
(
	i/task
	i/control
	i/fail
)

@main
main = () {
	callback = control.keep_env((_: Nil) {
		task.scope((s) {
			task.go(s, () { fail.raise("callback") })
			task.sync(task.never())
		})
	})
	assert(task.run(() {
		assert(fail.catch(() { callback(Nil) }) == Error("callback"))
		task.yield()
		42
	}) == 42)
}
