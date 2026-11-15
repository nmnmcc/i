// spec: 6.9
// expect: pass
// Syntax three O15: `|>` is the weakest operator, and its whole left
// expression is its left operand.
(
	main
)

not = (b : Bool) { b |> (True) { False } | (False) { True } }
inc = (n : Int) { n + 1 }

main = () {
	assert(1 == 2 |> not)
	assert(1 + 1 == 2 |> not |> not)
	assert({ 2 < 3 && 3 < 4 |> not } == False)
	assert({ 1 == 1 || 1 == 2 |> (True) { 7 } | (False) { 8 } } == 7)
	assert({ 1 |> inc |> inc } == 3)
	assert({ 1 + 2 * 3 |> inc } == 8)
	Nil
}
