(
	i/fail
)
S(E) = (() <- (Fail(E)) -> ())
use = <E>() <- (s : S(E)) {
	S(f) = s
	f()
}
first : () <- (Fail(String), S(String)) -> () = use
second : () <- (Fail(Int), S(Int)) -> () = use

example = () {
	{
		{
			Error("s") = fail.catch(first)
			()
		} <- (S(() { fail.raise("s") }))
	}
	{
		{
			Error(2) = fail.catch(second)
			()
		} <- (S(() { fail.raise(2) }))
	}
}
