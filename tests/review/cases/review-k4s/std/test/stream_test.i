(
	testlib = (test) = std/test
	i/fail
	i/control
	i/resource
	i/assertion
	std/stream
	std/list
	(Some, None) = std/option
)

// §3.4: constructing either factory needs no Bias service.
Bias {
	Bias(value: Int)
}

needs_bias = () -> stream.Stream(Int, Never, Bias) { stream.new() }

positive = () -> stream.Stream(Int, Never, Bias) {
	stream.make((b ! Bias) { stream.Next(b.value, stream.new()) })
}

source = (xs: List(Int)) -> stream.Stream(Int, Never, Never) {
	stream.from_list(xs)
}

broken = () -> stream.Stream(Int, String, Never) {
	stream.make(() { fail.raise("bad") })
}

head(A, E, N) = (s: stream.Stream(A, E, N)) -> option.Option(A) ^ E ! N {
	stream.pull(s) |> {
		stream.Done       -> None
		stream.Next(x, _) -> Some(x)
	}
}

// §3.3's finite-observation laws, with distinct functions for composition.
check_laws = (xs: List(Int), ys: List(Int), zs: List(Int)) -> Nil ! Assert {
	a = source(xs)
	b = source(ys)
	c = source(zs)
	f = (x: Int) -> Int { x + 1 }
	g = (x: Int) -> Int { x * 2 }
	singleton = (x: Int) -> stream.Stream(Int, Never, Never) { stream.wrap(x) }
	expand = (x: Int) -> stream.Stream(Int, Never, Never) { source([x, x + 1]) }
	assert(stream.to_list(a) == xs)
	assert(stream.to_list(stream.zip(a, b)) == list.zip(xs, ys))
	assert(stream.to_list(stream.map(a, (x) { x })) == xs)
	assert(
		stream.to_list(stream.map(stream.map(a, f), g)) ==
			stream.to_list(stream.map(a, (x) { g(f(x)) }))
	)
	assert(stream.to_list(stream.append(a, stream.new())) == xs)
	assert(stream.to_list(stream.append(stream.new(), a)) == xs)
	assert(
		stream.to_list(stream.append(stream.append(a, b), c)) ==
			stream.to_list(stream.append(a, stream.append(b, c)))
	)
	assert(stream.to_list(stream.flat_map(a, singleton)) == xs)
	assert(
		stream.to_list(stream.flat_map(singleton(4), expand)) ==
			stream.to_list(expand(4))
	)
	assert(
		stream.to_list(stream.flat_map(stream.flat_map(a, expand), (x) {
			source([x, -x])
		})) ==
			stream.to_list(stream.flat_map(a, (x) {
				stream.flat_map(expand(x), (y) { source([y, -y]) })
			}))
	)
	list.each(list.range(-2, list.length(xs) + 3), (n) {
		assert(stream.to_list(stream.take(a, n)) == list.take(xs, n))
		assert(stream.to_list(stream.drop(a, n)) == list.drop(xs, n))
	})
	Nil
}

test("stream_laws", () {
	lists = [[], [1], [1, 2, 3], [3, -1, 3, 0]]
	list.each(lists, (xs) {
		list.each(lists, (ys) {
			list.each(lists, (zs) { check_laws(xs, ys, zs) })
		})
	})
	// Independent new calls instantiate different element types.
	assert(stream.to_list(stream.append(stream.new(), source([1]))) == [1])
	assert(
		stream.to_list(stream.append(stream.new(), stream.wrap("x"))) == ["x"]
	)
	Nil
})

// S1-S7 example table: failure is distinct from end, with no tail prefetch.

