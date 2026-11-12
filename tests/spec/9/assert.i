// spec: 9.6, 6.10, 6.11
// expect: pass
(
	testlib = (test) = std/test
	i/assertion
	i/core
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

test("assert_true", () {
	assert(True)
	assert(1 + 1 == 2)
})

catch_assert = () {
	check(show(assertion.catch(() { 1 })), "Ok(1)")
	check(
		show(assertion.catch(() { assert(False) }))
		"Error(AssertionFailure(\"assert\"))"
	)
	check(show(assertion.catch(() {
		assert(True)
		2
	})), "Ok(2)")
}

test("catch_assert", () { catch_assert() })

first = (r) {
	Ok(v) = r
	v
}

// A constructor parameter takes only its own type's values (syntax three
// §2.12 with D4 only when other constructors' values are known), so the
// refutable pattern is a binding of a parameter of the union type.
param = (r: Result(Int, Int), (a, b)) {
	Ok(v) = r
	v + a + b
}

failing_patterns = () {
	check(show(first(Ok(1))), "1")
	check(
		show(assertion.catch(() { first(Error("no")) }))
		"Error(AssertionFailure(\"pattern Ok(v) did not match Error(\\\"no\\\")\"))"
	)
	check(show(param(Ok(1), (2, 3))), "6")
	check(
		show(assertion.catch(() { param(Error(1), (2, 3)) }))
		"Error(AssertionFailure(\"pattern Ok(v) did not match Error(1)\"))"
	)
	// Literals, lists, and nested patterns.
	lit = (x) {
		1 = x
		x
	}
	check(
		show(assertion.catch(() { lit(2) }))
		"Error(AssertionFailure(\"pattern 1 did not match 2\"))"
	)
	check(show(assertion.catch(() { lit(1) })), "Ok(1)")
	list = (xs) {
		[a, b] = xs
		a + b
	}
	check(show(list([1, 2])), "3")
	check(
		show(assertion.catch(() { list([1, 2, 3]) }))
		"Error(AssertionFailure(\"pattern [a, b] did not match [1, 2, 3]\"))"
	)
	nested = (p) {
		whole = (Ok(x), _) = p
		(x, whole)
	}
	check(show(nested((Ok(1), 2))), "(1, (Ok(1), 2))")
}

test("failing_patterns", () { failing_patterns() })

// A pattern that can not fail for its type needs no `Assert` (§7.3).
Pair {
	Pair(Int, Int)
}

add = (Pair(a, b)) { a + b }

test("can_not_fail", () {
	check(show(add(Pair(1, 2))), "3")
	(a, [..rest]) = (1, [2, 3])
	check(show((a, rest)), "(1, [2, 3])")
})

// Any code may give its own `Assert` (§9.6).

own_assert = () {
	// Nested: the inner `catch` takes the failures of its body.
	nested = assertion.catch(() {
		inner = assertion.catch(() { assert(False) })
		(inner, 2)
	})
	check(show(nested), "Ok((Error(AssertionFailure(\"assert\")), 2))")
	// The `Assert` of an outer `catch`, used inside an inner one, goes to the
	// outer one.
	get = (a ! Assert) { a }
	outer = assertion.catch(() {
		a = get()
		assertion.catch(() {
			! Assert = a
			assert(False)
		})
	})
	check(show(outer), "Error(AssertionFailure(\"assert\"))")
}

test("own_assert", () { own_assert() })
