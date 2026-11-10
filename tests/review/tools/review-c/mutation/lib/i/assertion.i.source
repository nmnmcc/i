//// Taking `Assert` failures (§9.6).

import (
	i/control
)

export (
	catch
)

/// Gives `Error(failure)` for the first `Assert` failure in `body`.
/// type: (body: () -> a ? e ! n) -> Result(a, AssertionFailure) ? e ! n - Assert
catch = (body) {
	control.boundary((b) {
		! Assert = Assert((failure) {
			control.suspend(b, (k) {
				control.cancel(k)
				Error(failure)
			})
		})
		Ok(body())
	})
}
