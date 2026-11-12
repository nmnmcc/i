// spec: 9.4, 9.2, 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
	i/resource
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

Name {
	Name(text: String)
}

name = (n ! Name) { n.text }

Bad {
	Bad(Int)
}

Paused = PausedCase | Done

PausedCase {
	Paused(Continuation(Nil, Paused, Never, Name))
}

Done {
	Done(String)
}

// The final function gives the name it sees to the value of `outer`.
final = (outer) {
	seen = name()
	control.suspend(outer, (k) { seen <> ", " <> control.resume(k, Nil) })
}

// The call of `resource.ensure` is under `! Name = Name("in")`, and is not
// a tail call, so the frame of `f` waits outside the ensure.
f = (b, outer) {
	! Name = Name("in")
	x = resource.ensure(() {
		control.suspend(b, (k) {
			! Name = Name("out")
			control.cancel(k)
			"cancelled"
		})
	}, () { final(outer) })
	x <> ""
}

// §9.4 rules 3 and 4: the final function of a cancel sees the environment
// of its frame, not the one of the place of `cancel`.

test("cancel_keeps_inner_environment", () {
	! Name = Name("outside")
	r = control.boundary((outer) { control.boundary((b) { f(b, outer) }) })
	check(r, "in, cancelled")
})

failing = (b, outer) {
	! Name = Name("in")
	x = resource.ensure(() {
		control.suspend(b, (k) { Paused(k) })
		fail.raise(Bad(1))
	}, () { final(outer) })
	x <> ""
}

// The same for an error after a resume from a place where `Name` is "out".

test("error_keeps_inner_environment", () {
	! Name = Name("outside")
	r = control.boundary((outer) {
		p = control.boundary((b) {
			Done(show(fail.catch(() { failing(b, outer) })))
		})
		p |> {
			Paused(k) -> {
				! Name = Name("out")
				{ control.resume(k, Nil) } |> {
					Done(s)   -> s
					Paused(_) -> "paused again"
				}
			}
			Done(s) -> s
		}
	})
	check(r, "in, Error(Bad(1))")
})

// Two more frames with environment statements, inside the ensure: they are
// passed over, and the final function sees the environment of `nested`.
nested = (b, outer) {
	! Name = Name("in")
	x = resource.ensure(() { mid1(b) }, () { final(outer) })
	x <> ""
}

mid1 = (b) {
	! Name = Name("mid1")
	x = mid2(b)
	x <> ""
}

mid2 = (b) {
	! Name = Name("mid2")
	x = control.suspend(b, (k) {
		! Name = Name("out")
		control.cancel(k)
		"cancelled"
	})
	x <> ""
}

test("frames_inside_the_ensure", () {
	! Name = Name("outside")
	r = control.boundary((outer) { control.boundary((b) { nested(b, outer) }) })
	check(r, "in, cancelled")
})

// The two frames outside the ensure: the final function sees the
// environment of the innermost of them.
around = (b, outer) {
	! Name = Name("in")
	x = around1(b, outer)
	x <> ""
}

around1 = (b, outer) {
	! Name = Name("around1")
	x = around2(b, outer)
	x <> ""
}

around2 = (b, outer) {
	! Name = Name("around2")
	x = resource.ensure(() {
		control.suspend(b, (k) {
			! Name = Name("out")
			control.cancel(k)
			"cancelled"
		})
	}, () { final(outer) })
	x <> ""
}

test("frames_outside_the_ensure", () {
	! Name = Name("outside")
	r = control.boundary((outer) { control.boundary((b) { around(b, outer) }) })
	check(r, "around2, cancelled")
})
