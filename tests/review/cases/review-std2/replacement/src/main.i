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

main = () {
	{
		Error("s") = fail.catch(first)
		{
			Error(2) = fail.catch(second)
			()
		} <- (S(() { fail.raise(2) }))
	} <- (S(() { fail.raise("s") }))
}
