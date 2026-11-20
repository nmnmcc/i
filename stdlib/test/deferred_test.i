(
	testlib = (test) = i/test
	i/list
	i/deferred
	i/fail
	i/resource
	i/task
	option = (Some, None) = i/option
)

test("await_before_completion_test", () {
	v = task.run(() {
		task.scope((group) {
			d : deferred.Deferred(Int, String) = resource.scoped(() {
				deferred.new()
			})
			reader = task.go(group, () { deferred.await(d) })
			task.yield()
			assert(deferred.complete(d, Ok(42)))
			task.join(reader)
		})
	})
	assert(v == 42)
	()
})

test("await_after_completion_test", () {
	v = task.run(() {
		d : deferred.Deferred(Int, String) = resource.scoped(() {
			deferred.new()
		})
		deferred.complete(d, Ok(7))
		fail.catch(() { deferred.await(d) })
	})
	assert(v == Ok(7))
	()
})

test("many_readers_one_outcome_test", () {
	v = task.run(() {
		task.scope((group) {
			d : deferred.Deferred(Int, String) = resource.scoped(() {
				deferred.new()
			})
			readers = [
				task.go(group, () { deferred.await(d) }),
				task.go(group, () { deferred.await(d) }),
				task.go(group, () { deferred.await(d) }),
				task.go(group, () { deferred.await(d) }),
				task.go(group, () { deferred.await(d) }),
			]
			task.yield()
			first = deferred.complete(d, Ok(5))
			second = deferred.complete(d, Ok(6))
			values = list.map(readers, (reader) { task.join(reader) })
			(first, second, values, deferred.poll(d))
		})
	})
	assert(v == (True, False, [5, 5, 5, 5, 5], Some(Ok(5))))
	()
})

test("second_complete_keeps_outcome_test", () {
	v = task.run(() {
		d : deferred.Deferred(Int, String) = resource.scoped(() {
			deferred.new()
		})
		first = deferred.complete(d, Error("first"))
		second = deferred.complete(d, Ok(1))
		third = deferred.complete(d, Error("third"))
		(first, second, third, deferred.poll(d))
	})
	assert(v == (True, False, False, Some(Error("first"))))
	()
})

test("error_outcome_raises_test", () {
	v = task.run(() {
		task.scope((group) {
			d : deferred.Deferred(Int, String) = resource.scoped(() {
				deferred.new()
			})
			reader = task.go(group, () { fail.catch(() { deferred.await(d) }) })
			task.yield()
			deferred.complete(d, Error("boom"))
			(task.join(reader), deferred.poll(d))
		})
	})
	assert(v == (Error("boom"), Some(Error("boom"))))
	()
})

test("poll_waiting_is_none_test", () {
	v = task.run(() {
		d : deferred.Deferred(Int, String) = resource.scoped(() {
			deferred.new()
		})
		before = (deferred.poll(d), deferred.is_done(d))
		deferred.complete(d, Ok(3))
		after = (deferred.poll(d), deferred.is_done(d))
		(before, after)
	})
	assert(v == ((None, False), (Some(Ok(3)), True)))
	()
})

test("is_done_flips_at_completion_test", () {
	v = task.run(() {
		task.scope((group) {
			d : deferred.Deferred(Int, String) = resource.scoped(() {
				deferred.new()
			})
			reader = task.go(group, () { deferred.await(d) })
			task.yield()
			task.yield()
			still = deferred.is_done(d)
			deferred.complete(d, Ok(9))
			now = deferred.is_done(d)
			(still, now, task.join(reader))
		})
	})
	assert(v == (False, True, 9))
	()
})

test("cancelled_waiter_leaves_producer_and_readers_test", () {
	v = task.run(() {
		task.scope((outer) {
			d : deferred.Deferred(Int, String) = resource.scoped(() {
				deferred.new()
			})
			survivor = task.go(outer, () { deferred.await(d) })
			// Two waiters in a group that is cancelled while they wait.
			cancelled = fail.catch(() {
				task.scope((doomed) {
					task.go(doomed, () { deferred.await(d) })
					task.go(doomed, () { deferred.await(d) })
					task.yield()
					fail.raise("cancel")
				})
			})
			// The producer runs outside the cancelled group.
			producer = task.go(outer, () {
				task.yield()
				deferred.complete(d, Ok(11))
			})
			(
				cancelled,
				task.join(producer),
				task.join(survivor),
				deferred.poll(d),
			)
		})
	})
	assert(v == (Error("cancel"), True, 11, Some(Ok(11))))
	()
})

test("scope_ends_with_waiter_waiting_test", () {
	v = task.run(() {
		d : deferred.Deferred(Int, String) = resource.scoped(() {
			deferred.new()
		})
		ended = fail.catch(() {
			task.scope((group) {
				task.go(group, () { deferred.await(d) })
				task.yield()
				fail.raise("stop")
			})
		})
		// The waiter is gone; the deferred still works afterwards.
		before = deferred.is_done(d)
		completed = deferred.complete(d, Ok(1))
		(ended, before, completed, deferred.poll(d))
	})
	assert(v == (Error("stop"), False, True, Some(Ok(1))))
	()
})
