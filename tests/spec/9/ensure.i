// spec: 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/core
	i/fail
	i/resource
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

nothing = () { Nil }

test("values_and_errors_go_through", () {
	check(show(resource.ensure(() { 1 }, nothing)), "1")
	r = fail.catch(() { resource.ensure(() { fail.raise("e") }, nothing) })
	check(show(r), "Error(\"e\")")
	nested = fail.catch(() {
		resource.ensure(
			() { resource.ensure(() { fail.raise(1) }, nothing) + 1 }
			nothing
		)
	})
	check(show(nested), "Error(1)")
	// After an error has gone through `ensure`, the program goes on.
	check(show(resource.ensure(() { 2 }, nothing)), "2")
})

acquire = () {
	v = resource.acquire(() { 10 }, (r) { Nil }, (r) { r + 1 })
	check(show(v), "11")
	e = fail.catch(() {
		resource.acquire(() { 10 }, (r) { Nil }, (r) { fail.raise(r) })
	})
	check(show(e), "Error(10)")
	// An error of `open` runs no `release`.
	o = fail.catch(() {
		resource.acquire(
			() { fail.raise("open") }
			(r) { panic("released") }
			(r) { r }
		)
	})
	check(show(o), "Error(\"open\")")
}

test("acquire", () { acquire() })
