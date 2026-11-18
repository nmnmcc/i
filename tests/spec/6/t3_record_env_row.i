// spec: 4.9, 9.2
// expect: compile-error E0515
(
	main
)
S(A) = (A)
Bad = { (f = () <- (S(Int), S(String)) -> ()) }
use = (x : Bad) { x }

main = () { () }
