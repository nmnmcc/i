//// Functions for the built-in type `Result(a, e) { Ok(a) Error(e) }`.
////
//// A `Result` is an error kept as a value. i also has errors as effects
//// (`? E` in a function type, `i/fail`): `to_error` and `catch` go from one
//// form to the other.

import (
	i/fail
	option = (Option, Some, None) = std/option
)

export (
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
is_ok = (result: Result(a, e)) -> Bool {
	case result {
		Ok(_)    -> True
		Error(_) -> False
	}
}

/// `True` for `Error`.
is_error = (result: Result(a, e)) -> Bool {
	case result {
		Ok(_)    -> False
		Error(_) -> True
	}
}

/// Changes the value of an `Ok` with a function. An `Error` stays.
///
/// ```i
/// result.map(Ok(1), (x) { x + 1 })  // Ok(2)
/// ```
map = (over: Result(a, e), with: (a) -> b ? e2 ! n) -> Result(b, e) ? e2 ! n {
	case over {
		Ok(value)    -> Ok(with(value))
		Error(error) -> Error(error)
	}
}

/// Changes the error of an `Error` with a function. An `Ok` stays.
map_error = (
	over: Result(a, e)
	with: (e) -> f ? e2 ! n
) -> Result(a, f) ? e2 ! n {
	case over {
		Ok(value)    -> Ok(value)
		Error(error) -> Error(with(error))
	}
}

/// Calls a function that gives a result on the value of an `Ok`. An `Error`
/// stays. This puts together steps that may each fail.
///
/// ```i
/// result.then(Ok(1), (x) { Ok(x + 1) })  // Ok(2)
/// ```
then = (
	result: Result(a, e)
	apply: (a) -> Result(b, e) ? e2 ! n
) -> Result(b, e) ? e2 ! n {
	case result {
		Ok(value)    -> apply(value)
		Error(error) -> Error(error)
	}
}

/// Takes away one level of `Result`.
flatten = (result: Result(Result(a, e), e)) -> Result(a, e) {
	case result {
		Ok(inner)    -> inner
		Error(error) -> Error(error)
	}
}

/// The value of an `Ok`, or `or` for an `Error`.
unwrap = (result: Result(a, e), or: a) -> a {
	case result {
		Ok(value) -> value
		Error(_)  -> or
	}
}

/// The value of an `Ok`, or the value of `or()` for an `Error`.
lazy_unwrap = (result: Result(a, e), or: () -> a ? e2 ! n) -> a ? e2 ! n {
	case result {
		Ok(value) -> value
		Error(_)  -> or()
	}
}

/// The error of an `Error`, or `or` for an `Ok`.
unwrap_error = (result: Result(a, e), or: e) -> e {
	case result {
		Ok(_)        -> or
		Error(error) -> error
	}
}

/// The value of an `Ok` or the error of an `Error`, when they have the same
/// type.
unwrap_both = (result: Result(a, a)) -> a {
	case result {
		Ok(value)    -> value
		Error(error) -> error
	}
}

/// The first result when it is `Ok`, and the second when not.
or = (first: Result(a, e), second: Result(a, e)) -> Result(a, e) {
	case first {
		Ok(_)    -> first
		Error(_) -> second
	}
}

/// The first result when it is `Ok`, and the value of `second()` when not.
lazy_or = (
	first: Result(a, e)
	second: () -> Result(a, e) ? e2 ! n
) -> Result(a, e) ? e2 ! n {
	case first {
		Ok(_)    -> first
		Error(_) -> second()
	}
}

/// `Ok` with all the values when every result is `Ok`, or the first `Error`.
///
/// ```i
/// result.all([Ok(1), Ok(2)])              // Ok([1, 2])
/// result.all([Ok(1), Error("a"), Error("b")])  // Error("a")
/// ```
all = (results: List(Result(a, e))) -> Result(List(a), e) {
	go = (rest, acc) {
		case rest {
			[]                 -> Ok(reverse(acc))
			[Ok(x), ..xs]      -> go(xs, [x, ..acc])
			[Error(error), ..] -> Error(error)
		}
	}
	go(results, [])
}

/// The values of the `Ok` results and the errors of the `Error` results,
/// each in order.
///
/// ```i
/// result.partition([Ok(1), Error("a"), Ok(2)])  // ([1, 2], ["a"])
/// ```
partition = (results: List(Result(a, e))) -> (List(a), List(e)) {
	go = (rest, oks, errors) {
		case rest {
			[]               -> (reverse(oks), reverse(errors))
			[Ok(x), ..xs]    -> go(xs, [x, ..oks], errors)
			[Error(x), ..xs] -> go(xs, oks, [x, ..errors])
		}
	}
	go(results, [], [])
}

/// The values of the `Ok` results, in order.
values = (results: List(Result(a, e))) -> List(a) {
	go = (rest, acc) {
		case rest {
			[]               -> reverse(acc)
			[Ok(x), ..xs]    -> go(xs, [x, ..acc])
			[Error(_), ..xs] -> go(xs, acc)
		}
	}
	go(results, [])
}

/// Puts `value` in the place of the value of an `Ok`.
replace = (result: Result(a, e), value: b) -> Result(b, e) {
	case result {
		Ok(_)        -> Ok(value)
		Error(error) -> Error(error)
	}
}

/// Puts `error` in the place of the error of an `Error`.
replace_error = (result: Result(a, e), error: f) -> Result(a, f) {
	case result {
		Ok(value) -> Ok(value)
		Error(_)  -> Error(error)
	}
}

/// Calls a function on the error of an `Error`, which may give a new
/// result. An `Ok` stays.
recover = (
	result: Result(a, e)
	with: (e) -> Result(a, f) ? e2 ! n
) -> Result(a, f) ? e2 ! n {
	case result {
		Ok(value)    -> Ok(value)
		Error(error) -> with(error)
	}
}

/// `Ok` with the value of a `Some`, or `Error(error)` for `None`.
from_option = (option: Option(a), error: e) -> Result(a, e) {
	case option {
		Some(value) -> Ok(value)
		None        -> Error(error)
	}
}

/// `Some` with the value of an `Ok`, or `None` for an `Error`.
to_option = (result: Result(a, e)) -> Option(a) {
	case result {
		Ok(value) -> Some(value)
		Error(_)  -> None
	}
}

/// The value of an `Ok`. For `Error(x)`, the computation ends with the
/// error `x` (§9.3), which `catch` or `i/fail` can take.
to_error = (result: Result(a, e)) -> a ? e { fail.from_result(result) }

/// Does `body`, and gives `Ok` with its value, or `Error` with the error it
/// ends with.
///
/// ```i
/// result.catch(() { fail.raise("no") })  // Error("no")
/// ```
catch = (body: () -> a ? e ! n) -> Result(a, e) ! n { fail.catch(body) }

reverse = (list) {
	go = (rest, acc) {
		case rest {
			[]        -> acc
			[x, ..xs] -> go(xs, [x, ..acc])
		}
	}
	go(list, [])
}
