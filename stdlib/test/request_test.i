(
	testlib = (test) = i/test
	i/request
	i/task
	i/core
)

// A finished fake request is answered at once, also inside a child task: the
// task-side Await is a scheduler command, and a settled ticket must be
// answered by a command reply, not just by queueing the task.
test("await_ready_request_in_a_child_task_test", () {
	task.run(() {
		got = task.ref(0)
		task.scope((g) {
			t = task.go(g, () {
				task.set(
					got,
					request.await(request.ready(42)),
				)
			})
			task.join(t)
			()
		})
		assert(task.get(got) == 42)
		()
	})
	()
})

test("await_failed_request_in_a_child_task_test", () {
	task.run(() {
		got = task.ref(Ok(0))
		task.scope((g) {
			t = task.go(g, () {
				task.set(
					got,
					request.await(request.ready(Error("boom"))),
				)
			})
			task.join(t)
			()
		})
		assert(task.get(got) == Error("boom"))
		()
	})
	()
})
