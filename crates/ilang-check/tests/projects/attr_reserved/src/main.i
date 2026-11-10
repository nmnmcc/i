@external("ilang:core", "not")
not: (Bool) -> Bool

@host("ilang:storage")
S {
	S(get: () -> Int)
}

@external("app:math/v1", "add")
add: (Int, Int) -> Int
