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
raise = <E>(error: E) <- (fail: Syn3ServiceFail(E)) -> Never {
	fail.raise(error)
}

/// Gives `Ok(value)` when `body` ends with `value`, or `Error(error)` for the
/// first `error` it raises through `Fail(E)`.
catch = <A, E, N>(body: () <- (Fail(Never), Syn3ServiceFail(E), ..N) -> A) -> Result(A, E) {
	control.boundary((b) {
		Ok(body()) <- (Fail(raise = (error: E) { control.escape(b, Error(error)) }))
	})
}
