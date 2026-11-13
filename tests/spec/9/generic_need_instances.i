// spec: 4.7, 9.2
// expect: pass

// The argument of a need entry inside a generic function's data type
// belongs to each instance of its scheme: `at` used with `String` and then
// with `Int` gives each its own type.
Yd(E) { Yd(emit: (value: E) -> Nil) }
Br(E, N) { Br(decode: (Int) -> Int ! Yd(E) | N) }

at(E, N) = (bs: List(Br(E, N)), i: Int) -> Br(E, N) {
	bs |> {
		[] -> panic("empty")
		[b, ..rest] -> { i == 0 } |> {
			True  -> b
			False -> at(rest, i - 1)
		}
	}
}

strings = (bs: List(Br(String, Never))) -> Br(String, Never) { at(bs, 0) }
ints = (bs: List(Br(Int, Never))) -> Br(Int, Never) { at(bs, 0) }

main = () {
	strings([Br((_) { 1 })])
	ints([Br((_) { 2 })])
	Nil
}
