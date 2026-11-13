//// Functions for the built-in type `Result(A, E) { Ok(A) Error(E) }`.
////
//// A `Result` is an error kept as a value. i also has errors as effects
//// (`^ E` in a function type, `i/fail`): `to_error` and `catch` go from one
//// form to the other.
(
	i/fail
	option = (Option, Some, None) = std/option
)

(
	is_ok
	is_error
	map
	map_error
	then
	flatten
	unwrap
	lazy_unwrap
	unwrap_error
	unwrap_both
	or
	lazy_or
	all
	partition
	values
	replace
	replace_error
	recover
	from_option
	to_option
	to_error
	catch
)

/// `True` for `Ok`.
is_ok = <A, E>(result : Result(A, E)) -> Bool {
	result |> (Ok(_)) { True } | (Error(_)) { False }
}

/// `True` for `Error`.
is_error = <A, E>(result : Result(A, E)) -> Bool {
	result |> (Ok(_)) { False } | (Error(_)) { True }
}

/// Changes the value of an `Ok` with a function. An `Error` stays.
///
/// ```i
/// result.map(Ok(1), (x) { x + 1 })  // Ok(2)
/// ```
map = <A, E, B, N>(over : Result(A, E), with : (A) <- N -> B) -> Result(B, E) {
	over |> (Ok(value)) { Ok(with(value)) } | (Error(error)) { Error(error) }
}

/// Changes the error of an `Error` with a function. An `Ok` stays.
map_error = <A, E, F, N>(over : Result(A, E), with : (E) <- N -> F) -> Result(
	A,
	F,
) { over |> (Ok(value)) { Ok(value) } | (Error(error)) { Error(with(error)) } }

/// Calls a function that gives a result on the value of an `Ok`. An `Error`
/// stays. This puts together steps that may each fail.
///
/// ```i
/// result.then(Ok(1), (x) { Ok(x + 1) })  // Ok(2)
/// ```
then = <A, E, B, N>(
	result : Result(A, E),
	apply : (A) <- N -> Result(B, E)
) -> Result(B, E) {
	result |> (Ok(value)) { apply(value) } | (Error(error)) { Error(error) }
}

/// Takes away one level of `Result`.
flatten = <A, E>(result : Result(Result(A, E), E)) -> Result(A, E) {
	result |> (Ok(inner)) { inner } | (Error(error)) { Error(error) }
}

/// The value of an `Ok`, or `or` for an `Error`.
unwrap = <A, E>(result : Result(A, E), or : A) -> A {
	result |> (Ok(value)) { value } | (Error(_)) { or }
}

/// The value of an `Ok`, or the value of `or()` for an `Error`.
lazy_unwrap = <A, E, N>(result : Result(A, E), or : () <- N -> A) -> A {
	result |> (Ok(value)) { value } | (Error(_)) { or() }
}

/// The error of an `Error`, or `or` for an `Ok`.
unwrap_error = <A, E>(result : Result(A, E), or : E) -> E {
	result |> (Ok(_)) { or } | (Error(error)) { error }
}

/// The value of an `Ok` or the error of an `Error`, when they have the same
/// type.
unwrap_both = <A>(result : Result(A, A)) -> A {
	result |> (Ok(value)) { value } | (Error(error)) { error }
}

/// The first result when it is `Ok`, and the second when not.
or = <A, E>(first : Result(A, E), second : Result(A, E)) -> Result(A, E) {
	first |> (Ok(_)) { first } | (Error(_)) { second }
}

/// The first result when it is `Ok`, and the value of `second()` when not.
lazy_or = <A, E, N>(
	first : Result(A, E),
	second : () <- N -> Result(A, E)
) -> Result(A, E) { first |> (Ok(_)) { first } | (Error(_)) { second() } }

/// `Ok` with all the values when every result is `Ok`, or the first `Error`.
///
/// ```i
/// result.all([Ok(1), Ok(2)])              // Ok([1, 2])
/// result.all([Ok(1), Error("a"), Error("b")])  // Error("a")
/// ```
all = <A, E>(results : List(Result(A, E))) -> Result(List(A), E) {
	go = (rest, acc) {
		rest |>
			([]) { Ok(reverse(acc)) }
			| ([Ok(x), ..xs]) { go(xs, [x, ..acc]) }
			| ([Error(error), ..]) { Error(error) }
	}
	go(results, [])
}

/// The values of the `Ok` results and the errors of the `Error` results,
/// each in order.
///
/// ```i
/// result.partition([Ok(1), Error("a"), Ok(2)])  // ([1, 2], ["a"])
/// ```
partition = <A, E>(results : List(Result(A, E))) -> (List(A), List(E)) {
	go = (rest, oks, errors) {
		rest |>
			([]) { (reverse(oks), reverse(errors)) }
			| ([Ok(x), ..xs]) { go(xs, [x, ..oks], errors) }
			| ([Error(x), ..xs]) { go(xs, oks, [x, ..errors]) }
	}
	go(results, [], [])
}

/// The values of the `Ok` results, in order.
values = <A, E>(results : List(Result(A, E))) -> List(A) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([Ok(x), ..xs]) { go(xs, [x, ..acc]) }
			| ([Error(_), ..xs]) { go(xs, acc) }
	}
	go(results, [])
}

/// Puts `value` in the place of the value of an `Ok`.
replace = <A, E, B>(result : Result(A, E), value : B) -> Result(B, E) {
	result |> (Ok(_)) { Ok(value) } | (Error(error)) { Error(error) }
}

/// Puts `error` in the place of the error of an `Error`.
replace_error = <A, E, F>(result : Result(A, E), error : F) -> Result(A, F) {
	result |> (Ok(value)) { Ok(value) } | (Error(_)) { Error(error) }
}

/// Calls a function on the error of an `Error`, which may give a new
/// result. An `Ok` stays.
recover = <A, E, F, N>(
	result : Result(A, E),
	with : (E) <- N -> Result(A, F)
) -> Result(A, F) {
	result |> (Ok(value)) { Ok(value) } | (Error(error)) { with(error) }
}

/// `Ok` with the value of a `Some`, or `Error(error)` for `None`.
from_option = <A, E>(option : Option(A), error : E) -> Result(A, E) {
	option |> (Some(value)) { Ok(value) } | (None) { Error(error) }
}

/// `Some` with the value of an `Ok`, or `None` for an `Error`.
to_option = <A, E>(result : Result(A, E)) -> Option(A) {
	result |> (Ok(value)) { Some(value) } | (Error(_)) { None }
}

/// The value of an `Ok`. For `Error(x)`, the computation ends with the
/// error `x` (§9.3), which `catch` or `i/fail` can take.
to_error = <A, E>(result : Result(A, E)) -> A { fail.from_result(result) }

/// Does `body`, and gives `Ok` with its value, or `Error` with the error it
/// ends with.
///
/// ```i
/// result.catch(() { fail.raise("no") })  // Error("no")
/// ```
catch = <A, E, N>(body : () <- (Fail(E), ..N) -> A) -> Result(A, E) {
	fail.catch(body)
}

reverse = (list) {
	go = (rest, acc) {
		rest |> ([]) { acc } | ([x, ..xs]) { go(xs, [x, ..acc]) }
	}
	go(list, [])
}
