// spec: 9.3, 9.4, 9.5
// expect: pass
// output: cancelled
(
	i/control
	i/fail
	i/resource
)

Done = ()
Paused = (Continuation(Nil, Step, Fail(String)))
Step = Done | Paused

@host("test:io/v1")
Out = (print = (String) -> Nil)

print = (text: String) <- (out: Out) { out.print(text) }

main = () {
	kept = control.keep_env((b) {
		control.suspend(b, (k) { Paused(k) })
		fail.raise("resumed")
		Done
	})
	first = fail.catch(() { control.boundary((b) { kept(b) }) })
	first |>
		(Ok(Paused(k))) {
			assert(
				fail.catch(() { control.resume(k, Nil) }) == Error("resumed"),
			)
		}
		| (_) { panic("expected a paused kept call") }
	with_final = control.keep_env((b) {
		resource.ensure(
			() {
				control.suspend(b, (k) { Paused(k) })
				Done
			},
			// A final may not raise errors (§9.5); observe that it ran instead.
			() { print("cancelled") },
		)
	})
	second = fail.catch(() { control.boundary((b) { with_final(b) }) })
	second |>
		(Ok(Paused(k))) {
			assert(
				fail.catch(() {
					control.cancel(k)
					fail.raise("cancelled")
				}) ==
					Error("cancelled"),
			)
		}
		| (_) { panic("expected a paused kept final") }
	// A handler provided inside the paused part still wins over the resumer.
	inner_catch = control.keep_env((b) {
		answer = fail.catch(() {
			control.suspend(b, (k) { Paused(k) })
			fail.raise("inside")
		})
		answer |>
			(Error("inside")) { Done }
			| (_) { panic("lost the inner handler") }
	})
	third = fail.catch(() { control.boundary((b) { inner_catch(b) }) })
	third |>
		(Ok(Paused(k))) {
			assert(fail.catch(() { control.resume(k, Nil) }) == Ok(Done))
		}
		| (_) { panic("expected a paused inner catch") }
}
