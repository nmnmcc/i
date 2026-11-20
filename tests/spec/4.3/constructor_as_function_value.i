// spec: 4.3, 6.3, 6.6, 6.7
// expect: pass

// A constructor is a function: `True : () -> True`, `Ok : (A) -> Ok(A)`.
// Its empty value is a call, `True()`, in values and in patterns. `True`
// itself has the type `() -> Bool`, so a function with parameters takes
// `(_) { True() }`.

pick = (f : (Int) -> Bool, xs : List(Int)) -> List(Int) {
	xs |>
		([]) { [] }
		& ([x, ..rest]) {
			f(x) |>
				(True()) { [x, ..pick(f, rest)] }
				& (False()) { pick(f, rest) }
		}
}

wrap = (make : (Int) -> Ok(Int), x : Int) -> Ok(Int) { make(x) }

main = () {
	yes : Bool = True()
	assert(yes)
	assert(pick((_) { True() }, [1, 2]) == [1, 2])
	assert(pick((_) { False() }, [1, 2]) == [])
	assert(wrap(Ok, 1) == Ok(1))
	()
}
