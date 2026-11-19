//// Errors: the service `Fail(E)` of the prelude, and ordinary
//// library code over `control.boundary` and `control.escape`.
////
//// This is the one API for typed errors. An operation that cannot give its
//// answer raises an error with `raise`; a caller takes the errors at a
//// boundary of its choice with `catch`, which gives a `Result`.
//// `from_result` raises the error of a stored `Error` again, and
//// `map_error` changes the errors on the way out. To handle only some
//// errors, `catch` them all, match, and `raise` the rest.
////
//// - A `Result` is a stored outcome, not a second way to fail; `i/result`
////   has the functions over it.
//// - `Assert` failures are a separate service: `catch` does not take them,
////   `i/assertion.catch` does.
//// - A stop (`panic`) is neither: no `catch` takes it, and final functions
////   do not run on it.
//// - A `Fail` used after its `catch` has ended stops the program with
////   `control: boundary is not active`.
(
	i/control
)

(
	raise
	catch
	from_result
	map_error
)

/// Ends the computation with `error`, through the current `Fail`.
raise = <E>(error : E) <- (handler : Fail(E)) -> Never { handler.raise(error) }

/// Gives `Ok(value)` when `body` ends with `value`, or `Error(error)` for the
/// first `error` it raises through `Fail(E)`. It does not take `Assert`
/// failures or a stop.
catch = <A, E, N>(body : () <- (Fail(E), ..N) -> A) -> Result(A, E) {
	control.boundary((b) {
		Ok(body()) <- (
			Fail(raise = (error) { control.escape(b, Error(error)) }),
		)
	})
}

/// The value of an `Ok`. For `Error(x)`, raises `x`: the way back from a
/// stored outcome to `Fail(E)`.
from_result = <A, E>(result : Result(A, E)) -> A {
	result |> (Ok(value)) { value } & (Error(error)) { raise(error) }
}

/// Changes the errors.
map_error : <A, E, N, E2>(() <- (Fail(E), ..N) -> A, (E) <- N -> E2) <- (
	Fail(E2),
	..N,
) -> A = (body, with) {
	catch(body) |> (Ok(value)) { value } & (Error(error)) { raise(with(error)) }
}
