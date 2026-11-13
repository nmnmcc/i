//// Errors as a service (§9.3): `Fail(E)`, `raise` and `catch` are
//// ordinary library code over `control.boundary` and `control.escape`.
(
	i/control
)

(
	raise
	catch
)

Syn3ServiceFail(E) = Fail(E)
/// Ends the computation with `error`, through the current `Fail`.
raise = <E>(error: E) <- (fail: Fail(E)) -> Never { fail.raise(error) }

/// Gives `Ok(value)` when `body` ends with `value`, or `Error(error)` for the
/// first `error` it raises through `Fail(E)`.
catch: <A, E, N>(
	() <- (Fail(Never), Syn3ServiceFail(E), ..N) -> A,
) <- N -> Result(A, E) = (body) {
	control.boundary((b) {
		Ok(body()) <- (Fail((error) { control.escape(b, Error(error)) }))
	})
}
