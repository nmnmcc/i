// The examples of the spec, with the types it gives.
(
	i/assertion
	i/control
	i/fail
)

// §9.1
retry(A, E, N) = (times: Int, body: () -> A ? E ! N) -> A ? E ! N {
	fail.catch(body) |> {
		Ok(value) -> value
		Error(error) -> { times > 1 } |> {
			True  -> retry(times - 1, body)
			False -> fail.raise(error)
		}
	}
}

load_user = (id: String) -> String ? Int { fail.raise(404) }

user = () { retry(3, () { load_user("1") }) }

// §9.2
Clock {
	Clock(now: () -> Int)
}

elapsed = (start: Int, clock ! Clock) { clock.now() - start }

// type: (start: Int) -> Int ! Clock

fixed = () {
	! Clock = Clock(now: () { 42 })
	elapsed(40)
}

// type: () -> Int

// §9.4
Yield(A) {
	Yield(emit: (value: A) -> Nil)
}

collect(A, N) = (body: () -> Nil ! Yield(A) | N) -> List(A) ! N {
	control.boundary((b) {
		! Yield(A) = Yield((x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		body()
		[]
	})
}

numbers = () {
	collect((y ! Yield(Int)) {
		y.emit(1)
		y.emit(2)
	})
}

// §9.6
my_assert = (c: Bool, a ! Assert) {
	c |> {
		True  -> Nil
		False -> a.fail(AssertionFailure("assert"))
	}
}

// type: (c: Bool) -> Nil ! Assert

catch = (body) {
	control.boundary((b) {
		! Assert = Assert((failure) {
			control.suspend(b, (k) {
				control.cancel(k)
				Error(failure)
			})
		})
		Ok(body())
	})
}

// type: (body: () -> A ? E ! N) -> Result(A, AssertionFailure) ? E ! N - Assert

checked = () {
	catch(() {
		assert(False)
		1
	})
}

checked_library = () {
	assertion.catch(() {
		assert(1 == 2)
		"x"
	})
}

// §4.4
inc = (x) {
	x + 1
	x
}

// type: (x: A & Int) -> A

// §6.5
mixed = [1, "a"]
