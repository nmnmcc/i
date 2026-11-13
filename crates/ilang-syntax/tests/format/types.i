Alias(A) = (A) <- (Assert, Nil) -> (Int) -> Int

Wide(A, B) = (List(A), Dict(A, B), Int) -> Bool

Handler = { (Int) -> Int } | (String) -> String

@external("env", "log")
log: (String) -> Nil

apply = (f: (Int) <- (Fail(E), N) -> Int, x: { Int | Float - Nil & Any }) -> Int {
	f(x)
}
