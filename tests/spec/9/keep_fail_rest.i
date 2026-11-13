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

// syntax-3 §2.10.8: keep N and use Fail from the later caller.
keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (
	Fail(E),
) -> A {
	(x) {
		// D16: keep a frame until B2 preserves tail-provided environments.
		value = f(x) <- env
		value
	}
}

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
	with_final = keep((b) {
		resource.ensure(
			() {
				control.suspend(b, (k) { Paused(k) })
				Done
			},
			() { raise("cancelled") },
		)
	})
	second = catch(() { control.boundary((b) { with_final(b) }) })
	second |>
		(Ok(Paused(k))) {
			assert(catch(() { control.cancel(k) }) == Error("cancelled"))
		}
		| (_) { panic("expected a paused kept final") }
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
