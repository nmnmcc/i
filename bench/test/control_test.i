//// Resuming a paused computation at depth 10, 100 and 1000. `scripts/bench`
//// runs these with `i test --time` at `--opt 2`. A resume and the next
//// suspend should take the same time at every depth (docs/performance.md).
(
	testlib = (test) = std/test
	i/control
	i/core
)

rounds = 100_000

Done = (Int)
More = (Int, Continuation(Nil, Step, Never, Never))
// What the paused computation gives: each number, and at the end the
// number of frames it went back through.
Step = Done | More

check = (ok : Bool, what : String) -> Nil {
	ok |> (True) { Nil } | (False) { panic(what) }
}

// Suspends once for each number from `n` down to 1. The call after the
// suspend is a tail call, so this is one frame.
yields = (b, n) {
	n |>
		(0) { Done(0) }
		| (_) {
			control.suspend(b, (k) { More(n, k) })
			yields(b, n - 1)
		}
}

// Counts one more frame on the way back.
deeper = (Done(d)) { Done(d + 1) } | (more) { more }

// `depth` frames that wait under the suspends.
down = (b, depth) {
	depth |> (0) { yields(b, rounds) } | (_) { deeper(down(b, depth - 1)) }
}

// Resumes until the end. It runs outside the boundary and calls itself as
// a tail call, so the place of resume is always at the same depth.
sum = (step, acc, depth) {
	step |>
		(Done(d)) {
			check(d == depth, "a frame was lost or ran twice")
			acc
		}
		| (More(v, k)) { sum(control.resume(k, Nil), acc + v, depth) }
}

run = (depth) {
	got = sum(control.boundary((b) { down(b, depth) }), 0, depth)
	check(got == 5_000_050_000, "the sum of 1 to rounds is wrong")
}

test("resume_depth_10", () { run(10) })

test("resume_depth_100", () { run(100) })

test("resume_depth_1000", () { run(1000) })
