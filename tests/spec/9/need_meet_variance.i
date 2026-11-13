// spec: 4.4, 4.5, 9.2
// expect: pass
(
	i/failure
)

// The other-direction parameter gives the same union on both paths.
need_int = () <- (f: Fail(Int)) { Nil }

need_string = () <- (f: Fail(String)) { Nil }

both_fail = () -> Nil {
	need_int()
	need_string()
}

fail_value = (x: Fail(Int) & Fail(String)) -> Fail(Int | String) { x }

// The same-direction parameter gives the same intersection on both paths.
Read(A) = (value = A)

need_read_int = () <- (r: Read(Int)) { Nil }

need_read_one = () <- (r: Read(1)) { Nil }

both_read = () -> Nil {
	need_read_int()
	need_read_one()
}

read_value = (x: Read(Int) & Read(1)) -> Read(1) { x }

main = () {
	{
		both_read()
		failure.catch(() {
			both_fail()
			Nil
		})
		Nil
	} <- (read_value(Read(value = 1)))
}
