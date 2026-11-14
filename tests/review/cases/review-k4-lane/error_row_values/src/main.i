(
	i/fail
)

one = () -> Nil { fail.raise(1) }

two = () -> Nil { fail.raise(2) }

pick = (b : Bool) { b |> (True) { one } | (False) { two } }

main = () {
	fail.catch(() { pick(True)() }) |>
		(Error(1)) { Nil }
		| (_) { assert(False) }
	fail.catch(() { pick(False)() }) |>
		(Error(2)) { Nil }
		| (_) { assert(False) }
}
