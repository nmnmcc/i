//// Replayable pull streams. Construction is pure; each pull repeats its work
//// with the services at that call. No stream owns or closes a resource: keep
//// the resource scope around consumption, including early termination.
//// Skipping may consume unbounded input in one pull. Searches and folds use
//// tail calls; collection reverses once. Composition chains still add depth.
//// Ordinary equality compares representation, not observations. There is no
//// memoization, fusion guarantee, or implicit cleanup when a value is dropped.
(
	i/fail
	std/list
	(Option, Some, None) = std/option
)

(
	Stream
	Step(..)
	new
	wrap
	from_list
	make
	defer
	unfold
	pull
	map
	filter
	filter_map
	flat_map
	append
	zip
	take
	drop
	take_while
	fold
	fold_until
	each
	to_list
	from_results
	attempt
	map_environment
)

Stream(A, E, N) = (() <- (Fail(E), ..N) -> Step(A, E, N), (N) -> Nil)

Done = ()
Next(A, E, N) = (A, Stream(A, E, N))
Step(A, E, N) = Done | Next(A, E, N)

/// An empty stream in any chosen common environment.
new = <A, N>() -> Stream(A, Never, N) {
	Stream(() -> Step(A, Never, N) { Done }, (_ : N) { Nil })
}

/// One item, followed by an empty tail.
wrap = <A, N>(item : A) -> Stream(A, Never, N) {
	make(() { Next(item, new()) })
}

/// Builds one cell per pull, without walking the list at construction.
from_list = <A, N>(items : List(A)) -> Stream(A, Never, N) {
	make(() { items |> ([]) { Done } | ([x, ..xs]) { Next(x, from_list(xs)) } })
}

/// Stores a step without calling it. The private marker makes N invariant.
make = <A, E, N>(step : () <- (Fail(E), ..N) -> Step(A, E, N)) -> Stream(
	A,
	E,
	N,
) { Stream(step, (_ : N) { Nil }) }

/// Defers both construction and the first pull until demand.
defer = <A, E, N>(body : () <- (Fail(E), ..N) -> Stream(A, E, N)) -> Stream(
	A,
	E,
	N,
) { make(() { pull(body()) }) }

/// Calls the state transition once per pull.
unfold = <S, A, E, N>(
	from : S,
	with : (S) <- (Fail(E), ..N) -> Option((A, S))
) -> Stream(A, E, N) {
	make(() {
		with(from) |>
			(None) { Done }
			| (Some((x, next))) { Next(x, unfold(next, with)) }
	})
}

/// Calls the stored step once. Repeated pulls do not cache or advance.
pull = <A, E, N>(stream : Stream(A, E, N)) -> Step(A, E, N) {
	Stream(step, _) = stream
	step()
}

/// Transforms only the demanded cell.
map = <A, B, E, F, N>(
	over : Stream(A, E, N),
	with : (A) <- (Fail(F), ..N) -> B
) -> Stream(B, E | F, N) {
	make(() {
		pull(over) |>
			(Done) { Done }
			| (Next(x, tail)) { Next(with(x), map(tail, with)) }
	})
}

/// Seeks the next retained item using tail calls.
filter = <A, E, F, N>(
	over : Stream(A, E, N),
	keeping : (A) <- (Fail(F), ..N) -> Bool
) -> Stream(A, E | F, N) {
	filter_map(over, (x) {
		keeping(x) |> (True) { Some(x) } | (False) { None }
	})
}

/// Seeks the next present mapped item using tail calls.
filter_map = <A, B, E, F, N>(
	over : Stream(A, E, N),
	with : (A) <- (Fail(F), ..N) -> Option(B)
) -> Stream(B, E | F, N) {
	seek = (source : Stream(A, E, N)) -> Step(B, E | F, N) {
		pull(source) |>
			(Done) { Done }
			| (Next(x, tail)) {
				with(x) |>
					(None) { seek(tail) }
					| (Some(y)) { Next(y, filter_map(tail, with)) }
			}
	}
	make(() { seek(over) })
}

/// Drains each inner stream before demanding another outer item.
flat_map = <A, B, E, F, G, N>(
	over : Stream(A, E, N),
	with : (A) <- (Fail(G), ..N) -> Stream(B, F, N)
) -> Stream(B, E | F | G, N) { flatten(over, new(), with) }

/// Exhausts first before pulling second; arguments themselves are strict.
append = <A, E, F, N>(
	first : Stream(A, E, N),
	second : Stream(A, F, N)
) -> Stream(A, E | F, N) {
	make(() {
		pull(first) |>
			(Done) { pull(second) }
			| (Next(x, tail)) { Next(x, append(tail, second)) }
	})
}

