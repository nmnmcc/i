//// Optional values: `Some(value)` when there is a value, and `None` when
//// there is not.
////
//// In `std`, a function that may find nothing gives an `Option`, for
//// example `list.first` and `dict.get`. A function that may fail, such as
//// `int.parse`, gives a `Result`.
(
	Option(..)
	is_some
	is_none
	map
	then
	flatten
	unwrap
	lazy_unwrap
	or
	lazy_or
	to_result
	from_result
	all
	values
	filter
	map2
)

Some(A) = (A)
None = ()
/// A value that may be there (`Some`) or not (`None`).
Option(A) = Some(A) | None

/// `True` when the option is `Some`.
is_some = <A>(option: Option(A)) -> Bool {
	option |> (Some(_)) { True } | (None) { False }
}

/// `True` when the option is `None`.
is_none = <A>(option: Option(A)) -> Bool {
	option |> (Some(_)) { False } | (None) { True }
}

/// Changes the value in a `Some` with a function. `None` stays `None`.
///
/// ```i
/// option.map(Some(1), (x) { x + 1 })  // Some(2)
/// ```
map = <A, B, N>(over: Option(A), with: (A) <- N -> B) -> Option(B) {
	over |> (Some(value)) { Some(with(value)) } | (None) { None }
}

/// Calls a function that gives an option on the value in a `Some`. `None`
/// stays `None`. This puts together steps that may each find nothing.
///
/// ```i
/// option.then(Some(1), (x) { Some(x + 1) })  // Some(2)
/// ```
then = <A, B, N>(option: Option(A), apply: (A) <- N -> Option(B)) -> Option(B) {
	option |> (Some(value)) { apply(value) } | (None) { None }
}

/// Takes away one level of `Option`.
flatten = <A>(option: Option(Option(A))) -> Option(A) {
	option |> (Some(inner)) { inner } | (None) { None }
}

/// The value in a `Some`, or `default` for `None`.
unwrap = <A>(option: Option(A), or: A) -> A {
	option |> (Some(value)) { value } | (None) { or }
}

/// The value in a `Some`, or the value of `default()` for `None`.
lazy_unwrap = <A, N>(option: Option(A), or: () <- N -> A) -> A {
	option |> (Some(value)) { value } | (None) { or() }
}

/// The first option when it is `Some`, and the second when not.
or = <A>(first: Option(A), second: Option(A)) -> Option(A) {
	first |> (Some(_)) { first } | (None) { second }
}

/// The first option when it is `Some`, and the value of `second()` when not.
lazy_or = <A, N>(first: Option(A), second: () <- N -> Option(A)) -> Option(A) {
	first |> (Some(_)) { first } | (None) { second() }
}

/// `Ok` with the value of a `Some`, or `Error(error)` for `None`.
to_result = <A, E>(option: Option(A), error: E) -> Result(A, E) {
	option |> (Some(value)) { Ok(value) } | (None) { Error(error) }
}

/// `Some` with the value of an `Ok`, or `None` for an `Error`.
from_result = <A, E>(result: Result(A, E)) -> Option(A) {
	result |> (Ok(value)) { Some(value) } | (Error(_)) { None }
}

/// `Some` with all the values when every option is `Some`, and `None` when
/// one of them is `None`.
///
/// ```i
/// option.all([Some(1), Some(2)])  // Some([1, 2])
/// option.all([Some(1), None])     // None
/// ```
all = <A>(options: List(Option(A))) -> Option(List(A)) {
	go = (rest, acc) {
		rest |>
			([]) { Some(reverse(acc)) }
			| ([Some(x), ..xs]) { go(xs, [x, ..acc]) }
			| ([None, ..]) { None }
	}
	go(options, [])
}

/// The values of the `Some` options, in order.
///
/// ```i
/// option.values([Some(1), None, Some(3)])  // [1, 3]
/// ```
values = <A>(options: List(Option(A))) -> List(A) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([Some(x), ..xs]) { go(xs, [x, ..acc]) }
			| ([None, ..xs]) { go(xs, acc) }
	}
	go(options, [])
}

/// Keeps a `Some` only when its value passes the test.
filter = <A, N>(option: Option(A), keeping: (A) <- N -> Bool) -> Option(A) {
	option |>
		(Some(value)) { keeping(value) |> (True) { option } | (False) { None } }
		| (None) { None }
}

/// Puts the values of two options together when both are `Some`.
map2 = <A, B, C, N>(
	a: Option(A),
	b: Option(B),
	with: (A, B) <- N -> C
) -> Option(C) {
	(a, b) |> ((Some(x), Some(y))) { Some(with(x, y)) } | (_) { None }
}

reverse = (list) {
	go = (rest, acc) {
		rest |> ([]) { acc } | ([x, ..xs]) { go(xs, [x, ..acc]) }
	}
	go(list, [])
}
