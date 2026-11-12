//// Taking `Assert` failures (§9.6).

(
	i/control
)

(
	catch
)

/// Gives `Error(failure)` for the first `Assert` failure in `body`.
/// type: (body: () -> A ^ E ! N) -> Result(A, AssertionFailure) ^ E ! N - Assert
catch = (body) {
	control.boundary((b) {
		! Assert = Assert(fail: (failure) {
			control.suspend(b, (k) {
				control.cancel(k)
				Error(failure)
			})
		})
		Ok(body())
	})
}