/// Pulls left first. An empty left never pulls right.
zip = <A, B, E, F, N>(
	first : Stream(A, E, N),
	second : Stream(B, F, N)
) -> Stream((A, B), E | F, N) {
	make(() {
		pull(first) |>
			(Done) { Done }
			| (Next(x, xs)) {
				pull(second) |>
					(Done) { Done }
					| (Next(y, ys)) { Next((x, y), zip(xs, ys)) }
			}
	})
}

/// Nonpositive bounds never pull the source, nor is the kth tail prefetched.
take = <A, E, N>(from : Stream(A, E, N), up_to : Int) -> Stream(A, E, N) {
	make(() {
		{ up_to <= 0 } |>
			(True) { Done }
			| (False) {
				pull(from) |>
					(Done) { Done }
					| (Next(x, tail)) { Next(x, take(tail, up_to - 1)) }
			}
	})
}

/// Skips only on demand; negative counts skip nothing.
drop = <A, E, N>(from : Stream(A, E, N), up_to : Int) -> Stream(A, E, N) {
	seek = (source : Stream(A, E, N), n : Int) -> Step(A, E, N) {
		{ n <= 0 } |>
			(True) { pull(source) }
			| (False) {
				pull(source) |>
					(Done) { Done }
					| (Next(_, tail)) { seek(tail, n - 1) }
			}
	}
	make(() { seek(from, up_to) })
}

/// Consumes the first rejected item, without pulling its tail.
take_while = <A, E, F, N>(
	from : Stream(A, E, N),
	keeping : (A) <- (Fail(F), ..N) -> Bool
) -> Stream(A, E | F, N) {
	make(() {
		pull(from) |>
			(Done) { Done }
			| (Next(x, tail)) {
				keeping(x) |>
					(False) { Done }
					| (True) { Next(x, take_while(tail, keeping)) }
			}
	})
}

/// Consumes in order, passing (accumulator, element).
fold = <A, Acc, E, N>(
	over : Stream(A, E, N),
	from : Acc,
	with : (Acc, A) <- N -> Acc
) -> Acc {
	pull(over) |>
		(Done) { from }
		| (Next(x, tail)) { fold(tail, with(from, x), with) }
}

/// Stop returns immediately without demanding the tail.
fold_until = <A, Acc, E, N>(
	over : Stream(A, E, N),
	from : Acc,
	with : (Acc, A) <- N -> list.ContinueOrStop(Acc)
) -> Acc {
	pull(over) |>
		(Done) { from }
		| (Next(x, tail)) {
			with(from, x) |>
				(list.Stop(acc)) { acc }
				| (list.Continue(acc)) { fold_until(tail, acc, with) }
		}
}

/// Discards callback results; an infinite input may never finish.
each = <A, B, E, N>(over : Stream(A, E, N), with : (A) <- N -> B) -> Nil {
	fold(over, Nil, (_, x) {
		with(x)
		Nil
	})
}

/// Collects in order with one reversal, using O(k) space.
to_list = <A, E, N>(stream : Stream(A, E, N)) -> List(A) {
	list.reverse(fold(stream, [], (acc, x) { [x, ..acc] }))
}

/// Raises the first Error when its position is pulled.
from_results = <A, E, N>(items : List(Result(A, E))) -> Stream(A, E, N) {
	make(() {
		items |>
			([]) { Done }
			| ([x, ..xs]) { Next(fail.from_result(x), from_results(xs)) }
	})
}

/// Emits one error value on failure and then ends. Does not catch Assert.
attempt = <A, E, N>(stream : Stream(A, E, N)) -> Stream(
	Result(A, E),
	Never,
	N,
) {
	make(() {
		fail.catch(() { pull(stream) }) |>
			(Error(e)) { Next(Error(e), new()) }
			| (Ok(Done)) { Done }
			| (Ok(Next(x, tail))) { Next(Ok(x), attempt(tail)) }
	})
}

/// Runs each input pull through a fixed-result runner, rebuilding every tail.
map_environment = <A, E, N, M>(
	stream : Stream(A, E, N),
	run : (() <- (Fail(E), ..N) -> Step(A, E, N)) <- (Fail(E), ..M) -> Step(
		A,
		E,
		N,
	)
) -> Stream(A, E, M) {
	make(() {
		run(() { pull(stream) }) |>
			(Done) { Done }
			| (Next(x, tail)) { Next(x, map_environment(tail, run)) }
	})
}

flatten = <A, B, E, F, G, N>(
	outer : Stream(A, E, N),
	inner : Stream(B, F, N),
	f : (A) <- (Fail(G), ..N) -> Stream(B, F, N)
) -> Stream(B, E | F | G, N) {
	seek = (out : Stream(A, E, N), current : Stream(B, F, N)) -> Step(
		B,
		E | F | G,
		N,
	) {
		pull(current) |>
			(Next(x, tail)) { Next(x, flatten(out, tail, f)) }
			| (Done) {
				pull(out) |>
					(Done) { Done }
					| (Next(x, tail)) { seek(tail, f(x)) }
			}
	}
	make(() { seek(outer, inner) })
}
