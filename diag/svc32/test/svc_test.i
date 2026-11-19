//// Diagnostic: the cost of one environment statement (`$env_with`) as the
//// number of service types K of the whole program grows. Variant K = 32 (package diag/svc32):
//// S0 is the hot service; S1..S31 only widen the environment (every type
//// that appears in an environment statement or parameter gets a slot).
//// The K = 4 variant is service-env-bench-k4.i (S1..S3 only). Each
//// variant must be its own package: `i test` builds one program per
//// package, so the slot count is shared by every test file in it.
(
	testlib = (test) = std/test
	i/core
)

rounds = 100_000

check = (ok : Bool, what : String) -> () {
	ok |> (True) { () } & (False) { panic(what) }
}

S0 = (step = Int)

read0 = () <- (s : S0) -> Int { s.step }

// One environment statement per iteration (a self tail call, as in
// tests/spec/9/environment.i `count`). The two values alternate, so the
// slot changes every time and nothing can be hoisted.
spin = (n : Int, acc : Int, odd : S0, even : S0) -> Int {
	n |>
		(0) { acc }
		& (_) {
			s = { n % 2 == 1 } |> (True) { odd } & (False) { even }
			spin(n - 1, acc + read0(), odd, even) <- (s)
		}
}

// rounds / 2 odd steps of 3 and rounds / 2 even steps of 1.
test("service_env_spin", () {
	got = spin(rounds, 0, S0(step = 3), S0(step = 1))
	check(got == 200_000, "spin")
})

// --- widening: S1..S31 (delete for a smaller K) ---
S1 = (step = Int)
S2 = (step = Int)
S3 = (step = Int)
S4 = (step = Int)
S5 = (step = Int)
S6 = (step = Int)
S7 = (step = Int)
S8 = (step = Int)
S9 = (step = Int)
S10 = (step = Int)
S11 = (step = Int)
S12 = (step = Int)
S13 = (step = Int)
S14 = (step = Int)
S15 = (step = Int)
S16 = (step = Int)
S17 = (step = Int)
S18 = (step = Int)
S19 = (step = Int)
S20 = (step = Int)
S21 = (step = Int)
S22 = (step = Int)
S23 = (step = Int)
S24 = (step = Int)
S25 = (step = Int)
S26 = (step = Int)
S27 = (step = Int)
S28 = (step = Int)
S29 = (step = Int)
S30 = (step = Int)
S31 = (step = Int)

sum_wide = () <- (
	a1 : S1, a2 : S2, a3 : S3, a4 : S4, a5 : S5, a6 : S6, a7 : S7, a8 : S8,
	a9 : S9, a10 : S10, a11 : S11, a12 : S12, a13 : S13, a14 : S14, a15 : S15,
	a16 : S16, a17 : S17, a18 : S18, a19 : S19, a20 : S20, a21 : S21,
	a22 : S22, a23 : S23, a24 : S24, a25 : S25, a26 : S26, a27 : S27,
	a28 : S28, a29 : S29, a30 : S30, a31 : S31,
) -> Int {
	// One line: no reliance on operator continuation across lines.
	a1.step + a2.step + a3.step + a4.step + a5.step + a6.step + a7.step + a8.step + a9.step + a10.step + a11.step + a12.step + a13.step + a14.step + a15.step + a16.step + a17.step + a18.step + a19.step + a20.step + a21.step + a22.step + a23.step + a24.step + a25.step + a26.step + a27.step + a28.step + a29.step + a30.step + a31.step
}

// Run once (not hot): makes S1..S31 service types of the program.
test("service_env_wide", () {
	got = sum_wide() <- (
		S1(step = 1), S2(step = 2), S3(step = 3), S4(step = 4), S5(step = 5),
		S6(step = 6), S7(step = 7), S8(step = 8), S9(step = 9),
		S10(step = 10), S11(step = 11), S12(step = 12), S13(step = 13),
		S14(step = 14), S15(step = 15), S16(step = 16), S17(step = 17),
		S18(step = 18), S19(step = 19), S20(step = 20), S21(step = 21),
		S22(step = 22), S23(step = 23), S24(step = 24), S25(step = 25),
		S26(step = 26), S27(step = 27), S28(step = 28), S29(step = 29),
		S30(step = 30), S31(step = 31),
	)
	check(got == 496, "wide")
})
