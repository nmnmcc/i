//// Errors (§9.3): the service `Fail(E)` of the prelude, and ordinary
//// library code over `control.boundary` and `control.escape`.
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
raise = <E>(error: E) <- (handler: Fail(E)) -> Never { handler.raise(error) }

/// Gives `Ok(value)` when `body` ends with `value`, or `Error(error)` for the
/// first `error` it raises through `Fail(E)`.
catch: <A, E, N>(() <- (Fail(E), ..N) -> A) <- N -> Result(A, E) = (body) {
	control.boundary((b) {
		Ok(body()) <- (
			Fail(raise = (error) { control.escape(b, Error(error)) }),
		)
	})
}

/// Gives the error `x` for `Error(x)`.
from_result = <A, E>(result: Result(A, E)) -> A {
	result |> (Ok(value)) { value } | (Error(error)) { raise(error) }
}

/// Changes the errors.
map_error: <A, E, N, E2>(() <- (Fail(E), ..N) -> A, (E) <- N -> E2) <- (
	Fail(E2),
	..N,
) -> A = (body, with) {
	catch(body) |> (Ok(value)) { value } | (Error(error)) { raise(with(error)) }
}
