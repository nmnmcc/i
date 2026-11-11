// spec: 9.9
// expect: pass
// runtime.md 6.1: shares are portable even in local scopes and need membership;
// children read the same portable value without embedding it in their handles.
(
	i/task
	i/core
)


main = () {
	task.local_scope((s) {
		shared = task.share(s, [1, 2, 3])
		assert(core.portable(shared))
		t = task.go(s, () { task.get(shared) })
		assert(task.join(t) == [1, 2, 3])
	})
}
