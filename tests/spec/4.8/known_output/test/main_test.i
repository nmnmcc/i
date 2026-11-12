// spec: 4.8
// expect: pass

// literal-types decision 7: hidden inputs make literal facts whole at every depth.
(
	testlib = (test) = std/test
	list = std/list
	option = (Option, Some, None) = std/option
	result = std/result
	control = i/control
)

Box(A) {
	Box(value: A)
}

Sink(A) {
	Sink(put: (A) -> Nil)
}

capture(A) = (x: A, f: ((A) -> Nil) -> Nil) -> A {
	control.boundary((b) {
		f((a) {
			control.suspend(b, (k) {
				control.cancel(k)
				a
			})
		})
		x
	})
}

capture2(A) = (x: A, f: (Sink(A)) -> Nil) -> A {
	capture(x, (k) { f(Sink(put: k)) })
}

make(S) = (init: S) { (f: (S) -> S) { f(init) } }

State(S) {
	State(get: () -> S)
}

cur(S) = (seed: S, st ! State(S)) { st.get() }

test("hidden", () {
	assert(capture(0, (k) { k(5) }) == 5)
	assert(capture((0, 0), (k) { k((5, 5)) }).0 == 5)
	assert(capture2(0, (s) { s.put(5) }) == 5)
	assert(capture2((0, 0), (s) { s.put((5, 5)) }).0 == 5)
	assert(make(0)((n) { n + 1 }) == 1)
	assert(make((0, 0))((p) { (p.0 + 1, p.1) }).0 == 1)
	g = make(0)
	assert(g((n) { n + 1 }) == 1)
	! State(Int) = State(get: () { 5 })
	assert(cur(0) == 5)
})

test("folds", () {
	xs = [3, 4, 5, 6, 7]
	assert(list.fold(xs, 0, (n, _) {
		{ n == 3 } |> {
			True  -> n
			False -> n + 1
		}
	}) == 3)
	assert(list.fold(xs, 0, {
		(n, _) {
			{ n == 3 } |> {
				True  -> n
				False -> n + 1
			}
		}
	}) == 3)
	flag = True
	assert(list.fold(xs, 0, flag |> {
		True -> (n, _) {
			{ n == 3 } |> {
				True  -> n
				False -> n + 1
			}
		}
		False -> (n, _) { n + 1 }
	}) == 3)
	assert(list.fold(xs, (0, 0), (p, x) {
		{ p.0 == 3 } |> {
			True  -> p
			False -> (p.0 + 1, x)
		}
	}).0 == 3)
	assert(list.fold(xs, Box(0), (s, _) { Box(s.value + 1) }).value == 5)
	acc = list.fold([3, 4, 5], [], (acc, x) {
		acc |> {
			[h, ..] -> { h == 3 } |> {
				True  -> acc
				False -> [x, ..acc]
			}
			[] -> [x]
		}
	})
	assert(acc == [3])
	assert(list.fold(xs, None, (acc, x) {
		acc |> {
			Some(p: (Int, Int)) -> Some((p.0 + 1, x))
			None                -> Some((0, x))
		}
	}) == Some((4, 7)))
	assert(result.unwrap(Ok(1), 50) == 1)
})

// Decision 7: a service argument and a parameter of a parameter are hidden.
Feed(A) {
	Feed(put: (A) -> Nil)
}

with_feed(A) = (seed: A, body: () -> Nil ! Feed(A)) -> A {
	capture(seed, (put) {
		! Feed(A) = Feed(put: put)
		body()
	})
}

loop(S) = (init: S, body: (S, (S) -> Nil) -> Nil) -> Nil {
	again = (s: S) { body(s, again) }
	again(init)
}

test("hidden_service_and_setter", () {
	assert(with_feed(0, (feed ! Feed(Int)) { feed.put(5) }) == 5)
	loop(0, (n, again) {
		{ n == 3 } |> {
			True  -> Nil
			False -> again(n + 1)
		}
	})
})
