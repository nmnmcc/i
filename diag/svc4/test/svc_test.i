//// Diagnostic: the cost of one environment statement (`$env_with`) as the
//// number of service types of the whole program grows. K = 4 here.
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

// One environment statement per iteration (a self tail call). The two values
// alternate, so the slot changes every time and nothing can be hoisted.
spin = (n : Int, acc : Int, odd : S0, even : S0) -> Int {
	n |>
		(0) { acc }
		& (_) {
			s = { n % 2 == 1 } |> (True) { odd } & (False) { even }
			spin(n - 1, acc + read0(), odd, even) <- (s)
		}
}

test("service_env_spin", () {
	got = spin(rounds, 0, S0(step = 3), S0(step = 1))
	check(got == 200_000, "spin")
})

// Widen the environment to four service types.
S1 = (step = Int)
S2 = (step = Int)
S3 = (step = Int)

sum_wide = () <- (a1 : S1, a2 : S2, a3 : S3) -> Int {
	a1.step + a2.step + a3.step
}

test("service_env_wide", () {
	got = sum_wide() <- (S1(step = 1), S2(step = 2), S3(step = 3))
	check(got == 6, "wide")
})
