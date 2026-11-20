// spec: 9.2, 9.4
// expect: pass
// multicore.md D4: every call keeps all services; each call makes a fresh function.
// multicore.md 4: retained bases survive repeated resume and cancellation.
(
	i/control
	i/core
	i/resource
)

// Exercise the shared library implementation of syntax-3 §2.10.8.
keep = control.keep

Name = (text = String)
// Decision 2: keep's returned function retains the real Fail(Never) key.
Paused = (Continuation((), Hop, Fail(Never)))
Done = (String)
Hop = Paused | Done
name = () <- (value: Name) { value.text }

walk = (b, n) {
	{ n == 3 } |>
		(True()) { Done(name()) }
		& (False()) {
			control.suspend(b, (k) { Paused(k) })
			walk(b, n + 1)
		}
}

identity = (value) { value }

make = () { keep((b) { walk(b, 0) }) }

main = () {
	{
		kept = make()
		pure = (n: Int) { n + 1 }
		a = keep(pure)
		b = keep(pure)
		assert(a != b)
		assert(a(37) == 38)
		inner = keep((boundary) {
			identity(walk(boundary, 0)) <- (Name(text = "inner"))
		})
		{
			Paused(k1) = control.boundary((boundary) { kept(boundary) })
			Paused(k2) = control.resume(k1, ())
			Paused(k3) = control.resume(k2, ())
			assert(control.resume(k3, ()) == Done("kept"))
			Paused(ki1) = control.boundary((boundary) { inner(boundary) })
			Paused(ki2) = control.resume(ki1, ())
			Paused(ki3) = control.resume(ki2, ())
			assert(control.resume(ki3, ()) == Done("inner"))
			{
				with_final = keep((boundary) {
					resource.ensure(
						() {
							control.suspend(boundary, (k) { Paused(k) })
							Done("unreachable")
						},
						() { assert(name() == "final") },
					)
				})
				{
					Paused(k4) = control.boundary((boundary) {
						with_final(boundary)
					})
					control.cancel(k4)
					control.boundary((boundary) {
						extra: Extra((), Assert) = Extra(boundary)
						{
							f = keep(pure)
							assert(
								core.not(core.portable(f)),
							)
						} <- (Name(text = "unused"), extra)
					})
				} <- (Name(text = "canceller"))
			} <- (Name(text = "final"))
		} <- (Name(text = "caller"))
	} <- (Name(text = "kept"))
}

Extra(A, N) = (Boundary(A, N))
