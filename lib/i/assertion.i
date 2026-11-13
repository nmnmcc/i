//// Taking `Assert` failures (§9.6).
(
	i/control
)

(
	catch
)

/// Gives `Error(failure)` for the first `Assert` failure in `body`.
/// type: (body: () -> A ! N) -> Result(A, AssertionFailure) ! N - Assert
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
