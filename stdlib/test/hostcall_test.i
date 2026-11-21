(
	testlib = (test, fails) = i/test
	i/core
	i/hostcall
	i/request
)

// A `Call` is the request cell, so a ready fake request stands in for an
// answered start in the tests that need no host.

test("with_wait_replaces_the_wait_test", () {
	// A one-way cell: the fake wait takes it, so `Taken` proves it ran.
	marker = request.ready(())
	w = hostcall.Wait(wait = (_) { request.take(marker) })
	got = hostcall.with_wait(w, () { hostcall.await(request.ready(3)) })
	assert(got == 3)
	assert(request.state(marker) == request.Taken())
	()
})

test("with_wait_restores_after_the_body_test", () {
	marker = request.ready(())
	w = hostcall.Wait(wait = (_) { request.take(marker) })
	hostcall.with_wait(w, () { () })
	assert(hostcall.await(request.ready(1)) == 1)
	assert(request.state(marker) == request.Ready(()))
	()
})

test("await_takes_a_ready_call_test", () {
	c = request.ready(42)
	assert(hostcall.settled(c))
	assert(hostcall.await(c) == 42)
	assert(hostcall.state(c) == request.Taken())
	()
})

test("take_takes_a_ready_call_test", () {
	c = request.ready(1)
	assert(hostcall.take(c) == 1)
	assert(hostcall.state(c) == request.Taken())
	()
})

test("cancel_is_idempotent_test", () {
	c = request.ready(1)
	hostcall.cancel(c)
	hostcall.cancel(c)
	assert(hostcall.settled(c))
	()
})

fails("take_twice_panics_test", () {
	c = request.ready(1)
	hostcall.take(c)
	hostcall.take(c)
	()
})

fails("await_a_taken_call_panics_test", () {
	c = request.ready(1)
	hostcall.take(c)
	hostcall.await(c)
	()
})

// `in_group` provides the group as the Owner of every start in the body:
// `request.within` opens its nested owner at the provided group's Root.
test("in_group_routes_a_start_test", () {
	r = hostcall.root()
	g = request.open_owner() <- (r)
	seen = hostcall.in_group(g, () { request.within(() { 5 }) })
	assert(seen == 5)
	hostcall.close(g)
	()
})

test("group_opens_and_closes_test", () {
	g = hostcall.group()
	assert(core.not(hostcall.closed(g)))
	hostcall.close(g)
	assert(hostcall.closed(g))
	hostcall.close(g)
	assert(hostcall.closed(g))
	()
})

test("closing_the_root_closes_its_groups_test", () {
	r = hostcall.open_root()
	g = hostcall.group_at(r)
	hostcall.close_root(r)
	assert(hostcall.closed(g))
	()
})

test("in_group_restores_after_the_body_test", () {
	outer = hostcall.group()
	inner = hostcall.group()
	hostcall.in_group(outer, () {
		hostcall.in_group(inner, () { () })
		hostcall.close(inner)
		// The outer group is the ambient one again: adopting into it works.
		assert(hostcall.await(hostcall.adopt(request.ready(1))) == 1)
		()
	})
	hostcall.close(outer)
	()
})

test("call_returns_a_call_and_await_takes_it_test", () {
	c = hostcall.call(() { 41 + 1 })
	assert(hostcall.settled(c))
	assert(hostcall.await(c) == 42)
	assert(hostcall.state(c) == request.Taken())
	()
})

fails("in_group_of_a_closed_group_panics_test", () {
	g = hostcall.group()
	hostcall.close(g)
	hostcall.in_group(g, () { () })
})

fails("call_in_a_closed_group_panics_test", () {
	g = hostcall.group()
	hostcall.close(g)
	hostcall.call_in(g, () { 1 })
	()
})
