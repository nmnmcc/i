//// Errors as a service (§9.3): `Fail(E)`, `raise` and `catch` are
//// ordinary library code over `control.boundary` and `control.escape`.

(
	i/control
)

(
	raise
	catch
)

/// Ends the computation with `error`, through the current `Fail`.
raise(E) = (error: E, fail ! Fail(E)) -> Never ! Fail(E) { fail.raise(error) }

/// Gives `Ok(value)` when `body` ends with `value`, or `Error(error)` for the
/// first `error` it raises through `Fail(E)`.
catch(A, E, N) = (body: () -> A ! Fail(E) | N) -> Result(A, E) ! N {
	control.boundary((b) {
		! Fail(E) = Fail((error) { control.escape(b, Error(error)) })
		Ok(body())
	})
}
