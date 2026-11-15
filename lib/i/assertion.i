//// Taking `Assert` failures.
(
	i/control
)

(
	catch
)

/// Gives `Error(failure)` for the first `Assert` failure in `body`.
catch = (body) {
	control.boundary((b) {
		Ok(body()) <- (
			Assert(
				fail = (failure) {
					control.suspend(b, (k) {
						control.cancel(k)
						Error(failure)
					})
				},
			),
		)
	})
}
