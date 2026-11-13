// A1 §3.10: declared variables are checked before resolving annotations.
pair = <A, A>(x: A) { x }
clash = <Int>(x: Int) { x }
args = <A>(x: A(Int)) { x }
unknown = (x: Missing) { x }
mixed = <A>(x: A, y: b) { x }
Box(Int) = { (Int) }
Free = (Undeclared)