test("stream_examples", () {
	assert(stream.to_list(stream.defer(() { stream.wrap(7) })) == [7])
	assert(stream.to_list(stream.map(source([1, 2]), (x) { x * 2 })) == [2, 4])
	assert(stream.to_list(stream.drop(source([1, 2, 3]), 2)) == [3])
	assert(stream.to_list(stream.drop(source([1, 2]), -2)) == [1, 2])
	assert(
		stream.to_list(stream.filter(source([1, 2, 3]), (x) { x % 2 == 0 })) ==
			[2]
	)
	assert(stream.to_list(stream.filter_map(source([1, 2, 3]), (x) {
		{ x == 2 } |> {
			True  -> Some(20)
			False -> None
		}
	})) == [20])
	assert(
		stream.to_list(stream.flat_map(source([1, 2]), (x) {
			source([x, x])
		})) ==
			[1, 1, 2, 2]
	)
	assert(
		stream.to_list(
			stream.take_while(stream.append(source([1, 3]), broken()), (x) {
				x < 3
			})
		) ==
			[1]
	)
	assert(stream.to_list(stream.take(broken(), 0)) == [])
	assert(stream.to_list(stream.take(broken(), -1)) == [])
	assert(
		stream.to_list(
			stream.take(stream.append(stream.wrap(1), broken()), 1)
		) ==
			[1]
	)
	assert(stream.to_list(stream.zip(stream.new(), broken())) == [])
	assert(
		fail.catch(() { stream.to_list(stream.zip(broken(), stream.new())) }) ==
			Error("bad")
	)
	assert(stream.fold(source([1, 2, 3]), 0, (acc, x) { acc + x }) == 6)
	assert(stream.each(source([1, 2]), (x) { x * 2 }) == Nil)
	assert(
		stream.fold_until(stream.append(source([1, 2]), broken()), 0, (acc, x) {
			{ x == 2 } |> {
				True  -> list.Stop(acc + x)
				False -> list.Continue(acc + x)
			}
		}) == 3
	)
	rs = [Ok(1), Error("bad"), Ok(3)]
	assert(
		fail.catch(() { stream.to_list(stream.from_results(rs)) }) ==
			Error("bad")
	)
	assert(
		stream.to_list(stream.attempt(stream.from_results(rs))) ==
			[Ok(1), Error("bad")]
	)
	assert(stream.to_list(stream.from_list(rs)) == rs)
	assert(stream.to_list(stream.attempt(source([1, 2]))) == [Ok(1), Ok(2)])
	assert(stream.to_list(stream.attempt(broken())) == [Error("bad")])
	Nil
})

Trace {
	Trace(visit: (Int) -> Nil)
}

visit = (x: Int, trace ! Trace) -> Nil ! Trace { trace.visit(x) }

traced = (xs: List(Int)) -> stream.Stream(Int, Never, Trace) {
	stream.unfold(xs, (rest: List(Int)) {
		rest |> {
			[] -> {
				visit(0)
				None
			}
			[x, ..tail] -> {
				visit(x)
				Some((x, tail))
			}
		}
	})
}

test("stream_demand", () {
	trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		s = traced([1, 2, 3, 1])
		// Construct every transformation before any demand.
		stream.map(s, (x) { visit(x + 10) })
		stream.filter(s, (x) {
			visit(x + 20)
			True
		})
		stream.drop(s, 3)
		stream.flat_map(s, (x) {
			visit(x + 30)
			stream.wrap(x)
		})
		stream.defer(() {
			visit(99)
			stream.wrap(1)
		})
		assert(stream.to_list(stream.take(s, 0)) == [])
		assert(stream.to_list(stream.take(s, 1)) == [1])
		assert(stream.to_list(stream.take_while(s, (x) {
			visit(x + 10)
			x < 3
		})) == [1, 2])
		[]
	})
	assert(trace == [1, 1, 11, 2, 12, 3, 13])
	// Left-first zip discards precisely one unmatched left element.
	zip_trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		assert(
			stream.to_list(stream.zip(traced([1, 2]), traced([9]))) == [(1, 9)]
		)
		[]
	})
	assert(zip_trace == [1, 9, 2, 0])
	Nil
})

