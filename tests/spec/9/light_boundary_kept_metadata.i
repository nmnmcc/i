// spec: 9.2, 9.4, 11.9
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
)

Reading = (read = () -> Int)

the_reading = () <- (r: Reading) { r }

read = () <- (r: Reading) { r.read() }

test("keep_env keeps a portable lazy service as its value", () {
	kept = control.boundary((b) {
		control.keep_env((x: Nil) { read() }) <- (Reading(read = () { 1 }))
	})
	core.portable(kept) |>
		(True) { Nil }
		| (False) { panic("portable service became non-portable") }
	{ kept(Nil) == 1 } |>
		(True) { Nil }
		| (False) { panic("wrong kept service") }
})

test("keep_env keeps services without light boundary metadata", () {
	r = control.boundary((b) {
		{
			original = the_reading()
			{ original == the_reading() } |>
				(True) { Nil }
				| (False) { panic("service identity changed") }
			// The old service is still cached in the boundary metadata. It is
			// no longer bound, so keep_env must keep only its replacement.
			{
				kept = control.keep_env((x: Nil) { read() })
				core.portable(kept) |>
					(True) { Nil }
					| (False) { panic("kept an unbound service") }
				kept(Nil)
			} <- (Reading(read = () { 1 }))
		} <- (Reading(read = () { control.escape(b, 0) }))
	})
	{ r == 1 } |> (True) { Nil } | (False) { panic("wrong kept service") }
})
