(
	i/fail
)

one = () -> () { fail.raise(1) }

two = () -> () { fail.raise(2) }

pick = (b : Bool) { b |> (True) { one } & (False) { two } }

main = () {
	fail.catch(() { pick(True)() }) |>
		(Error(1)) { () }
		& (_) { assert(False) }
	fail.catch(() { pick(False)() }) |>
		(Error(2)) { () }
		& (_) { assert(False) }
}