test("stream_replay_and_services", () {
	s = positive()
	assert({
		! Bias = Bias(value: 10)
		head(s)
	} == Some(10))
	assert({
		! Bias = Bias(value: 20)
		head(s)
	} == Some(20))
	assert({
		! Bias = Bias(value: 30)
		stream.to_list(needs_bias())
	} == [])
	a = source([1, 2])
	assert(head(a) == Some(1))
	assert(head(a) == Some(1))
	stream.pull(a) |> {
		stream.Done          -> panic("missing first step")
		stream.Next(_, tail) -> assert(stream.to_list(tail) == [2])
	}
	// Runner supplies Bias for each attempted input pull, including Done.
	run = (
		body: () -> stream.Step(Int, Never, Bias) ! Bias
	) -> stream.Step(Int, Never, Bias) {
		! Bias = Bias(value: 10)
		body()
	}
	adapted = stream.map_environment(s, run)
	assert(stream.to_list(adapted) == [10])
	assert(head(adapted) == Some(10))
	assert(head(adapted) == Some(10))
	Nil
})

Scale {
	Scale(value: Int)
}

common = () -> stream.Stream(Int, Never, Bias | Scale) {
	stream.make(
		(b ! Bias) -> stream.Step(Int, Never, Bias | Scale) ! Bias | Scale {
			stream.Next(b.value, stream.new())
		}
	)
}

test("stream_common_environment", () {
	s = stream.map(common(), (x: Int, scale ! Scale) { x * scale.value })
	! Bias = Bias(value: 10)
	! Scale = Scale(value: 3)
	assert(stream.to_list(s) == [30])
	Nil
})

OuterError {
	OuterError
}

InnerError {
	InnerError
}

CallbackError {
	CallbackError
}

test("stream_error_positions", () {
	prefix = stream.append(source([1, 2]), broken())
	assert(
		stream.to_list(stream.attempt(prefix)) == [Ok(1), Ok(2), Error("bad")]
	)
	assert(fail.catch(() { stream.to_list(prefix) }) == Error("bad"))
	assert(fail.catch(() { stream.to_list(prefix) }) == Error("bad"))
	assert(fail.catch(() {
		stream.to_list(stream.map(source([1, 2, 3]), (x) {
			{ x == 2 } |> {
				True  -> fail.raise(CallbackError)
				False -> x
			}
		}))
	}) == Error(CallbackError))
	outer = stream.from_results([Ok(1), Error(OuterError)])
	f = (x: Int) -> stream.Stream(Int, InnerError, Never) ^ CallbackError {
		{ x < 0 } |> {
			True  -> fail.raise(CallbackError)
			False -> stream.from_results([Ok(x), Error(InnerError)])
		}
	}
	assert(
		stream.to_list(stream.attempt(stream.flat_map(outer, f))) ==
			[Ok(1), Error(InnerError)]
	)
	assert(
		stream.to_list(stream.attempt(stream.flat_map(source([-1]), f))) ==
			[Error(CallbackError)]
	)
	assert(
		stream.to_list(stream.attempt(stream.flat_map(outer, (x) {
			source([x])
		}))) ==
			[Ok(1), Error(OuterError)]
	)
	runner = (
		body: () -> stream.Step(Int, String, Never) ^ String
	) -> stream.Step(Int, String, Never) ^ String {
		body()
		fail.raise("runner")
	}
	assert(
		stream.to_list(
			stream.attempt(
				stream.map_environment(
					stream.from_results([Ok(1), Error("source")])
					runner
				)
			)
		) ==
			[Error("runner")]
	)
	Nil
})

