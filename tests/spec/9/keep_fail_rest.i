// spec: 9.3, 9.4, 9.5
// expect: pass
(
	i/control
	i/resource
)

// Ordinary Fail and boundary code, independent of the concurrent i/fail port.
raise = <E>(error: E) <- (handler: Fail(E)) -> Never { handler.raise(error) }
catch: <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E) = (body) {
	control.boundary((b) {
		Ok(body()) <- (
			Fail(raise = (error) { control.escape(b, Error(error)) }),
		)
	})
}

// Exercise the shared library implementation of syntax-3 §2.10.8.
keep = control.keep

Done = ()
Paused = (Continuation(Nil, Step, Fail(String)))
Step = Done | Paused

main = () {
	kept = keep((b) {
		control.suspend(b, (k) { Paused(k) })
		raise("resumed")
		Done
	})
	first = catch(() { control.boundary((b) { kept(b) }) })
	first |>
		(Ok(Paused(k))) {
			assert(catch(() { control.resume(k, Nil) }) == Error("resumed"))
		}
		| (_) { panic("expected a paused kept call") }
	// §9.5: a final function may not raise errors. Observe cancellation by
	// an error-free escape, then check the later caller's Fail separately.
	cancelled = control.boundary((finished) {
		with_final = keep((b) {
			resource.ensure(
				() {
					control.suspend(b, (k) { Paused(k) })
					Done
				},
				() { control.escape(finished, "cancelled") },
			)
		})
		second = catch(() { control.boundary((b) { with_final(b) }) })
		second |>
			(Ok(Paused(k))) {
				control.cancel(k)
				panic("the cancelled final did not run")
			}
			| (_) { panic("expected a paused kept final") }
	})
	assert(cancelled == "cancelled")
	assert(catch(() { raise(cancelled) }) == Error("cancelled"))
	// A handler provided inside the paused part still wins over the resumer.
	inner_catch = keep((b) {
		answer = catch(() {
			control.suspend(b, (k) { Paused(k) })
			raise("inside")
		})
		answer |>
			(Error("inside")) { Done }
			| (_) { panic("lost the inner handler") }
	})
	third = catch(() { control.boundary((b) { inner_catch(b) }) })
	third |>
		(Ok(Paused(k))) {
			assert(catch(() { control.resume(k, Nil) }) == Ok(Done))
		}
		| (_) { panic("expected a paused inner catch") }
}
