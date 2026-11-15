// spec: 6.1
// expect: pass
(
	main
)
Ok(A) = (A)
Error(E) = (E)
Result(A, E) = Ok(A) | Error(E)
identity = (n) { n }
take_tuple = ((0, _) = t) { t } | (_) { (0, 0) }
take_result = (Ok(v) = r) { v } | (_) { 0 }
take_prefix = ("ab" <> rest = s) { s } | (_) { "" }

main = () {
	assert(
		{
			id = <A>(x : A) -> A { x }
			id(3)
		} ==
			3,
	)
	assert(
		{
			go = (n) -> Int { n + 1 }
			go(3)
		} ==
			4,
	)
	assert(
		{
			k : (Int) -> Int = identity
			k(5)
		} ==
			5,
	)
	assert(
		{
			x : Int = 6
			x
		} ==
			6,
	)
	assert(
		{
			k = 2
			g = (x) -> Int { x * k }
			g(3)
		} ==
			6,
	)
	assert({ 7 } == 7)
	assert(take_tuple((0, 9)) == (0, 9))
	assert(take_result(Ok(9)) == 9)
	assert(take_prefix("abcd") == "abcd")
	Nil
}
