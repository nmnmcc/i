(
	i/fail
)
S(E) = (() <- (Fail(E)) -> Nil)
use = <E>() <- (s: S(E)) {
	S(f) = s
	f()
}
first: () <- (Fail(String), S(String)) -> Nil = use
second: () <- (Fail(Int), S(Int)) -> Nil = use

main = () {
	{
		Error("s") = fail.catch(first)
		{
			Error(2) = fail.catch(second)
			Nil
		} <- (S(() { fail.raise(2) }))
	} <- (S(() { fail.raise("s") }))
}