test("stream_long_searches", () {
	numbers = stream.unfold(0, (n: Int) { Some((n, n + 1)) })
	assert(stream.to_list(stream.take(numbers, 3)) == [0, 1, 2])
	assert(
		stream.to_list(
			stream.take(stream.filter(numbers, (n) { n >= 1000000 }), 1)
		) ==
			[1000000]
	)
	assert(
		stream.to_list(stream.take(stream.drop(numbers, 1000000), 1)) ==
			[1000000]
	)
	assert(stream.to_list(stream.take(stream.flat_map(numbers, (n) {
		{ n < 1000000 } |> {
			True  -> source([])
			False -> source([n])
		}
	}), 1)) == [1000000])
	xs = list.range(0, 20000)
	assert(stream.to_list(source(xs)) == xs)
	// Depth-first demand never reaches another outer item for an infinite inner.
	assert(
		stream.to_list(
			stream.take(stream.flat_map(source([1, 2]), (_) { numbers }), 3)
		) ==
			[0, 1, 2]
	)
	Nil
})

// S2: each attempted pull, including Done and repeated positions, reruns runner.

test("stream_runner_demand", () {
	trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		run = (
			body: () -> stream.Step(Int, Never, Bias) ! Bias
		) -> stream.Step(Int, Never, Bias) ! Trace {
			visit(99)
			! Bias = Bias(value: 10)
			body()
		}
		adapted = stream.map_environment(positive(), run)
		assert(stream.to_list(adapted) == [10])
		assert(head(adapted) == Some(10))
		assert(head(adapted) == Some(10))
		[]
	})
	assert(trace == [99, 99, 99, 99])
	Nil
})

reject = (x: Int) -> Int ^ String {
	{ x == 2 } |> {
		True  -> fail.raise("callback")
		False -> x
	}
}

test("stream_callback_failures", () {
	s = source([1, 2, 3])
	assert(
		stream.to_list(stream.attempt(stream.filter(s, (x) {
			reject(x) > 0
		}))) ==
			[Ok(1), Error("callback")]
	)
	assert(
		stream.to_list(stream.attempt(stream.filter_map(s, (x) {
			Some(reject(x))
		}))) ==
			[Ok(1), Error("callback")]
	)
	assert(
		stream.to_list(stream.attempt(stream.take_while(s, (x) {
			reject(x) > 0
		}))) ==
			[Ok(1), Error("callback")]
	)
	assert(
		fail.catch(() { stream.fold(s, 0, (acc, x) { acc + reject(x) }) }) ==
			Error("callback")
	)
	assert(
		fail.catch(() {
			stream.fold_until(s, 0, (acc, x) { list.Continue(acc + reject(x)) })
		}) ==
			Error("callback")
	)
	assert(fail.catch(() { stream.each(s, reject) }) == Error("callback"))
	assert(
		fail.catch(() {
			stream.to_list(stream.drop(stream.append(s, broken()), 4))
		}) ==
			Error("bad")
	)
	assert(
		stream.to_list(stream.attempt(stream.unfold(1, (n: Int) {
			Some((reject(n), n + 1))
		}))) ==
			[Ok(1), Error("callback")]
	)
	assert(
		stream.to_list(stream.attempt(stream.defer(() {
			fail.raise("deferred")
		}))) ==
			[Error("deferred")]
	)
	runner = (
		body: () -> stream.Step(Int, String, Never) ^ String
	) -> stream.Step(Int, String, Never) ^ String {
		body()
		fail.raise("runner")
	}
	assert(
		stream.to_list(
			stream.attempt(
				stream.map_environment(
					stream.from_results([Error("source")])
					runner
				)
			)
		) ==
			[Error("source")]
	)
	Nil
})

// S3/S6: callbacks run in order, and no callback after a failing one is called.

test("stream_callback_order", () {
	trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		s = stream.map(traced([1, 2, 3]), (x) -> Int ^ String ! Trace {
			visit(x + 10)
			reject(x)
		})
		assert(stream.to_list(stream.attempt(s)) == [Ok(1), Error("callback")])
		[]
	})
	assert(trace == [1, 11, 2, 12])
	Nil
})

// S8: the caller keeps the scope through consumption; early termination does
// not close a resource itself. S7: attempt does not intercept Assert failures.

