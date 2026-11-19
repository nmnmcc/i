Alias(A) = (A) <- (Assert) -> (Int) -> Int

Wide(A, B) = (List(A), Dict(A, B), Int) -> Bool

Handler = { (Int) -> Int } | (String) -> String

@external("env", "log")
log : (String) -> ()

apply = (
	f : (Int) <- (Fail(E), N) -> Int,
	x : { Int | Float - () & Any }
) -> Int { f(x) }
