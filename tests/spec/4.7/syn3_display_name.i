// spec: 4.7
// expect: pass

// Syntax three D7: a shown union that is exactly the expansion of a declared
// alias is shown by its name: `first : (xs: List(Option(Int))) -> Int`.
Some(A) = (A)
None = ()
Option(A) = Some(A) | None

first = (xs: List(Option(Int))) -> Int {
	xs |> ([Some(x), ..r]) { x } | (_) { 0 }
}

main = () { assert(first([Some(3), None]) == 3) }