test("stream_resource_scope", () {
	trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		resource.ensure(() {
			visit(100)
			s = traced([1, 2, 3])
			assert(stream.to_list(stream.take(s, 1)) == [1])
			visit(200)
		}, () { visit(300) })
		[]
	})
	assert(trace == [100, 1, 200, 300])
	result = assertion.catch(() {
		s = stream.make(() -> stream.Step(Int, Never, Assert) ! Assert {
			assert(False)
			stream.Done
		})
		stream.to_list(stream.attempt(s))
	})
	assert(result |> {
		Error(_) -> True
		Ok(_)    -> False
	})
	Nil
})

Item {
	Item(value: Int)
}

Count {
	Count(total: Int)
}

test("stream_nominal_callbacks", () {
	s: stream.Stream(Item, Never, Never) = stream.from_list([Item(value: 2), Item(value: 3)])
	assert(stream.to_list(stream.map(s, (x) { x.value })) == [2, 3])
	assert(
		stream.fold(s, Count(total: 0), (acc, x) {
			Count(total: acc.total + x.value)
		}).total ==
			5
	)
	assert(stream.to_list(stream.unfold(Count(total: 0), (state) {
		{ state.total < 3 } |> {
			True  -> Some((Item(value: state.total), Count(total: state.total + 1)))
			False -> None
		}
	})) == [Item(value: 0), Item(value: 1), Item(value: 2)])
	Nil
})

// Checker cases required by std-1 §3.3/§3.4 and §9. They are temporary additions
// to this owned test file, never failing std tests. Each must report E0501;
// restore the original file even if checking fails. Run from the lane with:
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/stream_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import subprocess
//| from pathlib import Path
//| path = Path('std/test/stream_test.i')
//| original = path.read_text()
//| cases = {
//|     'invariant environment (§3.4)': '''
//| Sink(A) { Sink(put: (A) -> Nil) }
//| bad_coerce = (source: stream.Stream(Int, Never, Sink(Int))) -> stream.Stream(Int, Never, Sink(Never)) {
//|   source
//| }
//| ''',
//|     'pull requires Bias (S2)': 'bad = () -> List(Int) { stream.to_list(positive()) }',
//|     'runner cannot erase Bias (§3.3)': '''
//| bad_runner = (body: () -> stream.Step(Int, Never, Bias) ! Bias) -> stream.Step(Int, Never, Bias) {
//|   body()
//| }
//| ''',
//|     'pull exposes errors (S7)': 'bad = () -> List(Int) { stream.to_list(broken()) }',
//| }
//| try:
//|     for name, source in cases.items():
//|         path.write_text(original + '\n' + source + '\n')
//|         result = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//|         print(name, result.returncode, result.stderr, flush=True)
//|         assert result.returncode == 1 and 'error[E0501]' in result.stderr
//| finally:
//|     path.write_text(original)

// S1/S2/S7: defer and unfold run only on pulls; retry explicitly repeats work.

test("stream_factory_demand", () {
	trace = control.boundary((b) {
		! Trace = Trace(visit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		d = stream.defer(() -> stream.Stream(Int, Never, Trace) ! Trace {
			visit(99)
			stream.wrap(7)
		})
		u = stream.unfold(0, (n: Int) -> option.Option((Int, Int)) ! Trace {
			visit(n)
			{ n < 2 } |> {
				True  -> Some((n, n + 1))
				False -> None
			}
		})
		failing = stream.make(
			() -> stream.Step(Int, String, Trace) ^ String ! Trace {
				visit(88)
				fail.raise("bad")
			}
		)
		assert(head(d) == Some(7))
		assert(head(d) == Some(7))
		assert(stream.to_list(u) == [0, 1])
		assert(fail.catch(() { head(failing) }) == Error("bad"))
		assert(fail.catch(() { head(failing) }) == Error("bad"))
		[]
	})
	assert(trace == [99, 99, 0, 1, 2, 88, 88])
	Nil
})
