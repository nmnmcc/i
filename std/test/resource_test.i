(
	testlib = (test) = std/test
	i/fail
	i/resource
	i/task
)

test("scoped_gives_body_value_test", () {
	assert(resource.scoped(() { 7 }) == 7)
	()
})

test("scoped_empty_scope_test", () {
	assert(resource.scoped(() { () }) == ())
	()
})

test("scoped_nested_value_test", () {
	v = resource.scoped(() {
		inner = resource.scoped(() {
			resource.acquire(() { 20 }, (_) { () }) + 1
		})
		inner + resource.acquire(() { 100 }, (_) { () })
	})
	assert(v == 121)
	()
})

test("acquire_gives_opened_value_test", () {
	v = resource.scoped(() {
		a = resource.acquire(() { "a" }, (_) { () })
		b = resource.acquire(() { "b" }, (_) { () })
		a <> b
	})
	assert(v == "ab")
	()
})

test("acquire_open_error_runs_no_release_test", () {
	r = fail.catch(() {
		resource.scoped(() {
			resource.acquire(
				() { fail.raise("open") },
				(_) { panic("released") },
			)
		})
	})
	assert(r == Error("open"))
	()
})

test("body_error_goes_through_cleanup_test", () {
	r = fail.catch(() {
		resource.scoped(() {
			x = resource.acquire(() { 3 }, (_) { () })
			fail.raise(x)
		})
	})
	assert(r == Error(3))
	// The program goes on after the cleanup.
	assert(resource.scoped(() { resource.acquire(() { 4 }, (_) { () }) }) == 4)
	()
})

test("cancelled_task_scope_ends_test", () {
	r = task.run(() {
		fail.catch(() {
			task.scope((s) {
				gate : task.Channel(()) = task.channel()
				task.go(s, () {
					resource.scoped(() {
						resource.acquire(() { 1 }, (_) { () })
						task.send(gate, ())
						task.sync(task.never())
					})
				})
				task.receive(gate)
				fail.raise("cancel")
			})
		})
	})
	assert(r == Error("cancel"))
	()
})
