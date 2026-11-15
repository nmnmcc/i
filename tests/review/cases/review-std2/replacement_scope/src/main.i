(
	i/fail
)
S(E) = (() <- (Fail(E)) -> Nil)
use = <E>() <- (s : S(E)) {
	S(f) = s
	f()
}
first : () <- (Fail(String), S(String)) -> Nil = use
second : () <- (Fail(Int), S(Int)) -> Nil = use

example = () {
	{
		{
			Error("s") = fail.catch(first)
			Nil
		} <- (S(() { fail.raise("s") }))
	}
	{
		{
			Error(2) = fail.catch(second)
			Nil
		} <- (S(() { fail.raise(2) }))
	}
}
